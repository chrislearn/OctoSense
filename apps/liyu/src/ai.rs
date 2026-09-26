//! 礼遇在桌面 AI bus 上的服务清单与只读工具（04-rules 6 节；#5 只读投影）。
//!
//! 五个工具全部 `Risk::Read`：
//! - `list_gift_catalog`：礼物目录（名称、品类、形态、价格），可按品类过滤；
//! - `suggest_gift`：预算内 1–3 件建议 + 一句理由，可带场合；
//! - `get_gift_box_summary`：收到 / 送出各状态计数、余额、待兑现契约数；
//! - `get_gift_detail`：目录里一件商品的完整详情（名称、品牌、描述、规格、价格、
//!   可订状态）；
//! - `list_gift_candidates`：预算 / 品类过滤 + 按名称、卖点、描述搜关键词的候选清单。
//!
//! 隐私边界由类型保证：应答入口 [`answer`] 只接 [`BoxSummary`]（计数 + 金额）与目录
//! 投影 [`PubGiftDetail`]（只含公开商品字段），整个文件不持有 `LiyuState`、`Gift`、
//! 心愿单或资料，编译期就拿不到未揭晓送礼人、谜底 / 答案、地址、手机号、券码和
//! 别人的私有心愿。没有任何写操作——送礼、解谜、收下、折现都只能本人在界面上点。
//!
//! 目录来源：在线态（登录且 token 有效）逐件向服务端核对当前名称、描述、价格与
//! 可订状态，结果标 `"source":"server"`；离线、服务端不可达、或某件商品的服务端
//! 目录项不存在时回退内置演示常量表，如实标 `"source":"demo"`（逐件回退时
//! `"source":"mixed"`）。服务端没有目录列表 API，因此目录的条数与品类仍以常量表
//! 为索引，价格等字段以 server 标注如实呈现。

use crate::data::{item, yuan, Category, LiyuState, PactState, CATALOG};
use crate::{commerce_client, profile_client};
use makepad_app_module::makepad_ai_services::wire::{Risk, ServiceCall, ServiceManifest, ToolDef, ToolResult};

/// 本轮注册的工具列表。
pub const TOOL_NAMES: [&str; 5] = [
    "list_gift_catalog",
    "suggest_gift",
    "get_gift_box_summary",
    "get_gift_detail",
    "list_gift_candidates",
];

const CATALOG_ARGS: &str = r#"{"type":"object","properties":{"category":{"type":"string","description":"可选：咖啡茶饮 / 电影演出 / 潮流小物 / 盲盒 / 甜点鲜花 / 数码家电 / 家居生活 / 母婴亲子（也认 coffee / movie / trendy / blind / sweet / digital / home / baby）"}}}"#;
const SUGGEST_ARGS: &str = r#"{"type":"object","properties":{"budget":{"type":"number","description":"预算，单位元（上限 100000，必须是正数）"},"occasion":{"type":"string","description":"可选：场合，如 生日 / 感谢 / 道歉 / 约会 / 加油（上限 64 字符）"}},"required":["budget"]}"#;
const NO_ARGS: &str = r#"{"type":"object","properties":{}}"#;
const DETAIL_ARGS: &str = r#"{"type":"object","properties":{"id":{"type":"integer","description":"目录下标，0 起，来自 list_gift_catalog / list_gift_candidates 的 id 字段"}},"required":["id"]}"#;
const CANDIDATES_ARGS: &str = r#"{"type":"object","properties":{"category":{"type":"string","description":"可选：同 list_gift_catalog"},"budget":{"type":"number","description":"可选：只看预算（元）内的，上限 100000"},"keyword":{"type":"string","description":"可选：在名称、品类、规格卖点、描述里搜这个词（上限 64 字符）"}}}"#;

/// 参数整体上限：输入最多 4 KiB、JSON 嵌套最深 16 层（serde_json 默认 128 更宽松）。
const MAX_ARGS_BYTES: usize = 4096;
const MAX_DEPTH: usize = 16;
/// 单个字符串参数上限。
const MAX_STR_CHARS: usize = 64;
/// 预算上限：10 万元（分）。
const MAX_BUDGET_CENTS: i64 = 100_000 * 100;

pub fn manifest() -> ServiceManifest {
    ServiceManifest::new(
        "liyu",
        "礼遇 LiYu",
        "礼遇演示应用：匿名悬念送礼。挑一份礼物、设一道谜题、附一份小契约，对方解开才知道是谁送的。只读能力：礼物目录、商品详情、候选检索、预算内挑礼建议和礼盒计数摘要；不会输出谜题答案、暗号、未揭晓的送礼人、地址或别人的心愿。另可准备一份本机送礼草稿（prepare_gift_draft，Risk::Act）：只在本机打开待本人在界面确认，不会替人送礼、解谜、收下或折现，也不会自动扣款或改变服务端礼物状态——送不送由本人决定。",
    )
    .with_tool(ToolDef::new(
        TOOL_NAMES[0],
        "礼物目录：名称、品类、形态（实物 / 电子券）、价格（元）。可按品类过滤。在线时价格为服务端当前价（source=server），离线为演示价（source=demo）。",
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
        "礼盒摘要：收到 / 送出的礼物按状态计数、余额、待兑现契约数。不含送礼人或答案。",
        NO_ARGS,
        Risk::Read,
    ))
    .with_tool(ToolDef::new(
        TOOL_NAMES[3],
        "目录里一件商品的详情：名称、品牌、品类、形态、规格、卖点、描述、价格、是否可订。只含公开商品信息。",
        DETAIL_ARGS,
        Risk::Read,
    ))
    .with_tool(ToolDef::new(
        TOOL_NAMES[4],
        "按品类、预算、关键词检索候选礼物，回 id 列表（可交给 get_gift_detail 看详情）。只含公开商品信息。",
        CANDIDATES_ARGS,
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
        for g in s.received(crate::data::today_days()) {
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

// ---- 目录投影：公开商品信息 + 来源标注 ----

/// 目录来源：服务端当前数据 / 演示常量表 / 逐件混合。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Server,
    Demo,
    Mixed,
}

impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Source::Server => "server",
            Source::Demo => "demo",
            Source::Mixed => "mixed",
        }
    }
}

/// 一件商品对外可见的投影：只含目录公开字段，类型上就没有任何礼物实例 /
/// 送礼人 / 谜底 / 地址 / 券码 / 心愿数据的口子（同 [`BoxSummary`] 模式）。
#[derive(Clone, Debug, PartialEq)]
pub struct PubGiftDetail {
    /// 目录下标（`list_gift_catalog` / `list_gift_candidates` 里的 `id`）。
    pub id: u16,
    pub name: String,
    pub brand: String,
    pub category: &'static str,
    pub form: &'static str,
    pub spec: String,
    pub tags: Vec<String>,
    pub description: String,
    /// 分。
    pub price_cents: i64,
    pub available: bool,
    pub source: Source,
}

impl PubGiftDetail {
    /// 常量表一件（离线 / 无服务端数据时）。
    fn from_catalog(id: u16) -> Self {
        let it = item(id);
        PubGiftDetail {
            id,
            name: it.name.into(),
            brand: it.brand.into(),
            category: it.cat.label(),
            form: if it.physical { "实物" } else { "电子券" },
            spec: it.spec.into(),
            tags: it.tags.iter().map(|t| (*t).into()).collect(),
            description: it.desc.into(),
            price_cents: it.price,
            available: true,
            source: Source::Demo,
        }
    }

    /// 在线态用服务端当前数据覆盖名称 / 描述 / 价格 / 可订状态；服务端这一项
    /// 不存在或不可达时原样保留常量表字段并仍标 demo（如实标注，不假装在线）。
    fn fetch(id: u16) -> Self {
        let mut d = Self::from_catalog(id);
        if !profile_client::is_online() {
            return d;
        }
        if let Some(p) = commerce_client::product_detail(id) {
            d.name = p.name;
            if !p.brand.is_empty() {
                d.brand = p.brand;
            }
            d.description = p.description;
            d.price_cents = p.price_cents;
            d.available = p.available;
            d.source = Source::Server;
        }
        d
    }

    /// 列表投影（详情里的子集），价格单位元。
    fn row_json(&self) -> String {
        format!(
            "{{\"id\":{},\"name\":{},\"category\":{},\"form\":{},\"spec\":{},\"price\":{}}}",
            self.id,
            json_str(&self.name),
            json_str(self.category),
            json_str(self.form),
            json_str(&self.spec),
            yuan_num(self.price_cents),
        )
    }

    fn detail_json(&self) -> String {
        format!(
            "{{\"id\":{},\"name\":{},\"brand\":{},\"category\":{},\"form\":{},\"spec\":{},\"tags\":[{}],\"description\":{},\"price\":{},\"available\":{},\"source\":{}}}",
            self.id,
            json_str(&self.name),
            json_str(&self.brand),
            json_str(self.category),
            json_str(self.form),
            json_str(&self.spec),
            self.tags.iter().map(|t| json_str(t)).collect::<Vec<_>>().join(","),
            json_str(&self.description),
            yuan_num(self.price_cents),
            self.available,
            json_str(self.source.as_str()),
        )
    }
}

/// 拉一组商品的投影并汇总来源（逐件回退时 mixed）。
fn fetch_details(ids: &[u16]) -> (Vec<PubGiftDetail>, Source) {
    let rows: Vec<PubGiftDetail> = ids.iter().map(|&i| PubGiftDetail::fetch(i)).collect();
    let src = if rows.iter().all(|d| d.source == Source::Server) {
        Source::Server
    } else if rows.iter().any(|d| d.source == Source::Server) {
        Source::Mixed
    } else {
        Source::Demo
    };
    (rows, src)
}

fn source_note(src: Source) -> &'static str {
    match src {
        Source::Server => "名称、描述、价格、可订状态为服务端当前数据（source=server）",
        Source::Demo => "离线或服务端无此项：使用内置演示目录（source=demo），价格可能与线上不一致",
        Source::Mixed => "部分商品用了服务端数据、部分回退演示目录（source=mixed），逐件 source 为准",
    }
}

pub fn answer(summary: &BoxSummary, call: &ServiceCall) -> ToolResult {
    match call.tool.as_str() {
        "list_gift_catalog" => {
            let args = match parse_args_for(&call.args, &["category"]) {
                Ok(a) => a,
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let raw = match arg_string(&args, "category", MAX_STR_CHARS) {
                Ok(v) => v,
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let cat = raw.as_deref().and_then(parse_category);
            if raw.as_deref().is_some_and(|r| !r.trim().is_empty()) && cat.is_none() {
                return ToolResult::refused(
                    &call.call_id,
                    "不认识这个品类；可选：咖啡茶饮 / 电影演出 / 潮流小物 / 盲盒 / 甜点鲜花",
                );
            }
            let ids = crate::data::catalog_in(cat);
            let (rows, src) = fetch_details(&ids);
            let body = format!(
                "{{\"source\":{},\"items\":[{}],\"currency\":\"CNY\"}}",
                json_str(src.as_str()),
                rows.iter().map(PubGiftDetail::row_json).collect::<Vec<_>>().join(","),
            );
            ToolResult::ok(&call.call_id, body, source_note(src))
        }
        "suggest_gift" => {
            let args = match parse_args_for(&call.args, &["budget", "occasion"]) {
                Ok(a) => a,
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let budget = match arg_budget(&args, "budget", true) {
                Ok(Some(b)) => b,
                Ok(None) => return ToolResult::refused(&call.call_id, "需要一个大于 0 的预算（元）"),
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let occasion = match arg_string(&args, "occasion", MAX_STR_CHARS) {
                Ok(v) => v.unwrap_or_default(),
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            ToolResult::ok(
                &call.call_id,
                suggest_json(budget, &occasion),
                "只从目录里挑（演示常量表价格），建议仅供参考，送不送由本人决定",
            )
        }
        "get_gift_box_summary" => {
            if let Err(why) = parse_args_for(&call.args, &[]) {
                return ToolResult::refused(&call.call_id, why);
            }
            ToolResult::ok(
                &call.call_id,
                summary_json(summary),
                "只有计数与金额：不含送礼人、答案、暗号或地址",
            )
        }
        "get_gift_detail" => {
            let args = match parse_args_for(&call.args, &["id"]) {
                Ok(a) => a,
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let id = match arg_id(&args, "id") {
                Ok(v) => v,
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let d = PubGiftDetail::fetch(id);
            ToolResult::ok(&call.call_id, d.detail_json(), source_note(d.source))
        }
        "list_gift_candidates" => {
            let args = match parse_args_for(&call.args, &["category", "budget", "keyword"]) {
                Ok(a) => a,
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let cat = match arg_string(&args, "category", MAX_STR_CHARS) {
                Ok(v) => v,
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let cat = cat.as_deref().and_then(parse_category);
            let budget = match arg_budget(&args, "budget", false) {
                Ok(v) => v,
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let keyword = match arg_string(&args, "keyword", MAX_STR_CHARS) {
                Ok(v) => v.unwrap_or_default(),
                Err(why) => return ToolResult::refused(&call.call_id, why),
            };
            let kw = keyword.trim().to_lowercase();
            let ids: Vec<u16> = crate::data::catalog_in(cat)
                .into_iter()
                .filter(|&i| {
                    let it = item(i);
                    let in_budget = budget.map_or(true, |b| it.price <= b);
                    let hit = kw.is_empty()
                        || it.name.to_lowercase().contains(&kw)
                        || it.cat.label().contains(&kw)
                        || it.kind.contains(&kw)
                        || it.spec.to_lowercase().contains(&kw)
                        || it.desc.to_lowercase().contains(&kw)
                        || it.tags.iter().any(|t| t.to_lowercase().contains(&kw));
                    in_budget && hit
                })
                .collect();
            if ids.is_empty() {
                return ToolResult::ok(
                    &call.call_id,
                    "{\"source\":\"demo\",\"items\":[]}",
                    "没有符合条件的礼物，放宽预算或换个关键词试试（演示目录）",
                );
            }
            let (rows, src) = fetch_details(&ids);
            let body = format!(
                "{{\"source\":{},\"items\":[{}],\"currency\":\"CNY\"}}",
                json_str(src.as_str()),
                rows.iter().map(PubGiftDetail::row_json).collect::<Vec<_>>().join(","),
            );
            ToolResult::ok(&call.call_id, body, source_note(src))
        }
        other => ToolResult::refused(
            &call.call_id,
            format!("liyu 没有 `{other}` 工具；只能查目录、详情、候选、挑礼建议和礼盒摘要，不能代为操作"),
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
            Category::Digital => "digital",
            Category::Home => "home",
            Category::Baby => "baby",
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

/// 场合 → 优先品类 + 一句话。认不出的场合按价格挑。
fn occasion_hint(occasion: &str) -> (&'static [Category], &'static str) {
    const RULES: [(&[&str], &[Category], &str); 9] = [
        (&["生日", "birthday"], &[Category::Sweet, Category::Blind], "生日配点甜的，或者拆盲盒的惊喜"),
        (&["感谢", "谢谢", "thanks"], &[Category::Coffee, Category::Sweet], "一杯咖啡的谢意刚刚好，不让对方有负担"),
        (&["道歉", "对不起", "sorry"], &[Category::Sweet, Category::Coffee], "先递一份甜的，话更好说"),
        (&["约会", "电影", "date"], &[Category::Movie, Category::Sweet], "票在手里，下一次见面就有了理由"),
        (&["加油", "考试", "上班", "打气"], &[Category::Coffee, Category::Blind], "提神的咖啡，或者一点小期待"),
        (&["乔迁", "搬家", "新家"], &[Category::Home, Category::Digital], "新家缺的往往是用得上的东西，每天都会想起你"),
        (&["结婚", "婚礼", "新婚", "wedding"], &[Category::Home, Category::Sweet], "两个人过日子用得上的，再配一束花"),
        (&["宝宝", "满月", "出生", "baby"], &[Category::Baby, Category::Sweet], "给小朋友的第一份礼物，也照顾到新手爸妈"),
        (&["纪念", "毕业"], &[Category::Trendy, Category::Digital], "能留下来的小物件，看到就会想起你"),
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
    let items: Vec<String> = picks.iter().map(|&i| PubGiftDetail::from_catalog(i).row_json()).collect();
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

// ---- 参数：serde_json 结构化解析 + 明确限额，拒绝绕过 ----

/// 解析参数原文为 JSON object。拒绝：超长输入、深度超限、语法错误、重复键、
/// 顶层不是 object。
fn parse_args(args: &str) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    if args.len() > MAX_ARGS_BYTES {
        return Err(format!("参数太长（{} 字节，上限 {MAX_ARGS_BYTES}）", args.len()));
    }
    // 先独立扫一遍结构：嵌套深度超限、重复键都在这里拒掉（serde_json 的默认
    // 递归上限更宽松，且重复键是后者静默覆盖前者，会绕过语义检查）。
    match scan_structure(args) {
        Scan::Bad => return Err("参数不是合法 JSON object 或嵌套太深".into()),
        Scan::DuplicateKey => return Err("参数里有重复的键".into()),
        Scan::Clean => {}
    }
    let value: serde_json::Value =
        serde_json::from_str(args).map_err(|e| format!("参数不是合法 JSON：{e}"))?;
    let serde_json::Value::Object(map) = value else {
        return Err("参数必须是 JSON object".into());
    };
    Ok(map)
}

/// 严格 schema:每个工具只认自己的参数键,未知键(如给 list_gift_catalog 塞 budget)
/// 直接拒绝,避免「该工具不读的键被静默忽略」绕过调用方预期。
fn parse_args_for(
    args: &str,
    allowed: &[&str],
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let map = parse_args(args)?;
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("不认识的参数 `{key}`"));
        }
    }
    Ok(map)
}

enum Scan {
    Clean,
    Bad,
    DuplicateKey,
}

/// 独立结构扫描：重复键检测（serde_json 是后者覆盖前者——
/// `{"budget":1,"budget":999999}` 会静默绕过语义检查）+ 嵌套深度上限。
fn scan_structure(args: &str) -> Scan {
    struct Scanner<'a> {
        s: &'a [u8],
        i: usize,
        dup: bool,
    }
    impl<'a> Scanner<'a> {
        fn ws(&mut self) {
            while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
            }
        }
        /// 解析一个字符串字面量（当前位置是 `"`），返回其内容字节。
        fn string(&mut self) -> Option<Vec<u8>> {
            if self.s.get(self.i) != Some(&b'"') {
                return None;
            }
            self.i += 1;
            let mut out = Vec::new();
            while let Some(&c) = self.s.get(self.i) {
                self.i += 1;
                match c {
                    b'"' => return Some(out),
                    b'\\' => {
                        let e = *self.s.get(self.i)?;
                        self.i += 1;
                        if e == b'u' {
                            // \uXXXX:归一化为实际标量,使转义键与字面值键相等(防重复键绕过)。
                            let mut code = 0u32;
                            for _ in 0..4 {
                                let d = *self.s.get(self.i)?;
                                self.i += 1;
                                code = code * 16
                                    + match d {
                                        b'0'..=b'9' => (d - b'0') as u32,
                                        b'a'..=b'f' => (d - b'a' + 10) as u32,
                                        b'A'..=b'F' => (d - b'A' + 10) as u32,
                                        _ => return None,
                                    };
                            }
                            match char::from_u32(code) {
                                Some(ch) => {
                                    let mut buf = [0u8; 4];
                                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                                }
                                // 孤立代理等非法标量:保留原文,不参与相等性合并。
                                None => {
                                    out.extend_from_slice(b"\\u");
                                    out.extend_from_slice(format!("{code:04x}").as_bytes());
                                }
                            }
                        } else {
                            out.push(e);
                        }
                    }
                    c => out.push(c),
                }
            }
            None
        }
        /// 跳过一个值（object / array 递归）。
        fn value(&mut self, depth: usize) -> Option<()> {
            if depth > MAX_DEPTH {
                return None;
            }
            self.ws();
            match *self.s.get(self.i)? {
                b'{' => self.object(depth + 1),
                b'[' => {
                    self.i += 1;
                    self.ws();
                    if self.s.get(self.i) == Some(&b']') {
                        self.i += 1;
                        return Some(());
                    }
                    loop {
                        // 数组嵌套也计深度(与 object 一致),否则 `[[[...]]]` 绕过 MAX_DEPTH。
                        self.value(depth + 1)?;
                        self.ws();
                        match *self.s.get(self.i)? {
                            b',' => {
                                self.i += 1;
                            }
                            b']' => {
                                self.i += 1;
                                return Some(());
                            }
                            _ => return None,
                        }
                    }
                }
                b'"' => self.string().map(|_| ()),
                _ => {
                    // 数字 / true / false / null：扫到定界符。
                    while self.i < self.s.len() && !matches!(self.s[self.i], b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r') {
                        self.i += 1;
                    }
                    Some(())
                }
            }
        }
        fn object(&mut self, depth: usize) -> Option<()> {
            if depth > MAX_DEPTH {
                return None;
            }
            debug_assert_eq!(self.s.get(self.i), Some(&b'{'));
            self.i += 1;
            let mut seen: Vec<Vec<u8>> = Vec::new();
            self.ws();
            if self.s.get(self.i) == Some(&b'}') {
                self.i += 1;
                return Some(());
            }
            loop {
                self.ws();
                let key = self.string()?;
                if seen.iter().any(|k| *k == key) {
                    self.dup = true;
                    return None;
                }
                seen.push(key);
                self.ws();
                if *self.s.get(self.i)? != b':' {
                    return None;
                }
                self.i += 1;
                self.value(depth)?;
                self.ws();
                match *self.s.get(self.i)? {
                    b',' => {
                        self.i += 1;
                    }
                    b'}' => {
                        self.i += 1;
                        return Some(());
                    }
                    _ => return None,
                }
            }
        }
    }
    let mut sc = Scanner { s: args.as_bytes(), i: 0, dup: false };
    sc.ws();
    match sc.s.get(sc.i) {
        Some(b'{') => {
            if sc.dup {
                return Scan::DuplicateKey;
            }
            // 必须整个 object 扫完且后面只有空白，否则是坏结构。
            match sc.object(1) {
                Some(()) if sc.dup => Scan::DuplicateKey,
                Some(()) => {
                    sc.ws();
                    if sc.i == sc.s.len() { Scan::Clean } else { Scan::Bad }
                }
                None if sc.dup => Scan::DuplicateKey,
                None => Scan::Bad,
            }
        }
        // 顶层非 object：结构问题由 serde / 调用方报，不在此误判重复键。
        _ => Scan::Clean,
    }
}

/// 字符串参数：必须是 JSON string（null / 缺失视为无），限长，拒绝数字布尔等
/// 其他类型被隐式当字符串用。
fn arg_string(
    args: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    max_chars: usize,
) -> Result<Option<String>, String> {
    match args.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => {
            if s.chars().count() > max_chars {
                Err(format!("`{key}` 太长（上限 {max_chars} 字符）"))
            } else {
                Ok(Some(s.clone()))
            }
        }
        Some(_) => Err(format!("`{key}` 必须是字符串")),
    }
}

/// 预算（元 → 分）：JSON number 或数字字符串都认（向后兼容 `"60"`、`"¥66"`），
/// 必须有限、>0、不超过上限。`required=true` 时缺失返回 Ok(None) 让调用方给专门文案。
fn arg_budget(
    args: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    required: bool,
) -> Result<Option<i64>, String> {
    let raw = match args.get(key) {
        None | Some(serde_json::Value::Null) => return Ok(None),
        Some(serde_json::Value::Number(n)) => n.as_f64().ok_or("`budget` 数值超出表示范围")?,
        Some(serde_json::Value::String(s)) => {
            let t = s.trim().trim_start_matches('¥');
            t.parse::<f64>().map_err(|_| format!("`{key}` 字符串不是数字"))?
        }
        Some(_) => return Err(format!("`{key}` 必须是数字")),
    };
    if !raw.is_finite() || raw <= 0.0 {
        if required {
            return Ok(None); // suggest_gift 的专属文案。
        }
        return Err(format!("`{key}` 必须是大于 0 的数"));
    }
    let cents = (raw * 100.0).round();
    if cents > MAX_BUDGET_CENTS as f64 {
        return Err(format!("`{key}` 超过上限 {} 元", MAX_BUDGET_CENTS / 100));
    }
    Ok(Some(cents as i64))
}

/// 目录下标：必须是整数（number 或整数字符串），范围 [0, CATALOG.len())。
fn arg_id(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> Result<u16, String> {
    let n = match args.get(key) {
        None => return Err(format!("需要 `{key}`（目录下标）")),
        Some(serde_json::Value::Null) => return Err(format!("`{key}` 不能是 null")),
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .filter(|_| n.as_f64().is_some_and(|f| f.fract() == 0.0))
            .ok_or(format!("`{key}` 必须是非负整数"))?,
        Some(serde_json::Value::String(s)) => s
            .trim()
            .parse::<u64>()
            .map_err(|_| format!("`{key}` 必须是非负整数"))?,
        Some(_) => return Err(format!("`{key}` 必须是整数")),
    };
    u16::try_from(n)
        .ok()
        .filter(|&i| (i as usize) < CATALOG.len())
        .ok_or_else(|| format!("`{key}` 超出目录范围（0..{}）", CATALOG.len()))
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
    fn manifest_validates_with_five_read_tools() {
        let m = manifest();
        m.validate().unwrap();
        assert_eq!(m.tools.len(), 5);
        for name in TOOL_NAMES {
            let t = m.tool(name).expect("工具应在清单里");
            assert_eq!(t.risk, Risk::Read);
        }
    }

    #[test]
    fn unknown_or_write_tools_are_refused() {
        let s = BoxSummary::default();
        for tool in ["send_gift", "submit_answer", "cash_out", "who_sent_this", "list_wishes_of"] {
            assert_eq!(answer(&s, &call(tool, "{}")).outcome, ToolOutcome::Refused, "{tool}");
        }
    }

    #[test]
    fn catalog_lists_all_or_one_category_with_source() {
        let s = BoxSummary::default();
        let r = answer(&s, &call("list_gift_catalog", "{}"));
        assert_eq!(r.outcome, ToolOutcome::Ok);
        for it in CATALOG.iter() {
            assert!(r.text.contains(it.name), "{}", r.text);
        }
        assert!(r.text.contains("\"price\":109"), "{}", r.text);
        // 测试环境未登录：如实标注演示目录。
        assert!(r.text.contains("\"source\":\"demo\""), "{}", r.text);
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
        assert_eq!(sum.received.iter().sum::<u32>() as usize, s.received(crate::data::today_days()).len());
        assert_eq!(sum.sent.iter().sum::<u32>() as usize, s.sent().len());
        assert_eq!((sum.pacts_mine + sum.pacts_theirs) as usize, s.open_pacts());
        assert_eq!(STATE_KEYS[GiftState::CashedOut.id() as usize], "cashed_out");
        let r = answer(&sum, &call("get_gift_box_summary", "{}"));
        assert_eq!(r.outcome, ToolOutcome::Ok);
        assert!(r.text.contains("\"received\":{\"sealed\":"), "{}", r.text);
        assert!(r.text.contains(&format!("\"balance\":{}", yuan_num(s.balance()))), "{}", r.text);
    }

    #[test]
    fn detail_returns_public_fields_with_source() {
        let s = BoxSummary::default();
        let r = answer(&s, &call("get_gift_detail", r#"{"id":0}"#));
        assert_eq!(r.outcome, ToolOutcome::Ok, "{}", r.text);
        let it = item(0);
        assert!(r.text.contains(it.name) && r.text.contains(it.brand), "{}", r.text);
        assert!(r.text.contains(it.desc), "{}", r.text);
        assert!(r.text.contains("\"available\":true"), "{}", r.text);
        assert!(r.text.contains("\"source\":\"demo\""), "{}", r.text);
        assert!(r.text.contains(&format!("\"price\":{}", yuan_num(it.price))), "{}", r.text);
        // 字符串 id 也认。
        let r2 = answer(&s, &call("get_gift_detail", r#"{"id":"1"}"#));
        assert_eq!(r2.outcome, ToolOutcome::Ok, "{}", r2.text);
        assert!(r2.text.contains(item(1).name), "{}", r2.text);
    }

    #[test]
    fn detail_rejects_bad_ids() {
        let s = BoxSummary::default();
        for args in [
            r#"{}"#,                       // 缺 id
            r#"{"id":-1}"#,                // 负数
            r#"{"id":1.5}"#,               // 非整数
            r#"{"id":999}"#,               // 越界
            r#"{"id":true}"#,              // 错误类型
            r#"{"id":null}"#,              // null
            r#"{"id":"0","id":1}"#,        // 重复键
        ] {
            assert_eq!(answer(&s, &call("get_gift_detail", args)).outcome, ToolOutcome::Refused, "{args}");
        }
    }

    #[test]
    fn candidates_filter_by_budget_category_keyword() {
        let s = BoxSummary::default();
        // 关键词搜名称。
        let r = answer(&s, &call("list_gift_candidates", r#"{"keyword":"拿铁"}"#));
        assert_eq!(r.outcome, ToolOutcome::Ok);
        assert!(r.text.contains("星巴克中杯拿铁电子券"), "{}", r.text);
        // 有命中时带 id，可交给 get_gift_detail。
        assert!(r.text.contains("\"id\":"), "{}", r.text);
        assert!(!r.text.contains("电影通兑票"), "{}", r.text);
        // 预算过滤。
        let r = answer(&s, &call("list_gift_candidates", r#"{"budget":40}"#));
        assert!(r.text.contains("喜茶多肉葡萄兑换券"), "{}", r.text);
        assert!(!r.text.contains("三顿半精品咖啡礼盒"), "{}", r.text);
        // 品类 + 预算组合。
        let r = answer(&s, &call("list_gift_candidates", r#"{"category":"电影演出","budget":50}"#));
        assert!(r.text.contains("电影通兑票"), "{}", r.text);
        assert!(!r.text.contains("电影双人套票"), "{}", r.text);
        // 无命中：空列表也是 Ok。
        let r = answer(&s, &call("list_gift_candidates", r#"{"keyword":"潜水艇"}"#));
        assert_eq!(r.outcome, ToolOutcome::Ok);
        assert!(r.text.contains("\"items\":[]"), "{}", r.text);
    }

    #[test]
    fn malicious_json_is_refused() {
        let s = BoxSummary::default();
        let refused = [
            // 语法错误。
            r#"{"budget":}"#,
            r#"{"budget":50"#,           // 截断
            r#"[1,2,3]"#,                // 顶层数组
            r#"50"#,                     // 顶层数字
            r#"{"budget":50}}"#,         // 多余括号
            // 重复键绕过（后者覆盖前者会静默放大预算）。
            r#"{"budget":1,"budget":99999}"#,
            r#"{"category":"sweet","category":"盲盒"}"#,
            // 类型错误。
            r#"{"budget":true}"#,
            r#"{"budget":[50]}"#,
            r#"{"budget":{"x":1}}"#,
            r#"{"occasion":42}"#,
            // 超限。
            r#"{"budget":200000}"#,      // 超预算上限
            r#"{"budget":-5}"#,          // 非正数
            r#"{"budget":"abc"}"#,       // 字符串不是数字
        ];
        for args in refused {
            assert_eq!(answer(&s, &call("suggest_gift", args)).outcome, ToolOutcome::Refused, "{args}");
            assert_eq!(answer(&s, &call("list_gift_catalog", args)).outcome, ToolOutcome::Refused, "{args}");
            assert_eq!(answer(&s, &call("list_gift_candidates", args)).outcome, ToolOutcome::Refused, "{args}");
        }
        // 超长字符串参数。
        let long = format!(r#"{{"occasion":"{}"}}"#, "长".repeat(200));
        assert_eq!(answer(&s, &call("suggest_gift", &long)).outcome, ToolOutcome::Refused);
        let long = format!(r#"{{"keyword":"{}"}}"#, "长".repeat(200));
        assert_eq!(answer(&s, &call("list_gift_candidates", &long)).outcome, ToolOutcome::Refused);
        // 超长整体输入（>4KiB）。
        let huge = format!(r#"{{"occasion":"{}"}}"#, "长".repeat(5000));
        assert_eq!(answer(&s, &call("suggest_gift", &huge)).outcome, ToolOutcome::Refused);
        // 超深嵌套（顶层 object 里塞深数组），不崩溃且拒绝。
        let deep = format!("{{\"x\":{}1{}}}", "[".repeat(200), "]".repeat(200));
        assert_eq!(answer(&s, &call("get_gift_box_summary", &deep)).outcome, ToolOutcome::Refused);
        // 空参数、带空白都正常。
        assert_eq!(answer(&s, &call("suggest_gift", r#"{ "budget" : 88.5 }"#)).outcome, ToolOutcome::Ok);
        assert_eq!(answer(&s, &call("get_gift_box_summary", "  {}  ")).outcome, ToolOutcome::Ok);
    }

    #[test]
    fn duplicate_key_scanner() {
        fn dup(args: &str) -> bool { matches!(scan_structure(args), Scan::DuplicateKey) }
        assert!(dup(r#"{"a":1,"a":2}"#));
        assert!(dup(r#"{"a":{"b":1,"b":2}}"#));
        assert!(dup(r#"{"a":1,"b":{"c":1,"c":2}}"#));
        assert!(!dup(r#"{"a":1,"b":2}"#));
        assert!(!dup(r#"{"a":{"b":1},"b":2}"#));
        assert!(!dup(r#"{"a":[1,2,{"b":3}]}"#));
        // 转义后的相同键也算重复。
        assert!(dup(r#"{"bud\u0067et":1,"budget":2}"#));
        // 坏语法 / 过深：报 Bad 而不是误判重复键（serde 那边也会拒）。
        assert!(matches!(scan_structure(r#"{"a":"unclosed"#), Scan::Bad));
        assert!(matches!(scan_structure(&format!("{{\"x\":{}1{}}}", "[".repeat(40), "]".repeat(40))), Scan::Bad));
        assert!(matches!(scan_structure("{}"), Scan::Clean));
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
            ..Default::default()
        };
        s.send_gift(&d, TEST_TODAY).unwrap();
        s.settings.ship_phone = "13800138000".into();
        s.settings.ship_addr = "望京 SOHO T3".into();
        let sum = BoxSummary::from_state(&s);

        let mut forbidden: Vec<String> = Vec::new();
        for g in &s.gifts {
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
            call("list_gift_candidates", r#"{"budget":500,"keyword":"礼物"}"#),
        ];
        for c in &calls {
            let r = answer(&sum, c);
            assert_eq!(r.outcome, ToolOutcome::Ok, "{}", c.tool);
            for f in &forbidden {
                assert!(!r.text.contains(f.as_str()), "{} 输出泄露 {f}", c.tool);
                assert!(!r.note.contains(f.as_str()), "{} 备注泄露 {f}", c.tool);
            }
        }
        // get_gift_detail 单独跑：目录里每件商品的详情输出也不允许带出任何私密串。
        for id in 0..CATALOG.len() {
            let r = answer(&sum, &call("get_gift_detail", &format!(r#"{{"id":{id}}}"#)));
            assert_eq!(r.outcome, ToolOutcome::Ok);
            for f in &forbidden {
                assert!(!r.text.contains(f.as_str()), "detail {id} 泄露 {f}");
                assert!(!r.note.contains(f.as_str()), "detail {id} 备注泄露 {f}");
            }
        }
    }

    #[test]
    fn projection_never_reaches_state() {
        // 类型层保证：PubGiftDetail 只有公开商品字段，from_catalog/fetch 的入参是
        // 目录下标而不是状态——任何隐私字段都无从流入。这里静态确认字段集合。
        let d = PubGiftDetail::from_catalog(0);
        let json = d.detail_json();
        for banned in ["peer", "answer", "clue", "addr", "phone", "voucher", "wish", "message", "contract"] {
            assert!(!json.contains(banned), "投影里出现敏感键 {banned}");
        }
    }
}
