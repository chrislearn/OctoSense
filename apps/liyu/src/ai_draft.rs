//! 送礼草稿/确认状态机与受限写工具 `prepare_gift_draft`（OctoLoop #5 Peer B）。
//!
//! 与 ai.rs 的只读工具不同，这里只有一个 `Risk::Act` 工具，而且它的「写」被
//! 严格限制在**本机打开一份可审阅的送礼草稿**：不发送、不扣款、不改变服务端
//! 礼物状态。草稿是否送出，只能由本机用户在礼遇界面逐条确认驱动状态机，
//! 本模块提供状态与转换逻辑，UI 接线由 master 完成。
//!
//! 状态机（`DraftStatus`）：
//!
//! | 从                | 到                | 触发                                          |
//! |-------------------|-------------------|-----------------------------------------------|
//! | `Draft`           | `AwaitingConfirm` | 草稿填入完成、在本机界面打开待审阅            |
//! | `Draft`           | `Cancelled`       | 用户取消 / 窗口关闭 / 账号切换                |
//! | `Draft`           | `Expired`         | 超过 TTL 未进入确认                           |
//! | `AwaitingConfirm` | `Confirmed`       | 本机用户在礼遇界面点确认（账号一致、未超时）  |
//! | `AwaitingConfirm` | `Cancelled`       | 用户取消 / 窗口关闭 / 账号切换                |
//! | `AwaitingConfirm` | `Expired`         | 超过 TTL 未确认                               |
//! | 终态三选一        | —                 | `Confirmed` / `Cancelled` / `Expired` 不可再变 |
//!
//! 安全边界：
//! - **幂等**：同一 `call_id` 的重复调用/重试返回同一份草稿，绝不重复创建；
//!   所有写都走同一把锁（串行），并发重试也只建一份。
//! - **隔离**：草稿绑定 `account_id`；账号不一致的确认/取消一律拒绝，
//!   `switch_account` 会把旧账号所有未终态草稿置为 `Cancelled`。
//! - **超时**：超过 [`DEFAULT_TTL_MS`] 未确认的草稿置为 `Expired`，
//!   旧确认不可复用。
//! - **不暴露提交工具**：在线送礼下单 API 未接入（无法满足完整送礼参数），
//!   因此本模块不注册任何 `send_gift` / `submit_gift` / `place_order` 工具；
//!   调用这些名字一律拒绝，并在结果里如实说明后续 API 依赖。
//! - **ToolResult 真实**：`prepare_gift_draft` 成功只表示「草稿已在本机打开
//!   待本人确认」，结果文案与 JSON 里不会出现「已送出 / 已下单 / 已扣款」。

use crate::data::{item, yuan, CATALOG};
use makepad_app_module::makepad_ai_services::wire::{Risk, ServiceCall, ServiceManifest, ToolDef, ToolResult};
use std::sync::Mutex;

/// 本轮注册的工具列表。只有一个草稿工具——没有任何提交/下单工具。
pub const TOOL_NAMES: [&str; 1] = ["prepare_gift_draft"];

const PREPARE_ARGS: &str = r#"{"type":"object","properties":{"item_index":{"type":"integer","description":"目录下标（list_gift_catalog / get_gift_detail 返回的顺序，从 0 开始）"},"note":{"type":"string","description":"可选：附在草稿里的寄语，确认前可在界面修改"}},"required":["item_index"]}"#;

/// 草稿确认时限：5 分钟。超时未确认即 `Expired`，旧确认不可复用。
pub const DEFAULT_TTL_MS: u64 = 5 * 60 * 1000;

pub fn manifest() -> ServiceManifest {
    ServiceManifest::new(
        "liyu_draft",
        "礼遇送礼草稿闸",
        "只在本机打开并填入可审阅的送礼草稿：不发送、不扣款、不改变服务端礼物状态。草稿经本机用户在礼遇界面确认后才算数；拒绝、取消、超时、账号切换、窗口关闭都会让草稿进入终态且不可复用。没有也不接受任何送礼/下单提交工具——后续提交依赖在线送礼下单 API（未接入）。",
    )
    .with_tool(ToolDef::new(
        TOOL_NAMES[0],
        "打开一份送礼草稿待本人确认：只填礼物与寄语，返回草稿 ID 与摘要，状态 awaiting_user_confirm。不发送、不扣款。",
        PREPARE_ARGS,
        Risk::Act,
    ))
}

/// 草稿状态机状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftStatus {
    /// 已建草稿，尚未进入确认（内部瞬态；对外工具返回时已是 AwaitingConfirm）。
    Draft,
    /// 已在本机界面打开，等待本人确认。
    AwaitingConfirm,
    /// 本人在界面确认。终态。
    Confirmed,
    /// 拒绝 / 取消 / 窗口关闭 / 账号切换。终态。
    Cancelled,
    /// 超过 TTL 未确认。终态。
    Expired,
}

impl DraftStatus {
    pub fn key(self) -> &'static str {
        match self {
            DraftStatus::Draft => "draft",
            DraftStatus::AwaitingConfirm => "awaiting_user_confirm",
            DraftStatus::Confirmed => "confirmed",
            DraftStatus::Cancelled => "cancelled",
            DraftStatus::Expired => "expired",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, DraftStatus::Confirmed | DraftStatus::Cancelled | DraftStatus::Expired)
    }
}

/// 一份送礼草稿。只含目录下标与寄语，没有任何「已下单」字段。
#[derive(Clone, Debug)]
pub struct GiftDraft {
    pub draft_id: String,
    /// 幂等键：创建它的那次 `ServiceCall::call_id`。
    pub call_id: String,
    /// 创建草稿的账号；确认/取消必须同账号。
    pub account_id: String,
    pub item_index: u16,
    pub note: String,
    pub status: DraftStatus,
    pub created_ms: u64,
    pub expires_ms: u64,
    /// 终态原因（取消/超时/账号切换/窗口关闭），供界面如实展示。
    pub terminal_reason: Option<String>,
}

impl GiftDraft {
    /// 一句话摘要：只描述「草稿」，绝不声称已送出。
    pub fn summary(&self) -> String {
        let it = item(self.item_index);
        let mut s = format!("送礼草稿 {}：{}（{}）", self.draft_id, it.name, yuan(it.price));
        if !self.note.is_empty() {
            s.push_str(&format!("，寄语「{}」", self.note));
        }
        s.push_str(match self.status {
            DraftStatus::AwaitingConfirm => "——已在本机打开，等本人确认；尚未发送、未扣款",
            DraftStatus::Confirmed => "——本人已在界面确认；提交依赖在线送礼下单 API（未接入），尚未发送",
            DraftStatus::Cancelled => "——已取消，未发送",
            DraftStatus::Expired => "——已超时作废，未发送",
            DraftStatus::Draft => "——填写中，未发送",
        });
        s
    }
}

/// 确认/取消等本地转换的结果。这不是 ToolResult——它面向礼遇 UI。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateOutcome {
    /// 转换成功：草稿进入终态 `Confirmed`。
    Confirmed { draft_id: String, summary: String },
    /// 转换被拒绝：草稿保持或进入相应终态，reason 如实说明。
    Refused { draft_id: String, status: DraftStatus, reason: String },
}

impl GateOutcome {
    pub fn is_confirmed(&self) -> bool {
        matches!(self, GateOutcome::Confirmed { .. })
    }
}

#[derive(Default)]
struct DraftState {
    drafts: Vec<GiftDraft>,
    next_seq: u64,
}

/// 草稿闸：所有写都经同一把锁串行；`call_id` 幂等去重。
pub struct DraftGate {
    inner: Mutex<DraftState>,
}

impl Default for DraftGate {
    fn default() -> Self {
        Self::new()
    }
}

impl DraftGate {
    pub fn new() -> Self {
        DraftGate { inner: Mutex::new(DraftState::default()) }
    }

    /// AI 工具分发。只认 `prepare_gift_draft`；任何提交/下单名字一律拒绝。
    pub fn answer(&self, account_id: &str, call: &ServiceCall, now_ms: u64) -> ToolResult {
        match call.tool.as_str() {
            "prepare_gift_draft" => self.prepare(account_id, call, now_ms),
            other => ToolResult::refused(
                &call.call_id,
                format!(
                    "没有 `{other}` 工具，也不会提供：送礼的提交依赖在线送礼下单 API（未接入），这里只能打开本机草稿（prepare_gift_draft），送不送由本人在界面确认"
                ),
            ),
        }
    }

    /// `prepare_gift_draft`：只建/重放草稿。重复 `call_id` 返回同一份草稿。
    fn prepare(&self, account_id: &str, call: &ServiceCall, now_ms: u64) -> ToolResult {
        let Some(index) = arg_num(&call.args, "item_index") else {
            return ToolResult::refused(&call.call_id, "需要 `item_index`（目录下标，整数）");
        };
        if index < 0.0 || index.fract() != 0.0 || index as u64 >= CATALOG.len() as u64 {
            return ToolResult::refused(
                &call.call_id,
                format!("`item_index` 要在 0..{} 之间（目录只有 {} 件）", CATALOG.len() - 1, CATALOG.len()),
            );
        }
        let note = arg_str(&call.args, "note").unwrap_or_default();
        let mut st = self.inner.lock().unwrap();

        // 幂等:同一 call_id + 同一账号的重试/双击重放已有草稿,不重复创建。
        // 跨账号同 call_id:不返回旧草稿(含寄语),拒绝且不泄漏任何既有草稿内容。
        if let Some(d) = st.drafts.iter().find(|d| d.call_id == call.call_id) {
            if d.account_id != account_id {
                return ToolResult::refused(
                    &call.call_id,
                    "这个 call_id 属于另一个账号的草稿,已拒绝跨账号重放",
                );
            }
            let d = d.clone();
            let mut text = draft_json(&d, now_ms);
            text = text.trim_end_matches('}').to_string() + ",\"idempotent_replay\":true}";
            return ToolResult::ok(
                &call.call_id,
                text,
                "同一 call_id 的重放：返回既有草稿，没有新建、没有扣款",
            );
        }

        st.next_seq += 1;
        let draft = GiftDraft {
            draft_id: format!("liyu-draft-{:06}", st.next_seq),
            call_id: call.call_id.clone(),
            account_id: account_id.to_string(),
            item_index: index as u16,
            note,
            status: DraftStatus::AwaitingConfirm,
            created_ms: now_ms,
            expires_ms: now_ms + DEFAULT_TTL_MS,
            terminal_reason: None,
        };
        let text = draft_json(&draft, now_ms);
        st.drafts.push(draft);
        ToolResult::ok(
            &call.call_id,
            text,
            "草稿已在本机打开待本人确认：未发送、未扣款、服务端礼物状态未变",
        )
    }

    /// 本机用户在礼遇界面点确认。账号不一致、已终态、已超时都拒绝；
    /// 超时先把草稿置为 `Expired` 再拒绝——旧确认不可复用。
    pub fn confirm(&self, account_id: &str, draft_id: &str, now_ms: u64) -> GateOutcome {
        self.transition(account_id, draft_id, now_ms, Transition::Confirm)
    }

    /// 本机用户在礼遇界面取消。
    pub fn cancel(&self, account_id: &str, draft_id: &str, now_ms: u64) -> GateOutcome {
        self.transition(account_id, draft_id, now_ms, Transition::Cancel("本人已取消".into()))
    }

    /// 礼遇窗口关闭：未确认的草稿作废，旧确认不可复用。
    pub fn window_closed(&self, account_id: &str, draft_id: &str, now_ms: u64) -> GateOutcome {
        self.transition(account_id, draft_id, now_ms, Transition::Cancel("礼遇窗口已关闭，本次确认作废".into()))
    }

    /// 账号切换：旧账号所有未终态草稿置为 `Cancelled`，返回作废品数。
    /// 新账号拿不到旧账号的草稿，也复用不了旧确认。
    pub fn switch_account(&self, old_account: &str, _new_account: &str, _now_ms: u64) -> u32 {
        let mut st = self.inner.lock().unwrap();
        let mut n = 0;
        for d in st.drafts.iter_mut() {
            if d.account_id == old_account && !d.status.is_terminal() {
                d.status = DraftStatus::Cancelled;
                d.terminal_reason = Some("账号已切换，草稿作废".into());
                n += 1;
            }
        }
        n
    }

    pub fn status(&self, draft_id: &str) -> Option<DraftStatus> {
        self.inner.lock().unwrap().drafts.iter().find(|d| d.draft_id == draft_id).map(|d| d.status)
    }

    pub fn draft(&self, draft_id: &str) -> Option<GiftDraft> {
        self.inner.lock().unwrap().drafts.iter().find(|d| d.draft_id == draft_id).cloned()
    }

    /// 草稿总数（测试/诊断用）。
    pub fn draft_count(&self) -> usize {
        self.inner.lock().unwrap().drafts.len()
    }

    fn transition(&self, account_id: &str, draft_id: &str, now_ms: u64, t: Transition) -> GateOutcome {
        let mut st = self.inner.lock().unwrap();
        let Some(d) = st.drafts.iter_mut().find(|d| d.draft_id == draft_id) else {
            return GateOutcome::Refused {
                draft_id: draft_id.into(),
                status: DraftStatus::Cancelled,
                reason: format!("没有草稿 {draft_id}（可能已随窗口关闭或账号切换作废）"),
            };
        };
        if d.account_id != account_id {
            return GateOutcome::Refused {
                draft_id: draft_id.into(),
                status: d.status,
                reason: "草稿属于另一个账号：账号切换后不能提交或复用旧确认".into(),
            };
        }
        // 超时先行：AwaitingConfirm 超时的先落终态再拒绝。
        if d.status == DraftStatus::AwaitingConfirm && now_ms >= d.expires_ms {
            d.status = DraftStatus::Expired;
            d.terminal_reason = Some("超过确认时限未确认，草稿已超时".into());
            return GateOutcome::Refused {
                draft_id: draft_id.into(),
                status: DraftStatus::Expired,
                reason: "草稿已超时：旧确认不可复用，要送请重新开一份草稿".into(),
            };
        }
        match (t, d.status) {
            (Transition::Confirm, DraftStatus::AwaitingConfirm) => {
                d.status = DraftStatus::Confirmed;
                GateOutcome::Confirmed { draft_id: draft_id.into(), summary: d.summary() }
            }
            (Transition::Cancel(reason), s @ (DraftStatus::Draft | DraftStatus::AwaitingConfirm)) => {
                d.status = DraftStatus::Cancelled;
                d.terminal_reason = Some(reason.clone());
                let _ = s;
                GateOutcome::Refused { draft_id: draft_id.into(), status: DraftStatus::Cancelled, reason }
            }
            (Transition::Confirm, s) => GateOutcome::Refused {
                draft_id: draft_id.into(),
                status: s,
                reason: format!("草稿已是终态 {}，不能再确认", s.key()),
            },
            (Transition::Cancel(_), s) => GateOutcome::Refused {
                draft_id: draft_id.into(),
                status: s,
                reason: format!("草稿已是终态 {}，无需再取消", s.key()),
            },
        }
    }
}

enum Transition {
    Confirm,
    Cancel(String),
}

/// 草稿结果 DTO（JSON）。`status` 只会是状态机里的键；文案如实说明 API 依赖。
fn draft_json(d: &GiftDraft, now_ms: u64) -> String {
    let it = item(d.item_index);
    let remain = d.expires_ms.saturating_sub(now_ms);
    format!(
        "{{\"draft_id\":{},\"status\":{},\"item\":{{\"index\":{},\"name\":{},\"form\":{},\"price\":{}}},\"note\":{},\"summary\":{},\"expires_in_ms\":{},\"truth\":{}}}",
        json_str(&d.draft_id),
        json_str(d.status.key()),
        d.item_index,
        json_str(it.name),
        json_str(if it.physical { "实物" } else { "电子券" }),
        yuan_num(it.price),
        json_str(&d.note),
        json_str(&d.summary()),
        remain,
        json_str("这只是本机草稿：尚未发送、未扣款、服务端礼物状态未变。最终提交依赖在线送礼下单 API（当前未接入），因此没有也不接受任何提交/下单工具。"),
    )
}

// ---- 与 ai.rs 同款的极简参数/JSON 助手（那边是私有的，这里自带一份）----

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 分 → JSON 数字（元），与 ai.rs 的 `yuan_num` 一致。
fn yuan_num(cents: i64) -> String {
    if cents % 100 == 0 {
        format!("{}", cents / 100)
    } else {
        format!("{}.{:02}", cents / 100, (cents % 100).abs())
    }
}

fn arg_raw<'a>(args: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\"");
    let at = args.find(&pat)? + pat.len();
    let rest = args[at..].trim_start();
    Some(rest.strip_prefix(':')?.trim_start())
}

fn arg_str(args: &str, key: &str) -> Option<String> {
    let rest = arg_raw(args, key)?;
    let body = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)?);
                }
                c => out.push(c),
            },
            c => out.push(c),
        }
    }
    None
}

fn arg_num(args: &str, key: &str) -> Option<f64> {
    if let Some(s) = arg_str(args, key) {
        return s.trim().parse().ok();
    }
    let rest = arg_raw(args, key)?;
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use makepad_app_module::makepad_ai_services::wire::ToolOutcome;

    const T0: u64 = 1_000_000;

    fn call(call_id: &str, tool: &str, args: &str) -> ServiceCall {
        ServiceCall { call_id: call_id.into(), tool: tool.into(), args: args.into() }
    }

    fn prepare_ok(gate: &DraftGate, account: &str, call_id: &str) -> (String, String) {
        let r = gate.answer(account, &call(call_id, "prepare_gift_draft", r#"{"item_index":1,"note":"生日快乐"}"#), T0);
        assert_eq!(r.outcome, ToolOutcome::Ok, "{}", r.text);
        let d = (0..gate.draft_count())
            .filter_map(|i| gate.draft(&format!("liyu-draft-{:06}", i + 1)))
            .find(|d| d.call_id == call_id)
            .expect("草稿应已创建");
        (d.draft_id.clone(), r.text)
    }

    #[test]
    fn manifest_has_only_one_act_tool_and_no_submit() {
        let m = manifest();
        m.validate().unwrap();
        assert_eq!(m.tools.len(), 1);
        let t = m.tool("prepare_gift_draft").expect("草稿工具应在清单里");
        assert_eq!(t.risk, Risk::Act);
        for name in ["send_gift", "submit_gift", "place_order", "confirm_gift", "pay"] {
            assert!(m.tool(name).is_none(), "不得注册提交工具 {name}");
        }
    }

    #[test]
    fn prepare_opens_draft_awaiting_confirm_and_is_truthful() {
        let gate = DraftGate::new();
        let (id, text) = prepare_ok(&gate, "alice", "c1");
        assert_eq!(gate.status(&id), Some(DraftStatus::AwaitingConfirm));
        assert!(text.contains("\"status\":\"awaiting_user_confirm\""), "{}", text);
        assert!(text.contains("星巴克中杯拿铁电子券") && text.contains("\"price\":35"), "{}", text);
        assert!(text.contains("未发送") && text.contains("未扣款"), "{}", text);
        // 真实性：结果绝不能声称已送出/已下单/已扣款。
        for banned in ["已送出", "已下单", "已扣款", "sent", "order_placed"] {
            assert!(!text.contains(banned), "结果不得包含 `{banned}`: {text}");
        }
    }

    #[test]
    fn duplicate_call_id_replays_same_draft() {
        let gate = DraftGate::new();
        let (id, _) = prepare_ok(&gate, "alice", "c1");
        // 双击/重试/重放：同一 call_id 再调两次，参数不同也不得新建。
        for args in [r#"{"item_index":1,"note":"生日快乐"}"#, r#"{"item_index":9}"#] {
            let r = gate.answer("alice", &call("c1", "prepare_gift_draft", args), T0 + 1);
            assert_eq!(r.outcome, ToolOutcome::Ok);
            assert!(r.text.contains(&format!("\"draft_id\":\"{id}\"")), "{}", r.text);
            assert!(r.text.contains("\"idempotent_replay\":true"), "{}", r.text);
        }
        assert_eq!(gate.draft_count(), 1, "重复 call_id 不得重复创建");
    }

    #[test]
    fn cross_account_same_call_id_is_refused_without_leak() {
        let gate = DraftGate::new();
        // alice 用 call_id=c-shared 开了草稿(含寄语 note)。
        let (id, alice_text) = prepare_ok(&gate, "alice", "c-shared");
        assert!(alice_text.contains("生日快乐"), "alice 草稿含寄语: {}", alice_text);
        // bob 用同一 call_id 重放:必须拒绝,且不得泄漏 alice 的草稿内容/draft_id/寄语。
        let r = gate.answer("bob", &call("c-shared", "prepare_gift_draft", r#"{"item_index":1}"#), T0 + 1);
        assert_eq!(r.outcome, ToolOutcome::Refused, "跨账号同 call_id 应拒绝: {}", r.text);
        assert!(!r.text.contains(&id), "不得泄漏 alice 的 draft_id: {}", r.text);
        assert!(!r.text.contains("生日快乐"), "不得泄漏 alice 的寄语: {}", r.text);
        // bob 不能用拒绝的 call_id 重放,但可以用自己的 call_id 正常开草稿。
        let (bob_id, _) = prepare_ok(&gate, "bob", "c-bob-own");
        assert_ne!(bob_id, id, "bob 应得到独立草稿");
        assert_eq!(gate.draft_count(), 2, "拒绝重放不新建,但 bob 自有 call_id 可建");
        // 同账号 alice 重放仍正常返回既有草稿。
        let r2 = gate.answer("alice", &call("c-shared", "prepare_gift_draft", r#"{"item_index":1}"#), T0 + 2);
        assert_eq!(r2.outcome, ToolOutcome::Ok);
        assert!(r2.text.contains("\"idempotent_replay\":true"), "{}", r2.text);
    }

    #[test]
    fn concurrent_prepare_with_same_call_id_creates_one() {
        use std::sync::Arc;
        let gate = Arc::new(DraftGate::new());
        let mut handles = vec![];
        for _ in 0..8 {
            let g = gate.clone();
            handles.push(std::thread::spawn(move || {
                g.answer("alice", &call("c-race", "prepare_gift_draft", r#"{"item_index":2}"#), T0)
            }));
        }
        let texts: Vec<String> = handles.into_iter().map(|h| h.join().unwrap().text).collect();
        assert_eq!(gate.draft_count(), 1, "并发同 call_id 只建一份");
        let id = &gate.draft("liyu-draft-000001").unwrap().draft_id;
        assert!(texts.iter().all(|t| t.contains(id)), "{texts:?}");
    }

    #[test]
    fn confirm_happy_path_then_terminal() {
        let gate = DraftGate::new();
        let (id, _) = prepare_ok(&gate, "alice", "c1");
        let out = gate.confirm("alice", &id, T0 + 1000);
        assert!(out.is_confirmed(), "{out:?}");
        assert_eq!(gate.status(&id), Some(DraftStatus::Confirmed));
        // 终态不可再变：再确认、再取消都拒绝，状态不变。
        let out = gate.confirm("alice", &id, T0 + 2000);
        assert!(!out.is_confirmed() && matches!(out, GateOutcome::Refused { status: DraftStatus::Confirmed, .. }), "{out:?}");
        let out = gate.cancel("alice", &id, T0 + 3000);
        assert!(matches!(out, GateOutcome::Refused { status: DraftStatus::Confirmed, .. }), "{out:?}");
        assert_eq!(gate.status(&id), Some(DraftStatus::Confirmed));
    }

    #[test]
    fn cancel_then_confirm_is_refused() {
        let gate = DraftGate::new();
        let (id, _) = prepare_ok(&gate, "alice", "c1");
        let out = gate.cancel("alice", &id, T0 + 1000);
        assert!(matches!(out, GateOutcome::Refused { status: DraftStatus::Cancelled, .. }), "{out:?}");
        let out = gate.confirm("alice", &id, T0 + 1001);
        assert!(matches!(out, GateOutcome::Refused { status: DraftStatus::Cancelled, .. }), "{out:?}");
        assert_eq!(gate.status(&id), Some(DraftStatus::Cancelled));
    }

    #[test]
    fn expired_draft_cannot_be_confirmed() {
        let gate = DraftGate::new();
        let (id, _) = prepare_ok(&gate, "alice", "c1");
        // TTL 边界内可以确认。
        let (id2, _) = prepare_ok(&gate, "alice", "c2");
        assert!(gate.confirm("alice", &id2, T0 + DEFAULT_TTL_MS - 1).is_confirmed());
        // 过 TTL：确认被拒绝，草稿落 Expired，旧确认不可复用。
        let out = gate.confirm("alice", &id, T0 + DEFAULT_TTL_MS);
        assert!(matches!(out, GateOutcome::Refused { status: DraftStatus::Expired, .. }), "{out:?}");
        assert_eq!(gate.status(&id), Some(DraftStatus::Expired));
        let out = gate.confirm("alice", &id, T0 + DEFAULT_TTL_MS + 1);
        assert!(matches!(out, GateOutcome::Refused { status: DraftStatus::Expired, .. }), "{out:?}");
    }

    #[test]
    fn account_switch_isolates_drafts() {
        let gate = DraftGate::new();
        let (id, _) = prepare_ok(&gate, "alice", "c1");
        // bob 不能确认 alice 的草稿，草稿也不因误试而变。
        let out = gate.confirm("bob", &id, T0 + 1000);
        assert!(!out.is_confirmed(), "{out:?}");
        assert_eq!(gate.status(&id), Some(DraftStatus::AwaitingConfirm));
        // 切换账号：旧账号未终态草稿全部作废。
        assert_eq!(gate.switch_account("alice", "bob", T0 + 2000), 1);
        assert_eq!(gate.status(&id), Some(DraftStatus::Cancelled));
        // 原账号也复用不了旧确认。
        let out = gate.confirm("alice", &id, T0 + 3000);
        assert!(matches!(out, GateOutcome::Refused { status: DraftStatus::Cancelled, .. }), "{out:?}");
    }

    #[test]
    fn window_close_cancels_and_blocks_reuse() {
        let gate = DraftGate::new();
        let (id, _) = prepare_ok(&gate, "alice", "c1");
        let out = gate.window_closed("alice", &id, T0 + 500);
        assert!(matches!(out, GateOutcome::Refused { status: DraftStatus::Cancelled, .. }), "{out:?}");
        let out = gate.confirm("alice", &id, T0 + 600);
        assert!(matches!(out, GateOutcome::Refused { status: DraftStatus::Cancelled, .. }), "{out:?}");
        // 不存在的草稿（窗口关掉后 UI 拿着旧 ID 来）也是真实拒绝。
        let out = gate.confirm("alice", "liyu-draft-999999", T0 + 700);
        assert!(!out.is_confirmed(), "{out:?}");
    }

    #[test]
    fn bad_args_and_submit_tools_are_refused() {
        let gate = DraftGate::new();
        // 缺 item_index / 越界 / 非整数：拒绝且不建草稿。
        for args in ["{}", r#"{"item_index":33}"#, r#"{"item_index":-1}"#, r#"{"item_index":1.5}"#, r#"{"item_index":"abc"}"#] {
            let r = gate.answer("alice", &call("cx", "prepare_gift_draft", args), T0);
            assert_eq!(r.outcome, ToolOutcome::Refused, "{args}: {}", r.text);
        }
        assert_eq!(gate.draft_count(), 0);
        // 任何提交/下单名字：一律拒绝并如实说明 API 依赖。
        for tool in ["send_gift", "submit_gift", "place_order", "confirm_gift"] {
            let r = gate.answer("alice", &call("cy", tool, "{}"), T0);
            assert_eq!(r.outcome, ToolOutcome::Refused, "{tool}");
            assert!(r.text.contains("未接入"), "{}", r.text);
        }
        assert_eq!(gate.draft_count(), 0, "拒绝路径不得建草稿");
    }

    #[test]
    fn result_never_claims_gift_sent_even_after_confirm() {
        let gate = DraftGate::new();
        let (id, _) = prepare_ok(&gate, "alice", "c1");
        assert!(gate.confirm("alice", &id, T0 + 100).is_confirmed());
        // 确认后的重放结果仍然如实：确认 ≠ 已送出。
        let r = gate.answer("alice", &call("c1", "prepare_gift_draft", r#"{"item_index":1}"#), T0 + 200);
        assert!(r.text.contains("\"status\":\"confirmed\""), "{}", r.text);
        assert!(r.text.contains("尚未发送"), "{}", r.text);
        for banned in ["已送出", "已下单", "已扣款"] {
            assert!(!r.text.contains(banned), "{}", r.text);
        }
        let d = gate.draft(&id).unwrap();
        assert!(d.summary().contains("未接入"), "{}", d.summary());
    }
}
