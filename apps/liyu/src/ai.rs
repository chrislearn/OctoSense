//! 礼遇在桌面 AI bus 上的服务清单与只读工具（04-rules 6 节）。
//!
//! 三个工具全部 `Risk::Read`：
//! - `list_gift_catalog`：礼物目录（名称、品类、形态、价格），可按品类过滤；
//! - `suggest_gift`：预算内 1–3 件建议 + 一句理由，可带场合；
//! - `get_gift_box_summary`：收到 / 送出各状态计数、余额、待兑现契约数。
//!
//! 边界由类型保证：目录工具只读常量表；礼盒摘要先压成 [`BoxSummary`]（只有计数和
//! 金额），应答时根本拿不到答案、暗号、送礼人、地址、口令。没有任何写操作——
//! 送礼、解谜、收下、折现都只能本人在界面上点。

use crate::data::{item, yuan, Category, LiyuState, PactState, CATALOG};
use makepad_app_module::makepad_ai_services::wire::{Risk, ServiceCall, ServiceManifest, ToolDef, ToolResult};

/// 本轮注册的工具列表。
pub const TOOL_NAMES: [&str; 3] = ["list_gift_catalog", "suggest_gift", "get_gift_box_summary"];

const CATALOG_ARGS: &str = r#"{"type":"object","properties":{"category":{"type":"string","description":"可选：咖啡茶饮 / 电影演出 / 潮流小物 / 盲盒 / 甜点鲜花（也认 coffee / movie / trendy / blind / sweet）"}}}"#;
const SUGGEST_ARGS: &str = r#"{"type":"object","properties":{"budget":{"type":"number","description":"预算，单位元"},"occasion":{"type":"string","description":"可选：场合，如 生日 / 感谢 / 道歉 / 约会 / 加油"}},"required":["budget"]}"#;
const NO_ARGS: &str = r#"{"type":"object","properties":{}}"#;

pub fn manifest() -> ServiceManifest {
    ServiceManifest::new(
        "liyu",
        "礼遇 LiYu",
        "礼遇演示应用：匿名悬念送礼。挑一份礼物、设一道谜题、附一份小契约，对方解开才知道是谁送的。只提供礼物目录、预算内挑礼建议和礼盒计数摘要；不会输出谜题答案、暗号、未揭晓的送礼人、地址或口令，也不能替人送礼、解谜、收下或折现。",
    )
    .with_tool(ToolDef::new(
        TOOL_NAMES[0],
        "礼物目录：名称、品类、形态（实物 / 电子券）、价格（元）。可按品类过滤。",
        CATALOG_ARGS,
        Risk::Read,
    ))
    .with_tool(ToolDef::new(
        TOOL_NAMES[1],
        "按预算（元）和场合挑 1–3 件目录里的礼物，附一句理由。",
        SUGGEST_ARGS,
        Risk::Read,
    ))
    .with_tool(ToolDef::new(
        TOOL_NAMES[2],
        "礼盒摘要：收到 / 送出的礼物按状态计数、余额、待兑现契约数。不含送礼人、答案或口令。",
        NO_ARGS,
        Risk::Read,
    ))
}

/// 礼盒摘要：只有计数和金额，没有任何文本字段。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoxSummary {
    /// 按 `GiftState::id()` 下标计数。
    pub received: [u32; 8],
    pub sent: [u32; 8],
    /// 分。
    pub balance: i64,
    /// 我欠别人的 / 别人欠我的（都还在待兑现）。
    pub pacts_mine: u32,
    pub pacts_theirs: u32,
}

impl BoxSummary {
    pub fn from_state(s: &LiyuState) -> Self {
        let mut out = BoxSummary { balance: s.balance(), ..Default::default() };
        for g in s.received() {
            out.received[g.state().id() as usize] += 1;
        }
        for g in s.sent() {
            out.sent[g.state().id() as usize] += 1;
        }
        for p in &s.pacts {
            if p.state() == PactState::Pending {
                if p.mine {
                    out.pacts_mine += 1;
                } else {
                    out.pacts_theirs += 1;
                }
            }
        }
        out
    }
}

/// JSON 里的状态键，与 `GiftState::id()` 一一对应（见测试）。
const STATE_KEYS: [&str; 8] = [
    "sealed",
    "opened",
    "revealed",
    "accepted",
    "exchanged",
    "cashed_out",
    "expired",
    "withdrawn",
];

pub fn answer(summary: &BoxSummary, call: &ServiceCall) -> ToolResult {
    match call.tool.as_str() {
        "list_gift_catalog" => {
            let raw = arg_str(&call.args, "category");
            let cat = raw.as_deref().and_then(parse_category);
            if raw.as_deref().is_some_and(|r| !r.trim().is_empty()) && cat.is_none() {
                return ToolResult::refused(
                    &call.call_id,
                    "不认识这个品类；可选：咖啡茶饮 / 电影演出 / 潮流小物 / 盲盒 / 甜点鲜花",
                );
            }
            ToolResult::ok(&call.call_id, catalog_json(cat), "礼物目录（常量表），价格单位元")
        }
        "suggest_gift" => {
            let Some(budget) = arg_num(&call.args, "budget").filter(|b| b.is_finite() && *b > 0.0) else {
                return ToolResult::refused(&call.call_id, "需要一个大于 0 的预算（元）");
            };
            let occasion = arg_str(&call.args, "occasion").unwrap_or_default();
            ToolResult::ok(
                &call.call_id,
                suggest_json((budget * 100.0).round() as i64, &occasion),
                "只从目录里挑，建议仅供参考，送不送由本人决定",
            )
        }
        "get_gift_box_summary" => ToolResult::ok(
            &call.call_id,
            summary_json(summary),
            "只有计数与金额：不含送礼人、答案、暗号、地址或口令",
        ),
        other => ToolResult::refused(
            &call.call_id,
            format!("liyu 没有 `{other}` 工具；只能查目录、挑礼建议和礼盒摘要，不能代为操作"),
        ),
    }
}

/// 中文名、英文键、首字都认。
pub fn parse_category(s: &str) -> Option<Category> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    Category::ALL.into_iter().find(|c| {
        let en = match c {
            Category::Coffee => "coffee",
            Category::Movie => "movie",
            Category::Trendy => "trendy",
            Category::Blind => "blind",
            Category::Sweet => "sweet",
        };
        s == en || c.label() == s || c.label().starts_with(&s) || (s.chars().count() >= 2 && c.label().contains(&s))
    })
}

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

/// 分 → JSON 数字（元），`109` / `8.72`。
fn yuan_num(cents: i64) -> String {
    if cents % 100 == 0 {
        format!("{}", cents / 100)
    } else {
        format!("{}.{:02}", cents / 100, (cents % 100).abs())
    }
}

fn item_json(i: u16) -> String {
    let it = item(i);
    format!(
        "{{\"name\":{},\"category\":{},\"form\":{},\"spec\":{},\"price\":{}}}",
        json_str(it.name),
        json_str(it.cat.label()),
        json_str(if it.physical { "实物" } else { "电子券" }),
        json_str(it.spec),
        yuan_num(it.price),
    )
}

fn catalog_json(cat: Option<Category>) -> String {
    let items: Vec<String> = (0..CATALOG.len() as u16)
        .filter(|&i| cat.map_or(true, |c| item(i).cat == c))
        .map(item_json)
        .collect();
    format!("{{\"items\":[{}],\"currency\":\"CNY\"}}", items.join(","))
}

/// 场合 → 优先品类 + 一句话。认不出的场合按价格挑。
fn occasion_hint(occasion: &str) -> (&'static [Category], &'static str) {
    const RULES: [(&[&str], &[Category], &str); 6] = [
        (&["生日", "birthday"], &[Category::Sweet, Category::Blind], "生日配点甜的，或者拆盲盒的惊喜"),
        (&["感谢", "谢谢", "thanks"], &[Category::Coffee, Category::Sweet], "一杯咖啡的谢意刚刚好，不让对方有负担"),
        (&["道歉", "对不起", "sorry"], &[Category::Sweet, Category::Coffee], "先递一份甜的，话更好说"),
        (&["约会", "电影", "date"], &[Category::Movie, Category::Sweet], "票在手里，下一次见面就有了理由"),
        (&["加油", "考试", "上班", "打气"], &[Category::Coffee, Category::Blind], "提神的咖啡，或者一点小期待"),
        (&["纪念", "毕业", "搬家", "乔迁"], &[Category::Trendy, Category::Sweet], "能留下来的小物件，看到就会想起你"),
    ];
    let o = occasion.to_lowercase();
    RULES
        .iter()
        .find(|(keys, _, _)| keys.iter().any(|k| o.contains(k)))
        .map(|(_, cats, why)| (*cats, *why))
        .unwrap_or((&[], "预算内挑了几样不同品类的，挑一件配一道谜题就好"))
}

/// 预算内 1–3 件：先按场合优先品类，每个品类取预算内最贵的一件，再用其他品类补足。
pub fn suggest(budget_cents: i64, occasion: &str) -> Vec<u16> {
    let (prefer, _) = occasion_hint(occasion);
    let best_in = |c: Category| {
        (0..CATALOG.len() as u16)
            .filter(|&i| item(i).cat == c && item(i).price <= budget_cents)
            .max_by(|&a, &b| item(a).price.cmp(&item(b).price).then(b.cmp(&a)))
    };
    let mut picks: Vec<u16> = prefer.iter().filter_map(|&c| best_in(c)).collect();
    let mut rest: Vec<u16> = Category::ALL
        .into_iter()
        .filter(|c| !prefer.contains(c))
        .filter_map(best_in)
        .collect();
    rest.sort_by(|&a, &b| item(b).price.cmp(&item(a).price).then(a.cmp(&b)));
    picks.extend(rest);
    picks.truncate(3);
    picks
}

fn suggest_json(budget_cents: i64, occasion: &str) -> String {
    let picks = suggest(budget_cents, occasion);
    if picks.is_empty() {
        let cheapest = CATALOG.iter().map(|c| c.price).min().unwrap_or(0);
        return format!(
            "{{\"items\":[],\"reason\":{}}}",
            json_str(&format!("预算 {} 内没有合适的礼物，目录里最便宜的是 {}", yuan(budget_cents), yuan(cheapest)))
        );
    }
    let (_, why) = occasion_hint(occasion);
    let items: Vec<String> = picks.iter().map(|&i| item_json(i)).collect();
    format!(
        "{{\"items\":[{}],\"budget\":{},\"reason\":{}}}",
        items.join(","),
        yuan_num(budget_cents),
        json_str(why)
    )
}

fn summary_json(s: &BoxSummary) -> String {
    let counts = |c: &[u32; 8]| {
        let fields: Vec<String> = STATE_KEYS
            .iter()
            .zip(c.iter())
            .map(|(k, n)| format!("\"{k}\":{n}"))
            .collect();
        format!("{{{}}}", fields.join(","))
    };
    format!(
        "{{\"received\":{},\"sent\":{},\"balance\":{},\"open_pacts\":{{\"i_owe\":{},\"owed_to_me\":{}}}}}",
        counts(&s.received),
        counts(&s.sent),
        yuan_num(s.balance),
        s.pacts_mine,
        s.pacts_theirs,
    )
}

// ---- 参数：字段固定、只有一层，手取比引入 JSON 库省事 ----

/// 找到 `"key"` 后面冒号之后的原文。
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

/// 数字，也认 `"100"` 这样被引起来的数字。
fn arg_num(args: &str, key: &str) -> Option<f64> {
    if let Some(s) = arg_str(args, key) {
        return s.trim().trim_start_matches('¥').parse().ok();
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
    use crate::data::*;
    use makepad_app_module::makepad_ai_services::wire::ToolOutcome;

    fn call(tool: &str, args: &str) -> ServiceCall {
        ServiceCall { call_id: "t1".into(), tool: tool.into(), args: args.into() }
    }

    #[test]
    fn manifest_validates_with_three_read_tools() {
        let m = manifest();
        m.validate().unwrap();
        assert_eq!(m.tools.len(), 3);
        for name in TOOL_NAMES {
            let t = m.tool(name).expect("工具应在清单里");
            assert_eq!(t.risk, Risk::Read);
        }
    }

    #[test]
    fn unknown_or_write_tools_are_refused() {
        let s = BoxSummary::default();
        for tool in ["send_gift", "submit_answer", "cash_out", "who_sent_this"] {
            assert_eq!(answer(&s, &call(tool, "{}")).outcome, ToolOutcome::Refused, "{tool}");
        }
    }

    #[test]
    fn catalog_lists_all_or_one_category() {
        let s = BoxSummary::default();
        let r = answer(&s, &call("list_gift_catalog", "{}"));
        assert_eq!(r.outcome, ToolOutcome::Ok);
        for it in CATALOG.iter() {
            assert!(r.text.contains(it.name), "{}", r.text);
        }
        assert!(r.text.contains("\"price\":109"), "{}", r.text);
        let r = answer(&s, &call("list_gift_catalog", r#"{"category":"电影演出"}"#));
        assert!(r.text.contains("电影通兑票") && r.text.contains("电影双人套票"), "{}", r.text);
        assert!(!r.text.contains("香薰蜡烛"), "{}", r.text);
        let r = answer(&s, &call("list_gift_catalog", r#"{"category": "sweet"}"#));
        assert!(r.text.contains("向日葵花束") && !r.text.contains("电影"), "{}", r.text);
        let r = answer(&s, &call("list_gift_catalog", r#"{"category":"火箭"}"#));
        assert_eq!(r.outcome, ToolOutcome::Refused);
    }

    #[test]
    fn suggestions_stay_within_budget() {
        for budget in [30, 50, 100, 200] {
            let picks = suggest(budget * 100, "");
            assert!(!picks.is_empty() && picks.len() <= 3, "{budget}: {picks:?}");
            assert!(picks.iter().all(|&i| item(i).price <= budget * 100), "{budget}: {picks:?}");
            let cats: Vec<_> = picks.iter().map(|&i| item(i).cat).collect();
            let mut uniq = cats.clone();
            uniq.dedup();
            assert_eq!(cats.len(), uniq.len(), "同一品类只推一件");
        }
        // 生日优先甜点。
        let picks = suggest(100_00, "朋友生日");
        assert_eq!(item(picks[0]).cat, Category::Sweet);
        // 预算太低：空列表 + 说明。
        let r = answer(&BoxSummary::default(), &call("suggest_gift", r#"{"budget":10}"#));
        assert_eq!(r.outcome, ToolOutcome::Ok);
        assert!(r.text.contains("\"items\":[]"), "{}", r.text);
        // 预算可以是字符串数字。
        let r = answer(&BoxSummary::default(), &call("suggest_gift", r#"{"budget":"60","occasion":"感谢"}"#));
        assert!(r.text.contains("星巴克中杯拿铁电子券"), "{}", r.text);
        assert!(!r.text.contains("三顿半"), "超预算: {}", r.text);
        // 没预算：拒绝。
        let r = answer(&BoxSummary::default(), &call("suggest_gift", "{}"));
        assert_eq!(r.outcome, ToolOutcome::Refused);
    }

    #[test]
    fn summary_counts_states_and_pacts() {
        let s = LiyuState::for_tests();
        let sum = BoxSummary::from_state(&s);
        assert_eq!(sum.received.iter().sum::<u32>() as usize, s.received().len());
        assert_eq!(sum.sent.iter().sum::<u32>() as usize, s.sent().len());
        assert_eq!((sum.pacts_mine + sum.pacts_theirs) as usize, s.open_pacts());
        assert_eq!(STATE_KEYS[GiftState::CashedOut.id() as usize], "cashed_out");
        let r = answer(&sum, &call("get_gift_box_summary", "{}"));
        assert_eq!(r.outcome, ToolOutcome::Ok);
        assert!(r.text.contains("\"received\":{\"sealed\":"), "{}", r.text);
        assert!(r.text.contains(&format!("\"balance\":{}", yuan_num(s.balance()))), "{}", r.text);
    }

    #[test]
    fn outputs_never_leak_secrets() {
        let mut s = LiyuState::for_tests();
        // 再送一份带暗号、契约、地址的，确保这些都在状态里。
        let d = SendDraft {
            item: 0,
            peer: "许宁".into(),
            unlock: Unlock::Passphrase,
            clue: "老地方".into(),
            answer: "月亮不睡我不睡".into(),
            contract: Some("周末陪我看一场电影".into()),
            message: "天冷了多穿点".into(),
            use_balance: false,
        };
        s.send_gift(&d, TEST_TODAY).unwrap();
        s.settings.ship_phone = "13800138000".into();
        s.settings.ship_addr = "望京 SOHO T3".into();
        let sum = BoxSummary::from_state(&s);

        let mut forbidden: Vec<String> = Vec::new();
        for g in &s.gifts {
            forbidden.push(g.code.clone());
            if !g.answer.is_empty() {
                forbidden.push(g.answer.clone());
            }
            if !g.message.is_empty() {
                forbidden.push(g.message.clone());
            }
            forbidden.extend(split_aliases(&g.peer));
            for f in [&g.ship_name, &g.ship_phone, &g.ship_addr, &g.voucher] {
                if !f.is_empty() {
                    forbidden.push(f.clone());
                }
            }
        }
        forbidden.push(s.settings.ship_phone.clone());
        forbidden.push(s.settings.ship_addr.clone());
        forbidden.extend(s.contacts.iter().map(|c| c.label.clone()));
        forbidden.extend(s.directory.iter().cloned());

        let calls = [
            call("list_gift_catalog", "{}"),
            call("suggest_gift", r#"{"budget":200,"occasion":"生日"}"#),
            call("suggest_gift", r#"{"budget":1000}"#),
            call("get_gift_box_summary", "{}"),
        ];
        for c in &calls {
            let r = answer(&sum, c);
            assert_eq!(r.outcome, ToolOutcome::Ok, "{}", c.tool);
            for f in &forbidden {
                assert!(!r.text.contains(f.as_str()), "{} 输出泄露 {f}", c.tool);
                assert!(!r.note.contains(f.as_str()), "{} 备注泄露 {f}", c.tool);
            }
            assert!(!r.text.contains("LY-"), "{} 输出带了口令", c.tool);
        }
    }

    #[test]
    fn arg_parsing_tolerates_spacing_and_escapes() {
        assert_eq!(arg_num(r#"{ "budget" : 88.5 }"#, "budget"), Some(88.5));
        assert_eq!(arg_num(r#"{"budget":"¥66"}"#, "budget"), Some(66.0));
        assert_eq!(arg_str(r#"{"occasion":"生日"}"#, "occasion").as_deref(), Some("生日"));
        assert_eq!(arg_str(r#"{"occasion":null}"#, "occasion"), None);
        assert_eq!(arg_num("{}", "budget"), None);
    }
}
