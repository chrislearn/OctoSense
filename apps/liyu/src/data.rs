//! 礼遇 LiYu 的数据模型与纯逻辑：礼物目录、神秘礼物、社交契约、余额流水。
//!
//! 设计见 `liyu/docs/`：规则（04-rules.md）里的每个常量、每个函数都在这里，
//! 实体与持久化边界见 05-data-model.md。
//!
//! 约定：
//! - 金额一律是「分」（`i64`），显示时才转成 ¥x；
//! - 日期一律是「天序号」（1970-01-01 起的 UTC 天数），`today` 从外面传进来，
//!   所以过期、到期、通知这些跟时间有关的规则都能直接写单测；
//! - 枚举落盘存 `u8`，读出来认不得的值一律退回第一项，不 panic；
//! - 没有服务器、没有真实支付：「对方」由 `simulate_step` 驱动，
//!   余额不够的部分记一条「模拟支付」，不影响余额。
use makepad_widgets::makepad_micro_serde::*;

// ---- 日期 ----
//
// 本地时区偏移没有可移植的平台 API，演示统一用 UTC 日期。

/// 1970-01-01（周四）以来的 UTC 天数。
pub fn today_days() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (secs / 86400) as i64
}

/// 今天过了多少分钟（UTC）。通知判断要的就是这一个数。
///
/// 和 `today_days` 一样按 UTC 算：本地时区偏移没有可移植的平台 API，
/// 而通知的语义（「这个时段还剩半小时」）在两种时区下都成立。
pub fn minutes_of_day() -> u32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    ((secs % 86400) / 60) as u32
}

/// 今天的 ISO 日期字符串。
pub fn today_iso() -> String {
    fmt_days(today_days())
}

/// 格里历 → 天数（Howard Hinnant days_from_civil）。
pub fn civil_to_days(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = ((m as i64) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + (d as i64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// 天数 → 格里历（年, 月, 日）。
pub fn days_to_civil(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

/// 严格解析 YYYY-MM-DD；「今天」「09/12」等旧格式一律 None。
pub fn parse_iso_days(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |i: usize, n: usize| -> Option<i64> {
        let mut v: i64 = 0;
        for k in 0..n {
            let c = b[i + k];
            if !c.is_ascii_digit() {
                return None;
            }
            v = v * 10 + (c - b'0') as i64;
        }
        Some(v)
    };
    let y = num(0, 4)? as i32;
    let m = num(5, 2)? as u32;
    let d = num(8, 2)? as u32;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(civil_to_days(y, m, d))
}

/// 天数 → "YYYY-MM-DD"。
pub fn fmt_days(days: i64) -> String {
    let (y, m, d) = days_to_civil(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// 天数 → "MM/DD"（曲线 x 轴首尾标注）。
/// 「9 月 28 日」：心愿单、约好送到的日子都用这种写法。
pub fn md_cn(days: i64) -> String {
    let (_, m, d) = days_to_civil(days);
    format!("{m} 月 {d} 日")
}

pub fn fmt_md(days: i64) -> String {
    let (_, m, d) = days_to_civil(days);
    format!("{:02}/{:02}", m, d)
}

/// 周一为一周的起点（1970-01-01 是周四）。
pub fn week_start(days: i64) -> i64 {
    days - (days + 3).rem_euclid(7)
}

/// 周几，0 = 周一 … 6 = 周日（1970-01-01 是周四）。
pub fn weekday(days: i64) -> usize {
    (days + 3).rem_euclid(7) as usize
}

/// 周末（周六 / 周日）。商圈、公园的机会在周末更高，办公园区相反。
pub fn is_weekend(days: i64) -> bool {
    weekday(days) >= 5
}

// ---- 熟人索引 ----

/// 常见姓氏与拼音首字母。
///
/// 为什么是一张姓氏表而不是完整的汉字拼音表：这一栏索引的是**人名**，
/// 一张一百多字的姓氏表就能覆盖绝大多数称呼，而完整拼音表要几万条、还得
/// 处理多音字。认不出来的一律进「#」那一组，不猜。
const SURNAME_INITIALS: &[(&str, char)] = &[
    ("欧阳", 'O'), ("司马", 'S'), ("诸葛", 'Z'), ("上官", 'S'), ("慕容", 'M'), ("东方", 'D'),
    ("皇甫", 'H'), ("尉迟", 'Y'), ("公孙", 'G'), ("令狐", 'L'), ("艾", 'A'), ("安", 'A'),
    ("白", 'B'), ("包", 'B'), ("鲍", 'B'), ("贝", 'B'), ("毕", 'B'), ("卞", 'B'),
    ("卜", 'B'), ("柏", 'B'), ("蔡", 'C'), ("曹", 'C'), ("岑", 'C'), ("常", 'C'),
    ("车", 'C'), ("陈", 'C'), ("成", 'C'), ("程", 'C'), ("池", 'C'), ("褚", 'C'),
    ("崔", 'C'), ("昌", 'C'), ("戴", 'D'), ("邓", 'D'), ("狄", 'D'), ("丁", 'D'),
    ("董", 'D'), ("窦", 'D'), ("杜", 'D'), ("段", 'D'), ("樊", 'F'), ("范", 'F'),
    ("方", 'F'), ("房", 'F'), ("费", 'F'), ("冯", 'F'), ("凤", 'F'), ("傅", 'F'),
    ("符", 'F'), ("酆", 'F'), ("甘", 'G'), ("高", 'G'), ("戈", 'G'), ("葛", 'G'),
    ("龚", 'G'), ("古", 'G'), ("谷", 'G'), ("顾", 'G'), ("关", 'G'), ("管", 'G'),
    ("桂", 'G'), ("郭", 'G'), ("韩", 'H'), ("郝", 'H'), ("何", 'H'), ("和", 'H'),
    ("贺", 'H'), ("洪", 'H'), ("侯", 'H'), ("胡", 'H'), ("花", 'H'), ("华", 'H'),
    ("黄", 'H'), ("霍", 'H'), ("嵇", 'J'), ("汲", 'J'), ("计", 'J'), ("纪", 'J'),
    ("季", 'J'), ("贾", 'J'), ("江", 'J'), ("姜", 'J'), ("蒋", 'J'), ("焦", 'J'),
    ("金", 'J'), ("靳", 'J'), ("康", 'K'), ("柯", 'K'), ("孔", 'K'), ("寇", 'K'),
    ("邝", 'K'), ("赖", 'L'), ("蓝", 'L'), ("郎", 'L'), ("乐", 'L'), ("雷", 'L'),
    ("黎", 'L'), ("李", 'L'), ("连", 'L'), ("廉", 'L'), ("梁", 'L'), ("廖", 'L'),
    ("林", 'L'), ("凌", 'L'), ("刘", 'L'), ("柳", 'L'), ("龙", 'L'), ("卢", 'L'),
    ("鲁", 'L'), ("陆", 'L'), ("路", 'L'), ("吕", 'L'), ("罗", 'L'), ("骆", 'L'),
    ("马", 'M'), ("毛", 'M'), ("梅", 'M'), ("孟", 'M'), ("米", 'M'), ("苗", 'M'),
    ("闵", 'M'), ("明", 'M'), ("莫", 'M'), ("牟", 'M'), ("穆", 'M'), ("倪", 'N'),
    ("聂", 'N'), ("宁", 'N'), ("牛", 'N'), ("钮", 'N'), ("欧", 'O'), ("潘", 'P'),
    ("庞", 'P'), ("裴", 'P'), ("彭", 'P'), ("皮", 'P'), ("平", 'P'), ("戚", 'Q'),
    ("齐", 'Q'), ("祁", 'Q'), ("钱", 'Q'), ("乔", 'Q'), ("秦", 'Q'), ("邱", 'Q'),
    ("裘", 'Q'), ("曲", 'Q'), ("屈", 'Q'), ("覃", 'Q'), ("冉", 'R'), ("饶", 'R'),
    ("任", 'R'), ("阮", 'R'), ("邵", 'S'), ("佘", 'S'), ("申", 'S'), ("沈", 'S'),
    ("盛", 'S'), ("施", 'S'), ("石", 'S'), ("时", 'S'), ("史", 'S'), ("舒", 'S'),
    ("水", 'S'), ("司", 'S'), ("宋", 'S'), ("苏", 'S'), ("孙", 'S'), ("谭", 'T'),
    ("汤", 'T'), ("唐", 'T'), ("陶", 'T'), ("滕", 'T'), ("田", 'T'), ("童", 'T'),
    ("涂", 'T'), ("万", 'W'), ("汪", 'W'), ("王", 'W'), ("危", 'W'), ("韦", 'W'),
    ("卫", 'W'), ("魏", 'W'), ("温", 'W'), ("文", 'W'), ("翁", 'W'), ("邬", 'W'),
    ("巫", 'W'), ("吴", 'W'), ("伍", 'W'), ("武", 'W'), ("奚", 'X'), ("习", 'X'),
    ("郗", 'X'), ("夏", 'X'), ("向", 'X'), ("项", 'X'), ("萧", 'X'), ("肖", 'X'),
    ("谢", 'X'), ("辛", 'X'), ("邢", 'X'), ("熊", 'X'), ("徐", 'X'), ("许", 'X'),
    ("宣", 'X'), ("薛", 'X'), ("闫", 'Y'), ("阎", 'Y'), ("严", 'Y'), ("颜", 'Y'),
    ("晏", 'Y'), ("杨", 'Y'), ("姚", 'Y'), ("叶", 'Y'), ("伊", 'Y'), ("易", 'Y'),
    ("殷", 'Y'), ("尹", 'Y'), ("应", 'Y'), ("尤", 'Y'), ("游", 'Y'), ("于", 'Y'),
    ("余", 'Y'), ("俞", 'Y'), ("虞", 'Y'), ("喻", 'Y'), ("元", 'Y'), ("袁", 'Y'),
    ("岳", 'Y'), ("云", 'Y'), ("禹", 'Y'), ("臧", 'Z'), ("曾", 'Z'), ("翟", 'Z'),
    ("詹", 'Z'), ("张", 'Z'), ("章", 'Z'), ("赵", 'Z'), ("郑", 'Z'), ("钟", 'Z'),
    ("周", 'Z'), ("朱", 'Z'), ("诸", 'Z'), ("祝", 'Z'), ("庄", 'Z'), ("卓", 'Z'),
    ("宗", 'Z'), ("邹", 'Z'), ("祖", 'Z'), ("左", 'Z'),
];

/// 称呼前面这些字不是姓，是叫法。做索引时先剥掉。
const NICK_PREFIXES: [&str; 4] = ["老", "小", "大", "阿"];

/// 一个称呼排在字母索引的哪一格。认不出来的进 `#`。
///
/// 只有索引用它。搜索不走这条路 —— 搜索是子串匹配，不该被索引的近似猜测影响。
pub fn alpha_key(label: &str) -> char {
    let t = label.trim();
    if t.is_empty() {
        return '#';
    }
    // 拉丁字母的称呼（Anna、Bob）直接取首字母。
    if let Some(c) = t.chars().next() {
        if c.is_ascii_alphabetic() {
            return c.to_ascii_uppercase();
        }
    }
    // 中文称呼：剥掉「老 / 小 / 阿」之类的叫法，再查姓氏表。
    let mut body = t;
    for pfx in NICK_PREFIXES {
        if body.len() > pfx.len() && body.starts_with(pfx) {
            body = &body[pfx.len()..];
            break;
        }
    }
    for (name, init) in SURNAME_INITIALS {
        if body.starts_with(name) {
            return *init;
        }
    }
    '#'
}

/// 相对今天的短日期：「今天」「昨天」「9 月 24 日」。列表和时间线用。
pub fn rel_day(day: i64, today: i64) -> String {
    match today - day {
        0 => "今天".into(),
        1 => "昨天".into(),
        -1 => "明天".into(),
        _ => {
            let (_, m, d) = days_to_civil(day);
            format!("{} 月 {} 日", m, d)
        }
    }
}

// ---- 规则常量（04-rules.md）----

/// 每份礼物的解谜机会。
pub const MAX_ATTEMPTS: u8 = 3;
/// 契约期限：收下当天 + 7 天。
pub const PACT_DAYS: i64 = 7;
/// 收到的礼物 7 天内没揭晓就退回送礼人。
pub const EXPIRE_DAYS: i64 = 7;
/// 存档格式版本。版本不对的存档直接不读，换成演示数据。
pub const STATE_VERSION: u32 = 4;
/// 折现手续费率（%）。
pub const CASHOUT_FEE_PCT: i64 = 8;
/// 换购手续费率（%）。
pub const EXCHANGE_FEE_PCT: i64 = 5;
/// 手续费下限：¥1。
pub const MIN_FEE: i64 = 100;
pub const PACT_MAX_CHARS: usize = 24;
pub const MESSAGE_MAX_CHARS: usize = 40;
pub const CLUE_MAX_CHARS: usize = 30;
pub const ANSWER_MAX_CHARS: usize = 20;
pub const NICKNAME_MAX_CHARS: usize = 16;
/// 新人礼金（演示初始流水）。
pub const WELCOME_BONUS: i64 = 2000;
/// 「演示充值」一次加多少。
pub const TOP_UP_AMOUNT: i64 = 5000;
/// 收到的礼物离过期 ≤ 2 天时提醒。
pub const GIFT_NOTICE_LEAD_DAYS: i64 = 2;
/// 我答应的契约离到期 ≤ 1 天时提醒。
pub const PACT_NOTICE_LEAD_DAYS: i64 = 1;
/// 猜我是谁的候选人芯片个数。
pub const CANDIDATE_COUNT: usize = 6;
/// 契约禁用词：契约只写轻约定，不涉及钱。
pub const BANNED_WORDS: [&str; 7] = ["转账", "借钱", "红包", "还钱", "现金", "打钱", "贷款"];
/// 预设契约：（芯片上的短名, 契约全文）。
pub const PACT_PRESETS: [(&str, &str); 4] = [
    ("回请咖啡", "下周找时间回请我喝一杯咖啡"),
    ("晒一晒", "收下要发一条朋友圈晒一晒"),
    ("陪看电影", "周末陪我看一场电影"),
    ("见面拥抱", "下次见面先给我一个拥抱"),
];
/// 身份保密时对送礼人的称呼。
pub const MYSTERY_FRIEND: &str = "一位神秘的朋友";
/// 未揭晓的礼物在列表里的名字。
pub const MYSTERY_GIFT: &str = "一份神秘礼物";
/// 模拟器生成的回礼寄语。
pub const RETURN_MESSAGE: &str = "收到你的礼物啦，礼尚往来";
/// 默认称呼（猜我是谁时对方要猜的名字）。
pub const DEFAULT_NICKNAME: &str = "阿岚";

// ---- 礼物目录 ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Coffee,
    Movie,
    Trendy,
    Blind,
    Sweet,
    Digital,
    Home,
    Baby,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Category::Coffee,
        Category::Movie,
        Category::Trendy,
        Category::Blind,
        Category::Sweet,
        Category::Digital,
        Category::Home,
        Category::Baby,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Coffee => "咖啡茶饮",
            Category::Movie => "电影演出",
            Category::Trendy => "潮流小物",
            Category::Blind => "盲盒",
            Category::Sweet => "甜点鲜花",
            Category::Digital => "数码家电",
            Category::Home => "家居生活",
            Category::Baby => "母婴亲子",
        }
    }
}

/// 目录里的一件礼物。目录是代码常量，不落盘；下标就是 `Gift::item`，也是商品图
/// `resources/products/pNN.png` 的编号 —— 只追加不重排。
#[derive(Clone, Copy, Debug)]
pub struct CatalogItem {
    pub name: &'static str,
    /// 演示用的虚构品牌（前 12 件是常见的连锁品牌券）。
    pub brand: &'static str,
    /// 品类里的「是什么」：电视、耳机、咖啡…… 心愿单里「说个大概」就按它匹配。
    pub kind: &'static str,
    pub cat: Category,
    /// 实物要填收件地址；电子券收下即给券码。
    pub physical: bool,
    pub spec: &'static str,
    /// 分。
    pub price: i64,
    /// 规格标签：尺寸、卖点。「说个大概」里写的要求拿它来比。
    pub tags: &'static [&'static str],
    pub desc: &'static str,
}

#[allow(clippy::too_many_arguments)]
const fn ci(
    name: &'static str,
    brand: &'static str,
    kind: &'static str,
    cat: Category,
    physical: bool,
    spec: &'static str,
    yuan: i64,
    tags: &'static [&'static str],
    desc: &'static str,
) -> CatalogItem {
    CatalogItem { name, brand, kind, cat, physical, spec, price: yuan * 100, tags, desc }
}

use Category::{Baby, Blind, Coffee, Digital, Home, Movie, Sweet, Trendy};

pub const CATALOG: [CatalogItem; 33] = [
    ci("三顿半精品咖啡礼盒", "三顿半", "咖啡", Coffee, true, "24 颗装 · 包邮", 109,
        &["冷萃", "速溶", "礼盒"], "超即溶的精品咖啡，冷水牛奶都能三秒化开。24 颗 6 种风味，一个月的早晨都有着落。"),
    ci("星巴克中杯拿铁电子券", "星巴克", "咖啡", Coffee, false, "全国门店通用 · 30 天有效", 35,
        &["电子券", "门店"], "一杯中杯拿铁，想喝的时候去门店出示券码就行。"),
    ci("喜茶多肉葡萄兑换券", "喜茶", "奶茶", Coffee, false, "全国门店通用 · 30 天有效", 29,
        &["电子券", "水果茶"], "招牌多肉葡萄，一整杯的果肉。门店或小程序都能兑。"),
    ci("电影通兑票", "猫眼", "电影票", Movie, false, "2D 场次通兑 · 60 天有效", 49,
        &["电子券", "2D"], "全国大部分影院 2D 场次通兑，挑一部想看的片子就好。"),
    ci("电影双人套票", "猫眼", "电影票", Movie, false, "两张通兑票 + 爆米花套餐", 98,
        &["电子券", "双人", "爆米花"], "两张票加一份爆米花套餐 —— 送了这个，下次见面就有了理由。"),
    ci("帆布托特包", "野帆", "包", Trendy, true, "米白 · 加厚帆布 · 包邮", 79,
        &["米白", "大容量", "帆布"], "16 安加厚帆布，装得下电脑和一天的零碎。越用越软。"),
    ci("香薰蜡烛", "栖木", "香薰", Trendy, true, "无花果香 · 200g · 包邮", 88,
        &["无花果", "助眠", "200g"], "无花果叶和一点木质调，点燃 40 小时。睡前点一会儿，房间是暖的。"),
    ci("拍立得相纸", "即影", "相纸", Trendy, true, "mini 白边 · 40 张 · 包邮", 59,
        &["mini", "40张"], "mini 规格白边相纸 40 张，适配常见的拍立得相机。"),
    ci("潮玩盲盒", "泡泡岛", "盲盒", Blind, true, "随机一款 · 有隐藏款 · 包邮", 69,
        &["潮玩", "隐藏款", "摆件"], "12 款常规加 1 款隐藏，拆之前谁也不知道是哪一只。"),
    ci("文具盲盒", "纸上", "盲盒", Blind, true, "6 件随机 · 包邮", 39,
        &["文具", "手帐"], "胶带、便签、印章、钢笔…… 随机 6 件，做手帐的人会喜欢。"),
    ci("小蛋糕兑换券", "好利来", "蛋糕", Sweet, false, "6 寸 · 指定门店自提", 128,
        &["电子券", "6寸", "生日"], "6 寸鲜奶蛋糕，提前一天在小程序预约，门店自提。"),
    ci("向日葵花束", "花点时间", "鲜花", Sweet, true, "3 枝装 · 同城配送", 99,
        &["向日葵", "同城"], "三枝向日葵配尤加利叶，同城当天配送。"),
    ci("澄光 Q5 55 英寸 4K 电视", "澄光", "电视", Digital, true, "55 英寸 · 4K · 包邮包安装", 2699,
        &["55寸", "4K", "护眼", "客厅"], "55 英寸 4K 屏，低蓝光护眼模式。开机无广告，老人小孩都会用。"),
    ci("澄光 Q7 65 英寸 4K 电视", "澄光", "电视", Digital, true, "65 英寸 · 4K 120Hz · 包邮包安装", 3999,
        &["65寸", "4K", "120Hz", "游戏", "客厅"], "65 英寸 4K 120Hz 高刷，接游戏机不拖影。客厅一面墙刚刚好。"),
    ci("声屿 Mini 43 英寸电视", "声屿", "电视", Digital, true, "43 英寸 · 全高清 · 包邮", 1299,
        &["43寸", "全高清", "卧室"], "43 英寸全高清，放卧室或出租屋正合适。自带音箱不闷。"),
    ci("澄光 Q9 75 英寸 Mini LED 电视", "澄光", "电视", Digital, true, "75 英寸 · Mini LED · 包邮包安装", 6999,
        &["75寸", "4K", "MiniLED", "120Hz", "客厅"], "75 英寸 Mini LED，暗场景也看得清。给新家客厅的一份大礼。"),
    ci("声屿 Pebble 蓝牙音箱", "声屿", "音箱", Digital, true, "蓝牙 5.3 · IP67 防水 · 包邮", 399,
        &["蓝牙", "便携", "防水"], "鹅卵石大小，扔进包里就走。防水，浴室和野餐都能放。"),
    ci("声屿 Air 降噪耳机", "声屿", "耳机", Digital, true, "头戴式 · 主动降噪 · 包邮", 899,
        &["降噪", "蓝牙", "头戴", "长续航"], "主动降噪，地铁上也能听清轻音乐。一次充电用 40 小时。"),
    ci("光语 便携投影仪", "光语", "投影仪", Digital, true, "1080P · 自动对焦 · 包邮", 1999,
        &["1080P", "便携", "卧室"], "往白墙一照就是 100 寸。自动对焦，躺在床上看电影。"),
    ci("暖物 可视空气炸锅 5L", "暖物", "空气炸锅", Home, true, "5L · 可视窗 · 包邮", 329,
        &["5L", "可视", "大容量"], "透明可视窗，炸到几分熟一眼就知道。5L 够三四个人吃。"),
    ci("暖物 保温电热水壶", "暖物", "水壶", Home, true, "1.7L · 恒温 · 包邮", 159,
        &["1.7L", "恒温", "泡茶"], "五档恒温，泡茶冲奶都合适。304 不锈钢内胆。"),
    ci("栖木 全棉四件套", "栖木", "床品", Home, true, "1.8m 床 · 60 支全棉 · 包邮", 459,
        &["1.8m", "全棉", "奶油色"], "60 支长绒棉，奶油色，越洗越软。搬新家换一套新床品。"),
    ci("栖木 陶瓷餐具礼盒", "栖木", "餐具", Home, true, "8 件套 · 可进洗碗机 · 包邮", 269,
        &["陶瓷", "8件", "洗碗机"], "两人份的碗盘杯 8 件，哑光釉面，可进洗碗机和微波炉。"),
    ci("栖木 氛围落地灯", "栖木", "灯", Home, true, "暖光 · 三档调光 · 包邮", 239,
        &["暖光", "调光", "卧室"], "暖白两色、三档亮度。沙发边放一盏，晚上就不想开大灯了。"),
    ci("琴叶榕盆栽", "青田", "绿植", Home, true, "80cm 高 · 含盆 · 同城配送", 139,
        &["大盆", "好养", "客厅"], "80 厘米的琴叶榕，叶子大、好养活。放新家客厅一角。"),
    ci("多肉组合盆栽", "青田", "绿植", Home, true, "6 株 · 陶盆 · 包邮", 59,
        &["小盆", "好养", "桌面"], "六株多肉拼在一个陶盆里，两周浇一次水就行。"),
    ci("小橡 轻便婴儿推车", "小橡", "推车", Baby, true, "可登机 · 5.8kg · 包邮", 1299,
        &["轻便", "可登机", "可躺"], "一只手就能收起来，5.8 公斤，能带上飞机。"),
    ci("小橡 新生儿礼盒", "小橡", "母婴礼盒", Baby, true, "0–6 个月 · 纯棉 9 件 · 包邮", 369,
        &["纯棉", "0-6月", "满月"], "纯棉和尚服、口水巾、小袜子 9 件，满月礼刚刚好。"),
    ci("木作益智积木", "小橡", "玩具", Baby, true, "100 块 · 3 岁以上 · 包邮", 199,
        &["3岁+", "益智", "木质"], "100 块榉木积木，水性漆，棱角都磨圆了。"),
    ci("随行保温杯", "野帆", "杯子", Trendy, true, "480ml · 12 小时保温 · 包邮", 129,
        &["480ml", "保温", "通勤"], "一只手能开盖，放包里不漏。热水放到下午还烫。"),
    ci("暖物 扫拖机器人", "暖物", "扫地机", Home, true, "扫拖一体 · 自动集尘 · 包邮", 1899,
        &["扫拖一体", "自动集尘", "新家"], "扫拖一体，自动倒尘。搬进新家，地板交给它。"),
    ci("声屿 S6 55 英寸 QLED 电视", "声屿", "电视", Digital, true, "55 英寸 · QLED · 包邮包安装", 2499,
        &["55寸", "4K", "QLED", "卧室"], "55 英寸量子点屏，颜色比普通 4K 更鲜亮。窄边框，挂在卧室不占地方。"),
    ci("澄光 Q6 65 英寸 4K 电视", "澄光", "电视", Digital, true, "65 英寸 · 4K · 包邮包安装", 2999,
        &["65寸", "4K", "护眼", "客厅"], "65 英寸 4K，三千块以内能买到的大屏。护眼模式、开机无广告。"),
];

/// 按下标取目录项；越界（旧存档、坏数据）退回第一件，不 panic。
pub fn item(i: u16) -> &'static CatalogItem {
    CATALOG.get(i as usize).unwrap_or(&CATALOG[0])
}

/// 某个品类下的目录下标；`None` = 全部。
pub fn catalog_in(cat: Option<Category>) -> Vec<u16> {
    (0..CATALOG.len() as u16)
        .filter(|&i| cat.map_or(true, |c| item(i).cat == c))
        .collect()
}

/// 同品类的其它几件（商品详情页底部「同类还有」），价格近的在前。
pub fn related_items(i: u16, n: usize) -> Vec<u16> {
    let it = item(i);
    let mut v: Vec<u16> = (0..CATALOG.len() as u16)
        .filter(|&j| j != i && item(j).cat == it.cat)
        .collect();
    v.sort_by_key(|&j| ((item(j).price - it.price).abs(), j));
    v.truncate(n);
    v
}

/// 不超过 `budget` 的最贵一件（同价取靠前的）。回礼、AI 建议都用它。
pub fn best_item_within(budget: i64) -> Option<u16> {
    let mut best: Option<u16> = None;
    for i in 0..CATALOG.len() as u16 {
        let p = item(i).price;
        if p <= budget && best.map_or(true, |b| p > item(b).price) {
            best = Some(i);
        }
    }
    best
}

// ---- 金额 ----

/// 分 → 「¥109」/「¥8.72」/「-¥6」。
pub fn yuan(cents: i64) -> String {
    let a = cents.abs();
    let s = if a % 100 == 0 {
        format!("¥{}", a / 100)
    } else {
        format!("¥{}.{:02}", a / 100, a % 100)
    };
    if cents < 0 {
        format!("-{s}")
    } else {
        s
    }
}

/// 手续费：`max(¥1, 向上取整到元(price × pct / 100))`。
pub fn fee(price: i64, pct: i64) -> i64 {
    let cents = (price * pct + 99) / 100;
    let whole = (cents + 99) / 100 * 100;
    whole.max(MIN_FEE)
}

/// 折现报价：（手续费, 退回余额）。
pub fn cashout_quote(price: i64) -> (i64, i64) {
    let f = fee(price, CASHOUT_FEE_PCT);
    (f, (price - f).max(0))
}

/// 换购抵扣：（手续费, 抵扣额）。
pub fn exchange_credit(price: i64) -> (i64, i64) {
    let f = fee(price, EXCHANGE_FEE_PCT);
    (f, (price - f).max(0))
}

// ---- 解谜 ----

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Unlock {
    #[default]
    GuessWho,
    Question,
    Passphrase,
    Free,
}

impl Unlock {
    pub const ALL: [Unlock; 4] = [Unlock::GuessWho, Unlock::Question, Unlock::Passphrase, Unlock::Free];

    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Unlock::Question,
            2 => Unlock::Passphrase,
            3 => Unlock::Free,
            _ => Unlock::GuessWho,
        }
    }

    pub fn id(self) -> u8 {
        self as u8
    }

    /// 送礼页上的全称。
    pub fn label(self) -> &'static str {
        match self {
            Unlock::GuessWho => "猜我是谁",
            Unlock::Question => "专属私密问答",
            Unlock::Passphrase => "专属暗号",
            Unlock::Free => "无条件直接领取",
        }
    }

    /// 徽章 / 分段上的短名。
    pub fn short(self) -> &'static str {
        match self {
            Unlock::GuessWho => "猜我是谁",
            Unlock::Question => "私密问答",
            Unlock::Passphrase => "专属暗号",
            Unlock::Free => "直接领取",
        }
    }

    /// 解密页上那张卡片的标题。
    pub fn clue_title(self) -> &'static str {
        match self {
            Unlock::GuessWho => "TA 留下的线索",
            Unlock::Question => "TA 的问题",
            Unlock::Passphrase => "暗号提示",
            Unlock::Free => "",
        }
    }
}

fn to_halfwidth(c: char) -> char {
    match c as u32 {
        0xFF01..=0xFF5E => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
        0x3000 => ' ',
        _ => c,
    }
}

fn is_punct(c: char) -> bool {
    c.is_ascii_punctuation()
        || matches!(c as u32, 0x2010..=0x206F | 0x3000..=0x303F | 0xFE30..=0xFE4F)
        || "·～￥…".contains(c)
}

/// 答案规范化：全角转半角 → 去掉所有空白与标点 → 转小写。
pub fn normalize_answer(s: &str) -> String {
    s.chars()
        .map(to_halfwidth)
        .filter(|&c| !c.is_whitespace() && !is_punct(c))
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// 称呼别名：「林舟 / 舟舟、阿舟」→ [林舟, 舟舟, 阿舟]。
pub fn split_aliases(s: &str) -> Vec<String> {
    s.split(['/', '、', ',', '，'])
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect()
}

/// 规范化后比较；`aliases` 为真时命中任一别名即算对。空答案永远不对。
pub fn answer_matches(expected: &str, guess: &str, aliases: bool) -> bool {
    let g = normalize_answer(guess);
    if g.is_empty() {
        return false;
    }
    if aliases {
        split_aliases(expected).iter().any(|a| normalize_answer(a) == g)
    } else {
        normalize_answer(expected) == g
    }
}

// ---- 契约 / 文本校验 ----

/// 契约：去首尾空白后 1–24 字，不含禁用词。返回整理后的文本。
pub fn validate_pact(text: &str) -> Result<String, &'static str> {
    let t = text.trim();
    if t.is_empty() {
        return Err("契约还没写");
    }
    if t.chars().count() > PACT_MAX_CHARS {
        return Err("契约最多 24 个字");
    }
    if BANNED_WORDS.iter().any(|w| t.contains(w)) {
        return Err("契约只写轻约定，不涉及钱");
    }
    Ok(t.to_string())
}

/// 手机号：11 位数字、1 开头（演示只做这一层）。
pub fn valid_phone(s: &str) -> bool {
    let s = s.trim();
    s.len() == 11 && s.starts_with('1') && s.bytes().all(|b| b.is_ascii_digit())
}

// ---- 券码 ----

/// 券码字符集：去掉 0 / O / 1 / I，免得抄错。
pub const CODE_CHARS: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn code_chars(mut v: u64, n: usize) -> String {
    let mut s = String::new();
    for _ in 0..n {
        s.push(CODE_CHARS[(v % CODE_CHARS.len() as u64) as usize] as char);
        v /= CODE_CHARS.len() as u64;
    }
    s
}

/// 电子券券码：`XXXX-XXXX`。
pub fn gen_voucher(seed: u64) -> String {
    let v = mix(seed ^ 0x5EED);
    format!("{}-{}", code_chars(v, 4), code_chars(v >> 24, 4))
}

/// 礼卡链接（演示）：按礼物 id 打开。
pub fn gift_link(id: u64) -> String {
    format!("liyu://g/{id}")
}

/// 心愿单链接（演示）。
pub fn wish_link(id: u64) -> String {
    format!("liyu://w/{id}")
}

// ---- 礼物 ----

pub const DIR_SENT: u8 = 0;
pub const DIR_RECEIVED: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GiftState {
    Sealed,
    Opened,
    Revealed,
    Accepted,
    Exchanged,
    CashedOut,
    Expired,
    Withdrawn,
}

impl GiftState {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => GiftState::Opened,
            2 => GiftState::Revealed,
            3 => GiftState::Accepted,
            4 => GiftState::Exchanged,
            5 => GiftState::CashedOut,
            6 => GiftState::Expired,
            7 => GiftState::Withdrawn,
            _ => GiftState::Sealed,
        }
    }

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn is_terminal(self) -> bool {
        self.id() >= GiftState::Accepted.id()
    }

    /// 还没揭晓（会过期的那两态）。
    pub fn is_unrevealed(self) -> bool {
        matches!(self, GiftState::Sealed | GiftState::Opened)
    }
}

/// 状态颜色：进行中蓝、完成绿、退回灰。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Pending,
    Done,
    Returned,
}

#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct Gift {
    pub id: u64,
    pub dir: u8,
    pub item: u16,
    /// 下单时价格（分）。
    pub price: i64,
    /// 送出：收礼人备注（可空）；收到：送礼人称呼（别名用 `/` 分隔）。
    pub peer: String,
    pub unlock: u8,
    /// 猜我是谁：线索；私密问答：问题；专属暗号：提示。
    pub clue: String,
    /// 私密问答 / 专属暗号的答案；猜我是谁：送出时是我的称呼，收到时留空（用 peer）。
    pub answer: String,
    /// 空 = 无契约。
    pub contract: String,
    pub message: String,
    pub sent_on: i64,
    pub attempts: u8,
    pub state: u8,
    pub identity_known: bool,
    /// 是答对（或直接领取）拆开的，而不是机会用完。时间线靠它分两种说法；
    /// 身份保密后因同意契约又揭晓时，`identity_known` 会变、它不变。
    pub solved: bool,
    pub opened_on: i64,
    pub revealed_on: i64,
    pub settled_on: i64,
    /// 换购后的目录下标（`u16::MAX` = 无）。
    pub swap_item: u16,
    /// 折现 / 换购退回余额的金额（分）。
    pub refund: i64,
    pub voucher: String,
    pub ship_name: String,
    pub ship_phone: String,
    pub ship_addr: String,
    pub demo_tip: String,
    /// 从哪张心愿单认领的（`Wishlist::id`）。旧存档没有这个字段，所以是 Option。
    pub wish_id: Option<u64>,
    /// 预约送达的礼物：下单那天。这时 `sent_on` 是约好送到的那天（心愿单的日子），
    /// 到那天之前收礼人看不到它。
    pub booked_on: Option<i64>,
}

impl Gift {
    fn blank(id: u64, dir: u8, item_id: u16, sent_on: i64) -> Self {
        Gift {
            id,
            dir,
            item: item_id,
            price: item(item_id).price,
            peer: String::new(),
            unlock: Unlock::Free.id(),
            clue: String::new(),
            answer: String::new(),
            contract: String::new(),
            message: String::new(),
            sent_on,
            attempts: 0,
            state: GiftState::Sealed.id(),
            identity_known: false,
            solved: false,
            opened_on: 0,
            revealed_on: 0,
            settled_on: 0,
            swap_item: u16::MAX,
            refund: 0,
            voucher: String::new(),
            ship_name: String::new(),
            ship_phone: String::new(),
            ship_addr: String::new(),
            demo_tip: String::new(),
            wish_id: None,
            booked_on: None,
        }
    }

    /// 预约了还没到日子（收礼人那边还看不到）。
    pub fn is_scheduled(&self, today: i64) -> bool {
        self.sent_on > today
    }

    pub fn state(&self) -> GiftState {
        GiftState::from_u8(self.state)
    }

    fn set_state(&mut self, s: GiftState) {
        self.state = s.id();
    }

    pub fn unlock(&self) -> Unlock {
        Unlock::from_u8(self.unlock)
    }

    pub fn is_sent(&self) -> bool {
        self.dir == DIR_SENT
    }

    pub fn catalog(&self) -> &'static CatalogItem {
        item(self.item)
    }

    /// 最后到手的那件（换购过就是新的那件）。
    pub fn final_item(&self) -> &'static CatalogItem {
        if self.swap_item != u16::MAX {
            item(self.swap_item)
        } else {
            self.catalog()
        }
    }

    pub fn has_contract(&self) -> bool {
        !self.contract.is_empty()
    }

    /// 称呼的第一个别名（送出时是备注）。
    pub fn peer_name(&self) -> String {
        split_aliases(&self.peer).into_iter().next().unwrap_or_default()
    }

    /// 收到的礼物：此刻能给我看的送礼人名字。没揭晓、或揭晓了但身份保密，都是「一位神秘的朋友」。
    pub fn shown_sender(&self) -> String {
        let revealed = !self.state().is_unrevealed() && self.revealed_on > 0;
        if revealed && self.identity_known {
            let n = self.peer_name();
            if !n.is_empty() {
                return n;
            }
        }
        MYSTERY_FRIEND.to_string()
    }

    /// 送出的礼物：收礼人怎么称呼（没写备注就是「TA」）。
    pub fn shown_recipient(&self) -> String {
        let n = self.peer_name();
        if n.is_empty() {
            "TA".into()
        } else {
            n
        }
    }

    /// 列表上的名字：收到的、还没揭晓的只说「一份神秘礼物」。
    pub fn title(&self) -> String {
        if !self.is_sent() && self.revealed_on == 0 {
            return MYSTERY_GIFT.to_string();
        }
        self.final_item().name.to_string()
    }

    /// 解谜要比对的答案（猜我是谁 = 称呼别名）。
    pub fn expected_answer(&self) -> &str {
        if self.unlock() == Unlock::GuessWho && self.answer.is_empty() {
            &self.peer
        } else {
            &self.answer
        }
    }

    pub fn attempts_left(&self) -> u8 {
        MAX_ATTEMPTS.saturating_sub(self.attempts)
    }

    /// 离过期还有几天（只对未揭晓的有意义）。
    pub fn days_left(&self, today: i64) -> i64 {
        self.sent_on + EXPIRE_DAYS - today
    }

    pub fn tone(&self) -> Tone {
        match self.state() {
            GiftState::Sealed | GiftState::Opened | GiftState::Revealed => Tone::Pending,
            GiftState::Accepted | GiftState::Exchanged | GiftState::CashedOut => Tone::Done,
            GiftState::Expired | GiftState::Withdrawn => Tone::Returned,
        }
    }

    /// 状态文案（02-flows 3 节那张表）。
    pub fn status_text(&self, today: i64) -> String {
        let st = self.state();
        if self.is_sent() {
            match st {
                GiftState::Sealed if self.is_scheduled(today) => {
                    format!("待送达 · {}送到", rel_day(self.sent_on, today))
                }
                GiftState::Sealed => "待拆 · TA 还没打开".into(),
                GiftState::Opened => format!("解谜中 · 猜错 {} 次", self.attempts),
                GiftState::Revealed => "已揭晓 · 等 TA 决定".into(),
                GiftState::Accepted => "TA 收下了".into(),
                GiftState::Exchanged => "TA 换了一份更喜欢的".into(),
                GiftState::CashedOut => "TA 折成了余额".into(),
                GiftState::Expired => "已过期 · 已全额退回".into(),
                GiftState::Withdrawn => "已撤回 · 已全额退回".into(),
            }
        } else {
            match st {
                GiftState::Sealed => format!("待拆 · 还剩 {} 天", self.days_left(today).max(0)),
                GiftState::Opened => format!("解谜中 · 剩 {} 次机会", self.attempts_left()),
                GiftState::Revealed => "待你决定".into(),
                GiftState::Accepted => "已收下".into(),
                GiftState::Exchanged => format!("已换成{}", self.final_item().name),
                GiftState::CashedOut => format!("已折现 {}", yuan(self.refund)),
                GiftState::Expired => "已过期，已退回给 TA".into(),
                GiftState::Withdrawn => "TA 撤回了这份礼物".into(),
            }
        }
    }

    /// 送出详情的时间线（最多 5 行，旧的在前）。预约送达的礼物先记下单，
    /// 到了日子再记「送到」；没到日子的那一步交给界面画成空心的「接下来」。
    pub fn timeline(&self, today: i64) -> Vec<(i64, String)> {
        let mut t = Vec::new();
        if let Some(b) = self.booked_on {
            t.push((b, format!("下单 · 约好 {} 送到", rel_day(self.sent_on, b))));
            if !self.is_scheduled(today) {
                t.push((self.sent_on, "礼卡送到 TA 手里".into()));
            }
        } else {
            t.push((self.sent_on, "送出礼卡".into()));
        }
        if self.opened_on > 0 && self.unlock() != Unlock::Free {
            t.push((self.opened_on, "TA 打开了礼卡".into()));
        }
        if self.revealed_on > 0 {
            let s = if self.unlock() == Unlock::Free {
                "TA 领取了礼物".to_string()
            } else if self.solved {
                format!("TA 答对了（用了 {} 次机会）", self.attempts.max(1))
            } else {
                "机会用完，礼物拆开了（没透露你）".to_string()
            };
            t.push((self.revealed_on, s));
        }
        if self.settled_on > 0 {
            let s = match self.state() {
                GiftState::Accepted if self.has_contract() => "TA 收下了，契约生效".to_string(),
                GiftState::Accepted => "TA 收下了".to_string(),
                GiftState::Exchanged => format!("TA 换成了{}", self.final_item().name),
                GiftState::CashedOut => "TA 折成了余额".to_string(),
                GiftState::Expired => format!("7 天没拆开，已全额退回 {}", yuan(self.price)),
                GiftState::Withdrawn => format!("你撤回了礼物，已全额退回 {}", yuan(self.price)),
                _ => String::new(),
            };
            if !s.is_empty() {
                t.push((self.settled_on, s));
            }
        }
        t
    }
}

// ---- 契约 ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PactState {
    Pending,
    Done,
    Waived,
}

impl PactState {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => PactState::Done,
            2 => PactState::Waived,
            _ => PactState::Pending,
        }
    }
}

#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct Pact {
    pub id: u64,
    pub gift_id: u64,
    pub text: String,
    pub peer: String,
    /// true = 我答应的（我是收礼人）；false = 答应我的。
    pub mine: bool,
    pub made_on: i64,
    pub due_on: i64,
    pub state: u8,
    /// 最近一次「提醒 TA」的日期（0 = 从没提醒过）。
    pub nudged_on: i64,
}

impl Pact {
    pub fn state(&self) -> PactState {
        PactState::from_u8(self.state)
    }

    /// 「还有 3 天」「今天到期」「已逾期 2 天」「已兑现」「已免除」。
    pub fn due_text(&self, today: i64) -> String {
        match self.state() {
            PactState::Done => "已兑现".into(),
            PactState::Waived => "已免除".into(),
            PactState::Pending => {
                let d = self.due_on - today;
                if d > 0 {
                    format!("还有 {d} 天")
                } else if d == 0 {
                    "今天到期".into()
                } else {
                    format!("已逾期 {} 天", -d)
                }
            }
        }
    }
}

// ---- 流水 ----

#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct LedgerEntry {
    pub id: u64,
    pub day: i64,
    /// 对余额的影响（分，带符号）。
    pub amount: i64,
    /// 这笔里模拟支付的部分（分，≥ 0，不影响余额）。
    pub external: i64,
    pub title: String,
    pub note: String,
}

// ---- 心愿单 ----

/// 一张心愿单最多几件。
pub const WISH_MAX_ITEMS: usize = 8;
pub const WISH_TITLE_MAX_CHARS: usize = 16;
pub const WISH_NOTE_MAX_CHARS: usize = 40;
/// 「说个大概」里的要求（如「55 寸以上 · 4K」）。
pub const WISH_WANTS_MAX_CHARS: usize = 20;
/// 最多提前多久发布。
pub const WISH_MAX_AHEAD_DAYS: i64 = 180;
/// 日子过了之后还开放几天（补送的人还来得及）。
pub const WISH_GRACE_DAYS: i64 = 7;
/// 好友的日子前几天提醒我。
pub const WISH_NOTICE_LEAD_DAYS: i64 = 3;
/// 场合。下标存在 `Wishlist::occasion` 里 —— 只追加不重排。
pub const OCCASIONS: [&str; 6] = ["生日", "结婚", "乔迁", "毕业", "宝宝", "其他"];
/// 「说个大概」能选的东西，每个都对得上目录里的 `CatalogItem::kind`。
pub const WISH_KINDS: [&str; 16] = [
    "电视", "耳机", "音箱", "投影仪", "空气炸锅", "扫地机", "床品", "餐具",
    "灯", "绿植", "鲜花", "蛋糕", "咖啡", "盲盒", "玩具", "推车",
];
/// 「说个大概」的预算档（元）。0 = 不限。
pub const WISH_BUDGETS: [i64; 7] = [0, 100, 300, 500, 1000, 3000, 5000];
/// 结算页的付款方式（全是模拟的）。
pub const PAY_METHODS: [&str; 3] = ["微信支付", "支付宝", "银行卡"];
/// 换一份时给几个选择（价格离抵扣额近的在前）。
pub const EXCHANGE_CHOICES: usize = 11;

/// 「一台电视」「一副耳机」—— 含糊的心愿用量词说出来才像人话。
pub fn measure_word(kind: &str) -> &'static str {
    match kind {
        "电视" | "投影仪" | "空气炸锅" | "扫地机" | "音箱" => "台",
        "耳机" => "副",
        "床品" | "餐具" => "套",
        "灯" => "盏",
        "绿植" => "盆",
        "鲜花" => "束",
        "咖啡" => "份",
        "推车" => "辆",
        _ => "个",
    }
}

/// 预算档的说法。
pub fn budget_text(max_price: i64) -> String {
    if max_price <= 0 {
        "预算不限".into()
    } else {
        format!("{} 以内", yuan(max_price))
    }
}

/// 心愿单上的一件。要么是目录里具体的一件（`item`），要么只说个大概：
/// 品类 + 预算 + 一句要求，送礼的人从匹配的商品里挑。
#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct WishItem {
    /// 具体的那件（目录下标）；`None` = 说个大概。
    pub item: Option<u16>,
    /// 是什么（`CatalogItem::kind`）。具体的也存一份，方便按品类匹配。
    pub kind: String,
    /// 预算上限（分），0 = 不限。只对「说个大概」有意义。
    pub max_price: i64,
    /// 要求，如「55 寸以上」「降噪」。
    pub wants: String,
    /// 0 = 还没人送；1 = 别人认领了；2 = 我认领了。
    pub state: u8,
    /// 认领它的那份礼物（我送的，或者我心愿单上别人送我的）。
    pub gift_id: u64,
    pub claimed_on: i64,
    /// 实际送的是哪件（`u16::MAX` = 不知道）。
    pub given_item: u16,
}

pub const WISH_OPEN: u8 = 0;
pub const WISH_BY_OTHERS: u8 = 1;
pub const WISH_BY_ME: u8 = 2;

impl WishItem {
    pub fn exact(i: u16) -> Self {
        WishItem {
            item: Some(i),
            kind: item(i).kind.to_string(),
            max_price: 0,
            wants: String::new(),
            state: WISH_OPEN,
            gift_id: 0,
            claimed_on: 0,
            given_item: u16::MAX,
        }
    }

    pub fn vague(kind: &str, max_price: i64, wants: &str) -> Self {
        WishItem {
            item: None,
            kind: kind.to_string(),
            max_price: max_price.max(0),
            wants: wants.trim().to_string(),
            state: WISH_OPEN,
            gift_id: 0,
            claimed_on: 0,
            given_item: u16::MAX,
        }
    }

    pub fn is_exact(&self) -> bool {
        self.item.is_some()
    }

    pub fn is_open(&self) -> bool {
        self.state == WISH_OPEN
    }

    /// 列表上的名字：具体的是商品名，含糊的是「一台电视」。
    pub fn title(&self) -> String {
        match self.item {
            Some(i) => item(i).name.to_string(),
            None => format!("一{}{}", measure_word(&self.kind), self.kind),
        }
    }

    /// 第二行：具体的是规格 + 价格，含糊的是预算 + 要求。
    pub fn sub(&self) -> String {
        match self.item {
            Some(i) => format!("{} · {}", item(i).brand, yuan(item(i).price)),
            None if self.wants.is_empty() => format!("说了个大概 · {}", budget_text(self.max_price)),
            None => format!("{} · {}", budget_text(self.max_price), self.wants),
        }
    }

    /// 缩略图用哪件：具体的就是它；含糊的用送出的那件，没有就用同类里最便宜的一件当示意。
    pub fn thumb(&self) -> u16 {
        if let Some(i) = self.item {
            return i;
        }
        if self.given_item != u16::MAX && self.state == WISH_BY_ME {
            return self.given_item;
        }
        (0..CATALOG.len() as u16)
            .filter(|&i| item(i).kind == self.kind)
            .min_by_key(|&i| item(i).price)
            .unwrap_or(0)
    }

    /// 这件商品能不能兑现这条心愿：具体的必须是同一件，含糊的同类就行（预算只是参考）。
    pub fn fits(&self, i: u16) -> bool {
        match self.item {
            Some(x) => x == i,
            None => item(i).kind == self.kind,
        }
    }

    fn release(&mut self) {
        self.state = WISH_OPEN;
        self.gift_id = 0;
        self.claimed_on = 0;
        self.given_item = u16::MAX;
    }
}

/// 一张心愿单。`owner` 为空是我自己的，否则是好友的（演示里的好友心愿单是本机数据）。
#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct Wishlist {
    pub id: u64,
    pub owner: String,
    pub occasion: u8,
    pub title: String,
    pub note: String,
    /// 那个日子。
    pub event_on: i64,
    pub created_on: i64,
    /// 手动结束的那天，0 = 没结束。
    pub closed_on: i64,
    /// 谁能看见；空 = 所有熟人。
    pub audience: Vec<String>,
    pub items: Vec<WishItem>,
}

impl Wishlist {
    pub fn is_mine(&self) -> bool {
        self.owner.is_empty()
    }

    /// 还能认领：没手动结束，而且没过日子 7 天。
    pub fn is_open(&self, today: i64) -> bool {
        self.closed_on == 0 && today <= self.event_on + WISH_GRACE_DAYS
    }

    pub fn occasion_label(&self) -> &'static str {
        OCCASIONS.get(self.occasion as usize).copied().unwrap_or("其他")
    }

    /// 「林舟的生日」；场合是「其他」时只说「林舟的日子」。
    pub fn event_name(&self, me: &str) -> String {
        let who = if self.is_mine() { me } else { &self.owner };
        match self.occasion_label() {
            "其他" => format!("{who}的日子"),
            o => format!("{who}的{o}"),
        }
    }

    pub fn open_count(&self) -> usize {
        self.items.iter().filter(|w| w.is_open()).count()
    }

    pub fn claimed_count(&self) -> usize {
        self.items.len() - self.open_count()
    }

    /// 倒数：还有 3 天 / 明天 / 就是今天 / 已过 2 天 / 已结束。
    pub fn countdown(&self, today: i64) -> String {
        if !self.is_open(today) {
            return "已结束".into();
        }
        match self.event_on - today {
            d if d > 1 => format!("还有 {d} 天"),
            1 => "明天".into(),
            0 => "就是今天".into(),
            d => format!("已过 {} 天", -d),
        }
    }

    /// 「9 月 28 日 · 还有 3 天」。
    pub fn when_text(&self, today: i64) -> String {
        format!("{} · {}", md_cn(self.event_on), self.countdown(today))
    }

    /// 「5 件心愿 · 2 件已有人送」。
    pub fn progress_text(&self) -> String {
        match self.claimed_count() {
            0 => format!("{} 件心愿 · 还没人认领", self.items.len()),
            n if n == self.items.len() => format!("{} 件心愿 · 全都有人送啦", n),
            n => format!("{} 件心愿 · {} 件已有人送", self.items.len(), n),
        }
    }

    pub fn audience_text(&self) -> String {
        if self.audience.is_empty() {
            "所有熟人可见".into()
        } else {
            format!("仅 {} 位熟人可见", self.audience.len())
        }
    }
}

/// 发布 / 编辑心愿单的表单。离开编辑页就丢，不落盘。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WishDraft {
    /// 编辑已有的那张；`None` = 新发布。
    pub id: Option<u64>,
    pub occasion: u8,
    pub title: String,
    pub note: String,
    pub event_on: i64,
    pub audience: Vec<String>,
    pub items: Vec<WishItem>,
}

impl WishDraft {
    pub fn new(today: i64) -> Self {
        WishDraft { event_on: today + 14, ..Default::default() }
    }

    pub fn from_list(w: &Wishlist) -> Self {
        WishDraft {
            id: Some(w.id),
            occasion: w.occasion,
            title: w.title.clone(),
            note: w.note.clone(),
            event_on: w.event_on,
            audience: w.audience.clone(),
            items: w.items.clone(),
        }
    }

    /// 标题没写时用的默认标题：「阿岚的生日心愿单」。
    pub fn default_title(&self, nickname: &str) -> String {
        let me = split_aliases(nickname).into_iter().next().unwrap_or_else(|| "我".into());
        match OCCASIONS.get(self.occasion as usize).copied().unwrap_or("其他") {
            "其他" => format!("{me}的心愿单"),
            o => format!("{me}的{o}心愿单"),
        }
    }

    /// 已经在单子上的具体商品。
    pub fn has_item(&self, i: u16) -> bool {
        self.items.iter().any(|w| w.item == Some(i))
    }
}

/// 含糊心愿的一个候选：哪件、命中了哪些要求、在不在预算内。
#[derive(Clone, Debug, PartialEq)]
pub struct WishMatch {
    pub item: u16,
    pub hits: Vec<String>,
    pub within: bool,
}

impl WishMatch {
    /// 候选卡片上的一句理由：「符合 65寸 · 4K · 预算内」。
    pub fn reason(&self) -> String {
        let mut s = if self.hits.is_empty() {
            String::new()
        } else {
            format!("符合 {} · ", self.hits.join(" · "))
        };
        s.push_str(if self.within { "预算内" } else { "超出预算" });
        s
    }
}

/// 要求里的尺寸下限：「55 寸以上」「65寸」→ 55 / 65，同时返回去掉尺寸后剩下的字。
fn parse_min_size(wants: &str) -> (Option<u32>, String) {
    let chars: Vec<char> = wants.chars().collect();
    let Some(pos) = chars.iter().position(|&c| c == '寸') else {
        return (None, wants.to_string());
    };
    let mut start = pos;
    while start > 0 && chars[start - 1] == ' ' {
        start -= 1;
    }
    let digits_end = start;
    while start > 0 && chars[start - 1].is_ascii_digit() {
        start -= 1;
    }
    let n: Option<u32> = chars[start..digits_end].iter().collect::<String>().parse().ok();
    if n.is_none() {
        return (None, wants.to_string());
    }
    let mut end = pos + 1;
    let tail: String = chars[end..].iter().collect();
    for w in ["以上", "及以上", "起"] {
        if tail.starts_with(w) {
            end += w.chars().count();
            break;
        }
    }
    let rest: String = chars[..start].iter().chain(chars[end..].iter()).collect();
    (n, rest)
}

/// 商品的尺寸标签（「55寸」→ 55）。
fn size_of(i: u16) -> Option<u32> {
    item(i).tags.iter().find_map(|t| t.strip_suffix('寸').and_then(|n| n.parse().ok()))
}

/// 要求里的关键词：按空白和常见分隔符拆开，去掉「要」「最好」这类虚词。
fn want_tokens(rest: &str) -> Vec<String> {
    rest.split(|c: char| c.is_whitespace() || "，,、/;；·+和".contains(c))
        .map(|t| t.trim_start_matches("最好").trim_start_matches('要').trim_end_matches('的').trim())
        .filter(|t| !t.is_empty() && !["以上", "以内", "左右", "都行", "随便"].contains(t))
        .map(|t| t.to_string())
        .collect()
}

/// 给「说个大概」的心愿挑商品：同类 → 尺寸是硬条件 → 预算内的按命中要求多少、再按价格排。
/// 预算内一件都没有时，放宽预算（候选都标「超出预算」）；尺寸也卡死时再放宽尺寸。
/// 具体的心愿只返回那一件。
pub fn wish_candidates(w: &WishItem) -> Vec<WishMatch> {
    if let Some(i) = w.item {
        return vec![WishMatch { item: i, hits: Vec::new(), within: true }];
    }
    let (min_size, rest) = parse_min_size(&w.wants);
    let tokens = want_tokens(&rest);
    let pool: Vec<u16> = (0..CATALOG.len() as u16).filter(|&i| item(i).kind == w.kind).collect();
    let sized: Vec<u16> = match min_size {
        Some(n) => pool.iter().copied().filter(|&i| size_of(i).is_some_and(|s| s >= n)).collect(),
        None => pool.clone(),
    };
    let base = if sized.is_empty() { pool } else { sized };
    let score = |i: u16| -> WishMatch {
        let it = item(i);
        let mut hits = Vec::new();
        if let (Some(_), Some(s)) = (min_size, size_of(i)) {
            hits.push(format!("{s}寸"));
        }
        for t in &tokens {
            let tl = t.to_lowercase();
            let hit = it.tags.iter().any(|g| g.to_lowercase().contains(&tl) || tl.contains(&g.to_lowercase()))
                || it.name.to_lowercase().contains(&tl)
                || it.desc.contains(t.as_str());
            if hit && !hits.contains(t) {
                hits.push(t.clone());
            }
        }
        WishMatch { item: i, hits, within: w.max_price <= 0 || it.price <= w.max_price }
    };
    let mut all: Vec<WishMatch> = base.into_iter().map(score).collect();
    if all.iter().any(|m| m.within) {
        all.retain(|m| m.within);
        all.sort_by(|a, b| b.hits.len().cmp(&a.hits.len()).then(item(a.item).price.cmp(&item(b.item).price)));
    } else {
        all.sort_by_key(|m| item(m.item).price);
    }
    all
}

// ---- 熟人 / 设置 / 通知 ----

/// 本机熟人：只有一个称呼。
#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct ContactLocal {
    pub id: usize,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct Settings {
    /// 我的称呼：猜我是谁时对方要猜的名字，可以写多个别名。
    pub nickname: String,
    /// 礼物快过期提醒。
    pub notify_gift: bool,
    /// 我答应的契约快到期提醒。
    pub notify_pact: bool,
    /// 好友的日子快到、心愿单上还有没人送的，提醒我。
    pub notify_wish: bool,
    /// 开场三屏看完（或跳过）了没有。
    pub onboarded: bool,
    /// 界面深浅（`theme::ThemeMode::id`）。`None` = 还没选过，按夜色。
    pub theme: Option<String>,
    /// 上次填的收件信息，下次收实物时预填。只在本机。
    pub ship_name: String,
    pub ship_phone: String,
    pub ship_addr: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            nickname: DEFAULT_NICKNAME.into(),
            notify_gift: true,
            notify_pact: true,
            notify_wish: true,
            onboarded: false,
            theme: None,
            ship_name: String::new(),
            ship_phone: String::new(),
            ship_addr: String::new(),
        }
    }
}

impl Settings {
    pub fn theme_mode(&self) -> crate::theme::ThemeMode {
        self.theme
            .as_deref()
            .map(crate::theme::ThemeMode::parse)
            .unwrap_or_default()
    }

    pub fn set_theme_mode(&mut self, mode: crate::theme::ThemeMode) {
        self.theme = Some(mode.id().to_string());
    }
}

/// 只有三类通知（02-flows 6 节）。不做「TA 刚打开了你的礼卡」这类实时通知。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    GiftExpiring,
    PactDue,
    WishSoon,
}

impl NoticeKind {
    pub fn title(self) -> &'static str {
        match self {
            NoticeKind::GiftExpiring => "有礼物等你拆",
            NoticeKind::PactDue => "契约快到期了",
            NoticeKind::WishSoon => "好友的心愿单",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
    /// 点「去看看」要打开的东西：礼物 id、契约 id 或心愿单 id。
    pub target: u64,
}

// ---- 操作的输入与结果 ----

/// 送礼页的草稿。离开送礼页就丢，不落盘。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SendDraft {
    pub item: u16,
    pub peer: String,
    pub unlock: Unlock,
    /// 线索 / 问题 / 暗号提示。
    pub clue: String,
    /// 问题答案 / 暗号。
    pub answer: String,
    /// `None` = 不绑契约。
    pub contract: Option<String>,
    pub message: String,
    pub use_balance: bool,
    /// 从心愿单来的：（心愿单 id, 第几件）。送出即认领。
    pub wish: Option<(u64, usize)>,
    /// 约好哪天送到（只对心愿单，最晚到那个日子）。`None` = 现在就送。
    pub deliver_on: Option<i64>,
    /// 付款方式（`PAY_METHODS` 下标）。`None` = 流水里记「模拟支付」。
    pub pay: Option<u8>,
}

/// 收下 / 换购实物时填的东西。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AcceptForm {
    pub agree: bool,
    pub name: String,
    pub phone: String,
    pub addr: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerOutcome {
    /// 空答案，不扣机会。
    Empty,
    Wrong { left: u8 },
    Right,
    /// 机会用完，礼物照样拆开，身份保密。
    Exhausted,
    /// 这份礼物不在解谜阶段。
    NotOpen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExchangeResult {
    /// 抵扣额 − 新礼物价格：≥ 0 退回余额，< 0 要补。
    pub diff: i64,
    pub from_balance: i64,
    pub external: i64,
}

/// 模拟器推进一步的结果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimStep {
    /// 约好的日子提前到了（演示），礼卡送到 TA 手里。
    Delivered,
    Opened,
    Revealed { known: bool },
    Accepted { pact: bool },
    Exchanged,
    CashedOut { returned: bool },
}

impl SimStep {
    pub fn text(self) -> &'static str {
        match self {
            SimStep::Delivered => "到日子了，礼卡送到了 TA 手里",
            SimStep::Opened => "TA 打开了礼卡，第一次没猜中",
            SimStep::Revealed { known: true } => "TA 答对了，知道是你",
            SimStep::Revealed { known: false } => "TA 机会用完，礼物拆开了，没透露你",
            SimStep::Accepted { pact: true } => "TA 收下了，契约生效",
            SimStep::Accepted { pact: false } => "TA 收下了",
            SimStep::Exchanged => "TA 换了一份更喜欢的",
            SimStep::CashedOut { returned: true } => "TA 折成了余额，还给你回了一份礼",
            SimStep::CashedOut { returned: false } => "TA 折成了余额",
        }
    }
}

/// 删熟人前的快照，给 toast 上的「撤销」用。
pub struct UndoSnapshot {
    pub label: String,
    contacts: Vec<ContactLocal>,
    directory: Vec<String>,
}

/// 手动添加熟人时能出的三种错。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddContactError {
    Empty,
    TooLong,
    Duplicate,
}

impl AddContactError {
    pub fn text(self) -> &'static str {
        match self {
            AddContactError::Empty => "先写一个称呼",
            AddContactError::TooLong => "称呼太长了，16 个字以内",
            AddContactError::Duplicate => "已经有一位这个称呼的熟人了",
        }
    }
}

// ---- 应用状态 ----

pub struct LiyuState {
    pub contacts: Vec<ContactLocal>,
    /// 通讯录池（导入候选 / 猜人候选），和熟人不重名。
    pub directory: Vec<String>,
    pub gifts: Vec<Gift>,
    pub pacts: Vec<Pact>,
    pub ledger: Vec<LedgerEntry>,
    pub settings: Settings,
    /// 我的和好友的心愿单。
    pub wishlists: Vec<Wishlist>,
    next_id: u64,
    /// 读存档失败时给界面的一句话（toast 一次后清掉）。
    pub load_note: Option<&'static str>,
}

/// 视图的 `#[rust]` 字段要 Default：启动时读一次存档（没有就是演示数据）。
impl Default for LiyuState {
    fn default() -> Self {
        Self::load(today_days())
    }
}

/// 落盘的部分。全是 Option：缺字段不报错。
#[derive(Clone, Debug, Default, PartialEq, SerJson, DeJson)]
pub struct PersistedState {
    pub version: Option<u32>,
    pub contacts: Option<Vec<ContactLocal>>,
    pub directory: Option<Vec<String>>,
    pub gifts: Option<Vec<Gift>>,
    pub pacts: Option<Vec<Pact>>,
    pub ledger: Option<Vec<LedgerEntry>>,
    pub settings: Option<Settings>,
    pub next_id: Option<u64>,
    pub wishlists: Option<Vec<Wishlist>>,
}

impl LiyuState {
    /// 演示数据（05-data-model.md「演示数据」）。
    pub fn demo(today: i64) -> Self {
        let mut s = LiyuState {
            contacts: vec![
                ContactLocal { id: 0, label: "林舟".into() },
                ContactLocal { id: 1, label: "陈晓".into() },
                ContactLocal { id: 2, label: "许宁".into() },
            ],
            directory: vec!["周子墨".into(), "林小满".into(), "黄一诺".into(), "吴凯文".into()],
            gifts: Vec::new(),
            pacts: Vec::new(),
            ledger: Vec::new(),
            settings: Settings::default(),
            wishlists: Vec::new(),
            next_id: 1,
            load_note: None,
        };

        // 收到的四份。
        let mut g = s.new_gift(DIR_RECEIVED, 0, today - 1);
        g.peer = "林舟/舟舟".into();
        g.unlock = Unlock::GuessWho.id();
        g.clue = "上周一起淋雨的那个人".into();
        g.contract = "下周找时间回请我喝一杯咖啡".into();
        g.message = "降温了，喝点热的".into();
        s.gifts.push(g);

        let mut g = s.new_gift(DIR_RECEIVED, 4, today - 2);
        g.peer = "陈晓".into();
        g.unlock = Unlock::Question.id();
        g.clue = "我们第一次一起看的电影叫什么？".into();
        g.answer = "星际穿越".into();
        g.contract = "周末陪我看一场电影".into();
        g.message = "这次换我请".into();
        s.gifts.push(g);

        let mut g = s.new_gift(DIR_RECEIVED, 8, today - 5);
        g.peer = "许宁".into();
        g.unlock = Unlock::Passphrase.id();
        g.clue = "我们宿舍的口头禅".into();
        g.answer = "月亮不睡我不睡".into();
        g.message = "抽到隐藏款记得告诉我".into();
        g.demo_tip = "暗号是：月亮不睡我不睡".into();
        s.gifts.push(g);

        let mut g = s.new_gift(DIR_RECEIVED, 2, today - 6);
        g.peer = "陈晓".into();
        g.unlock = Unlock::Free.id();
        g.contract = "收下要发一条朋友圈晒一晒".into();
        g.message = "今天也要开心".into();
        g.set_state(GiftState::Accepted);
        g.identity_known = true;
        g.solved = true;
        g.opened_on = today - 6;
        g.revealed_on = today - 6;
        g.settled_on = today - 6;
        g.voucher = gen_voucher(g.id);
        let (gid, gpeer) = (g.id, g.peer_name());
        s.gifts.push(g);
        let pid = s.take_id();
        s.pacts.push(Pact {
            id: pid,
            gift_id: gid,
            text: "收下要发一条朋友圈晒一晒".into(),
            peer: gpeer,
            mine: true,
            made_on: today - 6,
            due_on: today + 1,
            state: 0,
            nudged_on: 0,
        });

        // 送出的两份。
        s.push_ledger(today - 10, WELCOME_BONUS, 0, "新人礼金", "欢迎来到礼遇");

        let mut g = s.new_gift(DIR_SENT, 6, today - 8);
        g.peer = "许宁".into();
        g.unlock = Unlock::Question.id();
        g.clue = "我们第一次一起旅行去的是哪座城市？".into();
        g.answer = "厦门".into();
        g.contract = "下次见面先给我一个拥抱".into();
        g.message = "晚上点一支，睡个好觉".into();
        g.set_state(GiftState::Accepted);
        g.attempts = 1;
        g.identity_known = true;
        g.solved = true;
        g.opened_on = today - 7;
        g.revealed_on = today - 7;
        g.settled_on = today - 6;
        let (gid, text) = (g.id, g.contract.clone());
        s.gifts.push(g);
        s.push_ledger(today - 8, -WELCOME_BONUS, 6800, "送出 · 香薰蜡烛", "余额抵 ¥20 · 模拟支付 ¥68");
        let pid = s.take_id();
        s.pacts.push(Pact {
            id: pid,
            gift_id: gid,
            text,
            peer: "许宁".into(),
            mine: false,
            made_on: today - 6,
            due_on: today + 1,
            state: 0,
            nudged_on: 0,
        });

        let mut g = s.new_gift(DIR_SENT, 1, today - 3);
        g.peer = "林舟".into();
        g.unlock = Unlock::GuessWho.id();
        g.clue = "猜猜是哪个老同学".into();
        g.answer = DEFAULT_NICKNAME.into();
        g.message = "加班辛苦啦".into();
        g.set_state(GiftState::Revealed);
        g.attempts = 1;
        g.identity_known = true;
        g.solved = true;
        g.opened_on = today - 2;
        g.revealed_on = today - 2;
        s.gifts.push(g);
        s.push_ledger(today - 3, 0, 3500, "送出 · 星巴克中杯拿铁电子券", "模拟支付 ¥35");

        // 心愿单：两张进行中的好友心愿单、一张结束了的，再加我自己的一张。
        let taken = |mut w: WishItem, day: i64| {
            w.state = WISH_BY_OTHERS;
            w.claimed_on = day;
            w
        };
        s.push_demo_wish(
            "林舟",
            0,
            "林舟的生日心愿单",
            "今年想把卧室布置成小影院",
            today + 3,
            today - 5,
            0,
            vec![
                WishItem::exact(17),
                taken(WishItem::exact(0), today - 2),
                WishItem::vague("盲盒", 10000, "潮玩"),
                WishItem::exact(6),
                WishItem::exact(18),
            ],
        );
        s.push_demo_wish(
            "陈晓",
            2,
            "新家暖房心愿单",
            "搬新家啦，缺的都在这儿，来暖房吃饭！",
            today + 12,
            today - 4,
            0,
            vec![
                WishItem::vague("电视", 300_000, "55 寸以上"),
                taken(WishItem::exact(19), today - 3),
                taken(WishItem::exact(21), today - 1),
                WishItem::vague("绿植", 0, "好养"),
                WishItem::exact(22),
            ],
        );
        s.push_demo_wish(
            "许宁",
            3,
            "毕业快乐心愿单",
            "要去新城市上班啦",
            today - 10,
            today - 30,
            today - 3,
            vec![
                taken(WishItem::exact(29), today - 20),
                taken(WishItem::exact(16), today - 15),
                WishItem::exact(7),
            ],
        );
        // 我的：林舟那份「猜我是谁」就是从这张单子上认领的咖啡。
        let lin = s.gifts[0].id;
        let mut coffee = taken(WishItem::exact(0), today - 1);
        coffee.gift_id = lin;
        coffee.given_item = 0;
        let mine = s.push_demo_wish(
            "",
            0,
            "阿岚的生日心愿单",
            "想要的不多，心意最重要",
            today - 1,
            today - 12,
            0,
            vec![coffee, WishItem::vague("耳机", 100_000, "降噪"), WishItem::exact(11)],
        );
        s.gifts[0].wish_id = Some(mine);

        s
    }

    /// 测试用：固定一个「今天」。
    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        Self::demo(TEST_TODAY)
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn new_gift(&mut self, dir: u8, item_id: u16, sent_on: i64) -> Gift {
        let id = self.take_id();
        Gift::blank(id, dir, item_id, sent_on)
    }

    fn push_ledger(&mut self, day: i64, amount: i64, external: i64, title: &str, note: &str) {
        let id = self.take_id();
        self.ledger.push(LedgerEntry {
            id,
            day,
            amount,
            external,
            title: title.into(),
            note: note.into(),
        });
    }

    // ---- 查询 ----

    /// 余额 = 流水之和。不单独存，避免不一致。
    pub fn balance(&self) -> i64 {
        self.ledger.iter().map(|e| e.amount).sum::<i64>().max(0)
    }

    pub fn gift(&self, id: u64) -> Option<&Gift> {
        self.gifts.iter().find(|g| g.id == id)
    }

    fn gift_mut(&mut self, id: u64) -> Option<&mut Gift> {
        self.gifts.iter_mut().find(|g| g.id == id)
    }

    pub fn pact(&self, id: u64) -> Option<&Pact> {
        self.pacts.iter().find(|p| p.id == id)
    }

    /// 收到的（撤回的、约好日子还没送到的不出现），新的在前。
    pub fn received(&self, today: i64) -> Vec<&Gift> {
        let mut v: Vec<&Gift> = self
            .gifts
            .iter()
            .filter(|g| !g.is_sent() && g.state() != GiftState::Withdrawn && !g.is_scheduled(today))
            .collect();
        v.sort_by(|a, b| b.sent_on.cmp(&a.sent_on).then(b.id.cmp(&a.id)));
        v
    }

    /// 送出的，新的在前。
    pub fn sent(&self) -> Vec<&Gift> {
        let mut v: Vec<&Gift> = self.gifts.iter().filter(|g| g.is_sent()).collect();
        v.sort_by(|a, b| b.sent_on.cmp(&a.sent_on).then(b.id.cmp(&a.id)));
        v
    }

    /// 等我处理的收到的礼物（待拆 / 解谜中 / 待决定）。
    pub fn pending_received(&self, today: i64) -> usize {
        self.received(today).iter().filter(|g| g.tone() == Tone::Pending).count()
    }

    /// 契约：`mine` 这一侧，待兑现的在前、到期早的在前。
    pub fn pacts_of(&self, mine: bool) -> Vec<&Pact> {
        let mut v: Vec<&Pact> = self.pacts.iter().filter(|p| p.mine == mine).collect();
        v.sort_by(|a, b| {
            (a.state() != PactState::Pending)
                .cmp(&(b.state() != PactState::Pending))
                .then(a.due_on.cmp(&b.due_on))
                .then(b.id.cmp(&a.id))
        });
        v
    }

    pub fn open_pacts(&self) -> usize {
        self.pacts.iter().filter(|p| p.state() == PactState::Pending).count()
    }

    /// 流水，新的在前。
    pub fn ledger_desc(&self) -> Vec<&LedgerEntry> {
        let mut v: Vec<&LedgerEntry> = self.ledger.iter().collect();
        v.sort_by(|a, b| b.day.cmp(&a.day).then(b.id.cmp(&a.id)));
        v
    }


    /// 送礼的付款拆分：（余额抵扣, 模拟支付）。
    pub fn pay_split(&self, price: i64, use_balance: bool) -> (i64, i64) {
        let a = if use_balance { self.balance().min(price) } else { 0 };
        (a, price - a)
    }

    /// 和这位熟人之间：（我送出的份数, 我收到且知道是 TA 的份数）。
    /// 没揭晓 / 身份保密的礼物不算 —— 否则一个计数就把谜底泄露了。
    pub fn gift_counts(&self, label: &str) -> (usize, usize) {
        let hit = |g: &Gift| split_aliases(&g.peer).iter().any(|a| a == label);
        let sent = self
            .gifts
            .iter()
            .filter(|g| g.is_sent() && g.state() != GiftState::Withdrawn && hit(g))
            .count();
        let recv = self
            .gifts
            .iter()
            .filter(|g| !g.is_sent() && g.shown_sender() != MYSTERY_FRIEND && hit(g))
            .count();
        (sent, recv)
    }

    /// 猜我是谁的候选人：真送礼人 + 熟人 + 通讯录，去重后取 6 个，
    /// 按礼物 id 确定性打乱；真送礼人一定在里面。
    pub fn guess_candidates(&self, g: &Gift) -> Vec<String> {
        let aliases = split_aliases(g.expected_answer());
        let Some(real) = aliases.first().cloned() else {
            return Vec::new();
        };
        let mut others: Vec<String> = Vec::new();
        for n in self.contacts.iter().map(|c| c.label.clone()).chain(self.directory.iter().cloned()) {
            if !aliases.contains(&n) && !others.contains(&n) && n != self.settings.nickname {
                others.push(n);
            }
        }
        // 确定性洗牌：按 (id, 名字) 的哈希排序。
        others.sort_by_key(|n| {
            let mut h = g.id;
            for b in n.bytes() {
                h = mix(h ^ b as u64);
            }
            h
        });
        others.truncate(CANDIDATE_COUNT - 1);
        let pos = (mix(g.id) % (others.len() as u64 + 1)) as usize;
        others.insert(pos, real);
        others
    }

    /// 换购可选项：（目录下标, 抵扣额 − 新价格）。除了原来那件都能换；
    /// 目录大了一屏放不下，只给价格离抵扣额最近的 11 件。
    pub fn exchange_options(&self, g: &Gift) -> Vec<(u16, i64)> {
        let (_, credit) = exchange_credit(g.price);
        let mut v: Vec<(u16, i64)> = (0..CATALOG.len() as u16)
            .filter(|&i| i != g.item)
            .map(|i| (i, credit - item(i).price))
            .collect();
        v.sort_by_key(|&(i, d)| (d.abs(), i));
        v.truncate(EXCHANGE_CHOICES);
        v
    }

    /// 此刻该发哪些通知。每类最多一条；开关是唯一的闸。
    pub fn due_notices(&self, today: i64) -> Vec<Notice> {
        let mut out = Vec::new();
        if self.settings.notify_gift {
            if let Some(g) = self
                .received(today)
                .into_iter()
                .filter(|g| g.state().is_unrevealed())
                .filter(|g| (0..=GIFT_NOTICE_LEAD_DAYS).contains(&g.days_left(today)))
                .min_by_key(|g| g.days_left(today))
            {
                let d = g.days_left(today);
                out.push(Notice {
                    kind: NoticeKind::GiftExpiring,
                    text: if d <= 0 {
                        "有一份神秘礼物还没拆，今天不拆就退回给 TA 了".into()
                    } else {
                        format!("有一份神秘礼物还没拆，{d} 天后会退回给 TA")
                    },
                    target: g.id,
                });
            }
        }
        if self.settings.notify_pact {
            if let Some(p) = self
                .pacts_of(true)
                .into_iter()
                .filter(|p| p.state() == PactState::Pending)
                .find(|p| p.due_on - today <= PACT_NOTICE_LEAD_DAYS)
            {
                let d = p.due_on - today;
                let when = if d > 0 {
                    "明天到期".to_string()
                } else if d == 0 {
                    "今天到期".to_string()
                } else {
                    format!("已逾期 {} 天", -d)
                };
                out.push(Notice {
                    kind: NoticeKind::PactDue,
                    text: format!("你答应的「{}」{}", p.text, when),
                    target: p.id,
                });
            }
        }
        if self.settings.notify_wish {
            if let Some(w) = self
                .friend_wishlists(today)
                .into_iter()
                .filter(|w| w.is_open(today) && w.open_count() > 0)
                .find(|w| (0..=WISH_NOTICE_LEAD_DAYS).contains(&(w.event_on - today)))
            {
                let when = match w.event_on - today {
                    0 => "就是今天".to_string(),
                    1 => "就在明天".to_string(),
                    d => format!("还有 {d} 天"),
                };
                out.push(Notice {
                    kind: NoticeKind::WishSoon,
                    text: format!("{}{}，心愿单上还有 {} 件没人送", w.event_name(""), when, w.open_count()),
                    target: w.id,
                });
            }
        }
        out
    }

    // ---- 送礼 ----

    /// 送礼前的校验。只返回第一条错，界面把它放在按钮上方。
    pub fn validate_draft(&self, d: &SendDraft) -> Result<(), &'static str> {
        if d.item as usize >= CATALOG.len() {
            return Err("先挑一件礼物");
        }
        let clue = d.clue.trim();
        let answer = d.answer.trim();
        match d.unlock {
            Unlock::GuessWho => {
                if clue.is_empty() {
                    return Err("写一句线索，让 TA 有迹可循");
                }
                if split_aliases(&self.settings.nickname).is_empty() {
                    return Err("先在「我」里写好你的称呼");
                }
            }
            Unlock::Question => {
                if clue.is_empty() {
                    return Err("写一个只有你们知道答案的问题");
                }
                if normalize_answer(answer).is_empty() {
                    return Err("问题的答案还没写");
                }
            }
            Unlock::Passphrase => {
                if normalize_answer(answer).is_empty() {
                    return Err("暗号还没写");
                }
            }
            Unlock::Free => {}
        }
        if d.unlock != Unlock::Free {
            if clue.chars().count() > CLUE_MAX_CHARS {
                return Err("线索 / 问题最多 30 个字");
            }
            if answer.chars().count() > ANSWER_MAX_CHARS {
                return Err("答案最多 20 个字");
            }
        }
        if let Some(c) = &d.contract {
            validate_pact(c)?;
        }
        if d.message.trim().chars().count() > MESSAGE_MAX_CHARS {
            return Err("寄语最多 40 个字");
        }
        Ok(())
    }

    /// 送出一份礼物：校验 → 付款（余额优先，余下模拟支付）→ 生成礼卡。返回礼物 id。
    pub fn send_gift(&mut self, d: &SendDraft, today: i64) -> Result<u64, &'static str> {
        self.validate_draft(d)?;
        let claim = self.check_claim(d, today)?;
        let mut g = self.new_gift(DIR_SENT, d.item, today);
        g.peer = d.peer.trim().to_string();
        if let Some((wid, _, owner)) = &claim {
            g.peer = owner.clone();
            g.wish_id = Some(*wid);
        }
        if let Some(day) = d.deliver_on.filter(|&day| day > today) {
            g.sent_on = day;
            g.booked_on = Some(today);
        }
        g.unlock = d.unlock.id();
        match d.unlock {
            Unlock::GuessWho => {
                g.clue = d.clue.trim().into();
                g.answer = self.settings.nickname.trim().into();
            }
            Unlock::Question | Unlock::Passphrase => {
                g.clue = d.clue.trim().into();
                g.answer = d.answer.trim().into();
            }
            Unlock::Free => {}
        }
        g.contract = match &d.contract {
            Some(c) => validate_pact(c)?,
            None => String::new(),
        };
        g.message = d.message.trim().into();
        let (a, b) = self.pay_split(g.price, d.use_balance);
        let title = format!("送出 · {}", g.catalog().name);
        let method = d.pay.and_then(|p| PAY_METHODS.get(p as usize).copied()).unwrap_or("模拟支付");
        let note = match (a > 0, b > 0) {
            (true, true) => format!("余额抵 {} · {method} {}", yuan(a), yuan(b)),
            (true, false) => format!("余额抵 {}", yuan(a)),
            _ => format!("{method} {}", yuan(b)),
        };
        let id = g.id;
        self.gifts.push(g);
        if let Some((wid, idx, _)) = claim {
            let x = &mut self.wishlist_mut(wid).unwrap().items[idx];
            x.state = WISH_BY_ME;
            x.gift_id = id;
            x.claimed_on = today;
            x.given_item = d.item;
        }
        self.push_ledger(today, -a, b, &title, &note);
        self.save();
        Ok(id)
    }

    /// 去结算之前的校验：表单 + 心愿单认领，和 `send_gift` 的前两步完全一样，
    /// 只是不动数据。结算页因此只会因为付款本身出错。
    pub fn check_send(&self, d: &SendDraft, today: i64) -> Result<(), &'static str> {
        self.validate_draft(d)?;
        self.check_claim(d, today).map(|_| ())
    }

    /// 送礼人撤回：只在「待拆」可以，全额退回余额。
    pub fn withdraw(&mut self, id: u64, today: i64) -> Result<(), &'static str> {
        let g = self.gift_mut(id).ok_or("找不到这份礼物")?;
        if !g.is_sent() || g.state() != GiftState::Sealed {
            return Err("TA 已经打开了，撤不回来了");
        }
        g.set_state(GiftState::Withdrawn);
        g.settled_on = today;
        let (price, title) = (g.price, format!("退回 · {}", g.catalog().name));
        self.push_ledger(today, price, 0, &title, "撤回礼物，全额退回");
        self.release_wish(id);
        self.save();
        Ok(())
    }

    // ---- 收礼 ----

    /// 打开一份收到的礼物：待拆 → 解谜中；直接领取的打开即揭晓。返回打开后的状态。
    pub fn open(&mut self, id: u64, today: i64) -> Option<GiftState> {
        let g = self.gift_mut(id)?;
        if g.is_sent() {
            return None;
        }
        if g.state() == GiftState::Sealed {
            g.opened_on = today;
            if g.unlock() == Unlock::Free {
                g.set_state(GiftState::Revealed);
                g.identity_known = true;
                g.solved = true;
                g.revealed_on = today;
            } else {
                g.set_state(GiftState::Opened);
            }
            let st = g.state();
            self.save();
            return Some(st);
        }
        Some(g.state())
    }

    /// 提交一次答案。空答案不扣机会；3 次用完照样揭晓，身份保密。
    pub fn submit_answer(&mut self, id: u64, guess: &str, today: i64) -> AnswerOutcome {
        let Some(g) = self.gift_mut(id) else {
            return AnswerOutcome::NotOpen;
        };
        if g.is_sent() || g.state() != GiftState::Opened {
            return AnswerOutcome::NotOpen;
        }
        if normalize_answer(guess).is_empty() {
            return AnswerOutcome::Empty;
        }
        g.attempts = g.attempts.saturating_add(1);
        let aliases = g.unlock() == Unlock::GuessWho;
        let out = if answer_matches(g.expected_answer(), guess, aliases) {
            g.set_state(GiftState::Revealed);
            g.identity_known = true;
            g.solved = true;
            g.revealed_on = today;
            AnswerOutcome::Right
        } else if g.attempts >= MAX_ATTEMPTS {
            g.set_state(GiftState::Revealed);
            g.identity_known = false;
            g.revealed_on = today;
            AnswerOutcome::Exhausted
        } else {
            AnswerOutcome::Wrong { left: g.attempts_left() }
        };
        self.save();
        out
    }

    fn check_ship(physical: bool, f: &AcceptForm) -> Result<(), &'static str> {
        if !physical {
            return Ok(());
        }
        if f.name.trim().is_empty() {
            return Err("收件人还没填");
        }
        if !valid_phone(&f.phone) {
            return Err("手机号要 11 位数字");
        }
        if f.addr.trim().is_empty() {
            return Err("收件地址还没填");
        }
        Ok(())
    }

    fn keep_ship(&mut self, id: u64, physical: bool, f: &AcceptForm) {
        if !physical {
            return;
        }
        let (n, p, a) = (f.name.trim().to_string(), f.phone.trim().to_string(), f.addr.trim().to_string());
        if let Some(g) = self.gift_mut(id) {
            g.ship_name = n.clone();
            g.ship_phone = p.clone();
            g.ship_addr = a.clone();
        }
        self.settings.ship_name = n;
        self.settings.ship_phone = p;
        self.settings.ship_addr = a;
    }

    fn revealed_received(&self, id: u64) -> Result<&Gift, &'static str> {
        let g = self.gift(id).ok_or("找不到这份礼物")?;
        if g.is_sent() || g.state() != GiftState::Revealed {
            return Err("这份礼物已经处理过了");
        }
        Ok(g)
    }

    /// 开心收下：有契约必须同意（同意即揭晓 TA）；实物要地址；电子券给券码。
    pub fn accept(&mut self, id: u64, f: &AcceptForm, today: i64) -> Result<(), &'static str> {
        let g = self.revealed_received(id)?;
        if g.has_contract() && !f.agree {
            return Err("先打开「我同意」，才能收下");
        }
        let physical = g.catalog().physical;
        Self::check_ship(physical, f)?;
        let pact = g.has_contract().then(|| (g.contract.clone(), g.peer_name()));
        self.keep_ship(id, physical, f);
        let g = self.gift_mut(id).unwrap();
        g.set_state(GiftState::Accepted);
        g.settled_on = today;
        if !physical {
            g.voucher = gen_voucher(g.id);
        }
        if let Some((text, peer)) = pact {
            g.identity_known = true;
            let pid = self.take_id();
            self.pacts.push(Pact {
                id: pid,
                gift_id: id,
                text,
                peer,
                mine: true,
                made_on: today,
                due_on: today + PACT_DAYS,
                state: 0,
                nudged_on: 0,
            });
        }
        self.save();
        Ok(())
    }

    /// 折成余额：扣 8% 手续费（至少 ¥1），契约作废。返回退回的金额。
    pub fn cash_out(&mut self, id: u64, today: i64) -> Result<i64, &'static str> {
        let g = self.revealed_received(id)?;
        let (f, refund) = cashout_quote(g.price);
        let title = format!("折现 · {}", g.catalog().name);
        let g = self.gift_mut(id).unwrap();
        g.set_state(GiftState::CashedOut);
        g.settled_on = today;
        g.refund = refund;
        self.push_ledger(today, refund, 0, &title, &format!("手续费 {}（{}%）", yuan(f), CASHOUT_FEE_PCT));
        self.save();
        Ok(refund)
    }

    /// 换一份：抵扣额 = 价格 − 5% 手续费，多退少补（余额先补，不够的模拟支付）。
    pub fn exchange(&mut self, id: u64, new_item: u16, f: &AcceptForm, today: i64) -> Result<ExchangeResult, &'static str> {
        let g = self.revealed_received(id)?;
        if new_item as usize >= CATALOG.len() {
            return Err("先选一件要换的礼物");
        }
        if new_item == g.item {
            return Err("换一件不一样的吧");
        }
        let physical = item(new_item).physical;
        Self::check_ship(physical, f)?;
        let (fe, credit) = exchange_credit(g.price);
        let diff = credit - item(new_item).price;
        let names = format!("{} → {}", g.catalog().name, item(new_item).name);
        let fee_note = format!("手续费 {}（{}%）", yuan(fe), EXCHANGE_FEE_PCT);
        let res = if diff >= 0 {
            self.push_ledger(today, diff, 0, &format!("换购退差 · {names}"), &fee_note);
            ExchangeResult { diff, from_balance: 0, external: 0 }
        } else {
            let need = -diff;
            let a = self.balance().min(need);
            let b = need - a;
            let mut note = fee_note;
            if a > 0 {
                note.push_str(&format!(" · 余额补 {}", yuan(a)));
            }
            if b > 0 {
                note.push_str(&format!(" · 模拟支付 {}", yuan(b)));
            }
            self.push_ledger(today, -a, b, &format!("换购补差 · {names}"), &note);
            ExchangeResult { diff, from_balance: a, external: b }
        };
        self.keep_ship(id, physical, f);
        let g = self.gift_mut(id).unwrap();
        g.set_state(GiftState::Exchanged);
        g.settled_on = today;
        g.swap_item = new_item;
        g.refund = diff.max(0);
        if !physical {
            g.voucher = gen_voucher(g.id ^ 0xE5);
        }
        self.save();
        Ok(res)
    }

    // ---- 过期 ----

    /// 7 天没揭晓的礼物过期：收到的退回给 TA，送出的全额退回我的余额。幂等。
    /// 返回这次过期了几份。
    pub fn sweep(&mut self, today: i64) -> usize {
        let due: Vec<u64> = self
            .gifts
            .iter()
            .filter(|g| g.state().is_unrevealed() && g.days_left(today) <= 0)
            .map(|g| g.id)
            .collect();
        for id in &due {
            let g = self.gift_mut(*id).unwrap();
            g.set_state(GiftState::Expired);
            g.settled_on = g.sent_on + EXPIRE_DAYS;
            if g.is_sent() {
                let (day, price, title) = (g.settled_on, g.price, format!("退回 · {}", g.catalog().name));
                self.push_ledger(day, price, 0, &title, "7 天没拆开，全额退回");
            }
            self.release_wish(*id);
        }
        if !due.is_empty() {
            self.save();
        }
        due.len()
    }

    // ---- 契约 ----

    /// 标记 / 确认已兑现。逾期了也可以，没有惩罚。
    pub fn fulfil_pact(&mut self, id: u64) -> Result<(), &'static str> {
        let p = self.pacts.iter_mut().find(|p| p.id == id).ok_or("找不到这条契约")?;
        if p.state() != PactState::Pending {
            return Err("这条契约已经了结了");
        }
        p.state = 1;
        self.save();
        Ok(())
    }

    /// 「免了吧」：只有答应我的那一侧能放过对方。
    pub fn waive_pact(&mut self, id: u64) -> Result<(), &'static str> {
        let p = self.pacts.iter_mut().find(|p| p.id == id).ok_or("找不到这条契约")?;
        if p.mine {
            return Err("答应了就是答应了");
        }
        if p.state() != PactState::Pending {
            return Err("这条契约已经了结了");
        }
        p.state = 2;
        self.save();
        Ok(())
    }

    /// 「提醒 TA」：每条每天一次。
    pub fn nudge_pact(&mut self, id: u64, today: i64) -> Result<(), &'static str> {
        let p = self.pacts.iter_mut().find(|p| p.id == id).ok_or("找不到这条契约")?;
        if p.mine || p.state() != PactState::Pending {
            return Err("这条契约不用提醒");
        }
        if p.nudged_on == today {
            return Err("今天已经提醒过啦");
        }
        p.nudged_on = today;
        self.save();
        Ok(())
    }

    // ---- 钱包 ----

    pub fn top_up(&mut self, today: i64) {
        self.push_ledger(today, TOP_UP_AMOUNT, 0, "充值（模拟）", "演示用，不是真钱");
        self.save();
    }

    // ---- 演示模拟器 ----

    /// 替「对方」推进一步（只对送出的礼物）。结果只由礼物 id 决定，方便复现。
    pub fn simulate_step(&mut self, id: u64, today: i64) -> Option<SimStep> {
        // Knuth 乘法散列：低位只是 id 本身的余数（乘数 ≡ 1 mod 3、mod 4），
        // 所以三个判断各取乘积高处的一段，互不相关。
        let seed = id.wrapping_mul(2_654_435_761);
        let (r_guess, r_decide, r_return) = ((seed >> 16) % 4, (seed >> 20) % 3, (seed >> 24) % 2);
        let mut g = self.gift(id)?.clone();
        if !g.is_sent() {
            return None;
        }
        // 先在副本上推进，再一次写回：新契约、回礼要取新 id，不能和礼物的借用叠在一起。
        let mut new_pact: Option<(String, String)> = None;
        let mut return_gift: Option<(u16, String)> = None;
        let step = match g.state() {
            GiftState::Sealed if g.is_scheduled(today) => {
                g.sent_on = today;
                SimStep::Delivered
            }
            GiftState::Sealed if g.unlock() == Unlock::Free => {
                g.set_state(GiftState::Revealed);
                g.identity_known = true;
                g.solved = true;
                g.opened_on = today;
                g.revealed_on = today;
                SimStep::Revealed { known: true }
            }
            GiftState::Sealed => {
                g.set_state(GiftState::Opened);
                g.attempts = 1;
                g.opened_on = today;
                SimStep::Opened
            }
            GiftState::Opened => {
                let known = r_guess != 0;
                g.set_state(GiftState::Revealed);
                g.attempts = if known { 2 } else { MAX_ATTEMPTS };
                g.identity_known = known;
                g.solved = known;
                g.revealed_on = today;
                SimStep::Revealed { known }
            }
            GiftState::Revealed => {
                g.settled_on = today;
                match r_decide {
                    0 => {
                        g.set_state(GiftState::Accepted);
                        let pact = g.has_contract();
                        if pact {
                            // 同意契约即揭晓：契约要有对象。
                            g.identity_known = true;
                            new_pact = Some((g.contract.clone(), g.shown_recipient()));
                        }
                        SimStep::Accepted { pact }
                    }
                    1 => {
                        g.set_state(GiftState::Exchanged);
                        g.swap_item = (g.item + 1) % CATALOG.len() as u16;
                        SimStep::Exchanged
                    }
                    _ => {
                        g.set_state(GiftState::CashedOut);
                        let (_, refund) = cashout_quote(g.price);
                        g.refund = refund;
                        let best = best_item_within(refund).filter(|_| r_return == 0);
                        if let Some(best) = best {
                            let peer = if g.peer.is_empty() { "一位朋友".to_string() } else { g.peer.clone() };
                            return_gift = Some((best, peer));
                        }
                        SimStep::CashedOut { returned: best.is_some() }
                    }
                }
            }
            _ => return None,
        };
        *self.gift_mut(id)? = g;
        if let Some((text, peer)) = new_pact {
            let pid = self.take_id();
            self.pacts.push(Pact {
                id: pid,
                gift_id: id,
                text,
                peer,
                mine: false,
                made_on: today,
                due_on: today + PACT_DAYS,
                state: 0,
                nudged_on: 0,
            });
        }
        if let Some((best, peer)) = return_gift {
            // 「反击礼物」：同一个人、不超过刚折现金额的最贵一件、直接领取。
            let mut r = self.new_gift(DIR_RECEIVED, best, today);
            r.peer = peer;
            r.unlock = Unlock::Free.id();
            r.message = RETURN_MESSAGE.into();
            self.gifts.push(r);
        }
        self.save();
        Some(step)
    }

    // ---- 心愿单 ----

    #[allow(clippy::too_many_arguments)]
    fn push_demo_wish(
        &mut self,
        owner: &str,
        occasion: u8,
        title: &str,
        note: &str,
        event_on: i64,
        created_on: i64,
        closed_on: i64,
        items: Vec<WishItem>,
    ) -> u64 {
        let id = self.take_id();
        self.wishlists.push(Wishlist {
            id,
            owner: owner.into(),
            occasion,
            title: title.into(),
            note: note.into(),
            event_on,
            created_on,
            closed_on,
            audience: Vec::new(),
            items,
        });
        id
    }

    pub fn wishlist(&self, id: u64) -> Option<&Wishlist> {
        self.wishlists.iter().find(|w| w.id == id)
    }

    fn wishlist_mut(&mut self, id: u64) -> Option<&mut Wishlist> {
        self.wishlists.iter_mut().find(|w| w.id == id)
    }

    /// 我的心愿单：进行中的在前，日子近的在前。
    pub fn my_wishlists(&self, today: i64) -> Vec<&Wishlist> {
        let mut v: Vec<&Wishlist> = self.wishlists.iter().filter(|w| w.is_mine()).collect();
        v.sort_by(|a, b| {
            b.is_open(today)
                .cmp(&a.is_open(today))
                .then(a.event_on.cmp(&b.event_on))
                .then(b.id.cmp(&a.id))
        });
        v
    }

    /// 好友的心愿单：进行中的按日子由近到远，结束了的放后面（新结束的在前）。
    pub fn friend_wishlists(&self, today: i64) -> Vec<&Wishlist> {
        let mut v: Vec<&Wishlist> = self.wishlists.iter().filter(|w| !w.is_mine()).collect();
        v.sort_by(|a, b| {
            let (oa, ob) = (a.is_open(today), b.is_open(today));
            ob.cmp(&oa).then(if oa {
                a.event_on.cmp(&b.event_on)
            } else {
                b.event_on.cmp(&a.event_on)
            })
        });
        v
    }

    /// 「加到我的心愿单」加到哪张：进行中、日子最近的那张。
    pub fn current_my_wishlist(&self, today: i64) -> Option<u64> {
        self.my_wishlists(today).into_iter().find(|w| w.is_open(today)).map(|w| w.id)
    }

    /// 这位熟人进行中的心愿单（通讯录副标题用）。
    pub fn wish_hint(&self, label: &str, today: i64) -> Option<String> {
        let w = self.friend_wishlists(today).into_iter().find(|w| w.owner == label && w.is_open(today))?;
        Some(format!("{}心愿单 · {} · {} 件没人送", w.occasion_label(), w.countdown(today), w.open_count()))
    }

    /// 一件心愿此刻的状态文案。我自己的看「谁送的」（揭晓后才说名字），好友的只看「有没有人送」。
    pub fn wish_item_status(&self, list: &Wishlist, idx: usize, today: i64) -> String {
        let Some(w) = list.items.get(idx) else {
            return String::new();
        };
        if !list.is_mine() {
            return match w.state {
                WISH_OPEN if !list.is_open(today) => "没人送".into(),
                WISH_OPEN => "还没人送".into(),
                WISH_BY_ME => "你送的".into(),
                _ => "已有人送".into(),
            };
        }
        if w.is_open() {
            return "还没人认领".into();
        }
        match self.gift(w.gift_id) {
            Some(g) if g.is_scheduled(today) => format!("已被认领 · {} 送到", md_cn(g.sent_on)),
            Some(g) if g.state().is_unrevealed() => "已被认领 · 礼物在礼盒里等你拆".into(),
            Some(g) if g.shown_sender() != MYSTERY_FRIEND => format!("{} 送的", g.shown_sender()),
            _ => "已被认领".into(),
        }
    }

    /// 发布 / 保存前的校验。只返回第一条错。
    pub fn validate_wish(&self, d: &WishDraft, today: i64) -> Result<(), &'static str> {
        if d.title.trim().chars().count() > WISH_TITLE_MAX_CHARS {
            return Err("标题最多 16 个字");
        }
        if d.note.trim().chars().count() > WISH_NOTE_MAX_CHARS {
            return Err("想说的话最多 40 个字");
        }
        let old = d.id.and_then(|id| self.wishlist(id));
        let unchanged_day = old.is_some_and(|w| w.event_on == d.event_on);
        if d.event_on < today && !unchanged_day {
            return Err("日子不能早于今天");
        }
        if d.event_on > today + WISH_MAX_AHEAD_DAYS {
            return Err("最多提前半年发布");
        }
        if d.items.is_empty() {
            return Err("至少放一件心愿");
        }
        if d.items.len() > WISH_MAX_ITEMS {
            return Err("一张心愿单最多 8 件");
        }
        for (k, w) in d.items.iter().enumerate() {
            if w.item.is_none() && !WISH_KINDS.contains(&w.kind.as_str()) {
                return Err("选一下想要的是什么");
            }
            if w.wants.chars().count() > WISH_WANTS_MAX_CHARS {
                return Err("要求最多 20 个字");
            }
            if w.item.is_some() && d.items[..k].iter().any(|x| x.item == w.item) {
                return Err("同一件放了两次");
            }
        }
        if let Some(old) = old {
            if !old.is_mine() {
                return Err("只能改自己的心愿单");
            }
            // 已经有人认领的那几件是别人的心意，编辑时不能拿掉。
            if old.items.iter().filter(|w| !w.is_open()).any(|w| !d.items.contains(w)) {
                return Err("已经有人认领的心愿不能删");
            }
        }
        Ok(())
    }

    /// 发布新的，或者保存编辑。返回心愿单 id。
    pub fn publish_wish(&mut self, d: &WishDraft, today: i64) -> Result<u64, &'static str> {
        self.validate_wish(d, today)?;
        let title = match d.title.trim() {
            "" => d.default_title(&self.settings.nickname),
            t => t.to_string(),
        };
        let audience: Vec<String> = d.audience.iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
        let id = match d.id {
            Some(id) => {
                let w = self.wishlist_mut(id).ok_or("找不到这张心愿单")?;
                w.occasion = d.occasion;
                w.title = title;
                w.note = d.note.trim().into();
                w.event_on = d.event_on;
                w.audience = audience;
                w.items = d.items.clone();
                id
            }
            None => {
                let id = self.take_id();
                self.wishlists.push(Wishlist {
                    id,
                    owner: String::new(),
                    occasion: d.occasion,
                    title,
                    note: d.note.trim().into(),
                    event_on: d.event_on,
                    created_on: today,
                    closed_on: 0,
                    audience,
                    items: d.items.clone(),
                });
                id
            }
        };
        self.save();
        Ok(id)
    }

    /// 把一件具体的商品加到我进行中的心愿单上。没有进行中的心愿单时返回错，界面转去发布一张。
    pub fn add_to_wishlist(&mut self, list: u64, i: u16, today: i64) -> Result<(), &'static str> {
        let w = self.wishlist_mut(list).ok_or("找不到这张心愿单")?;
        if !w.is_mine() || !w.is_open(today) {
            return Err("这张心愿单已经结束了");
        }
        if w.items.iter().any(|x| x.item == Some(i)) {
            return Err("已经在心愿单上了");
        }
        if w.items.len() >= WISH_MAX_ITEMS {
            return Err("心愿单满了，最多 8 件");
        }
        w.items.push(WishItem::exact(i));
        self.save();
        Ok(())
    }

    /// 提前结束：之后好友看得到，但不能再认领。
    pub fn close_wish(&mut self, id: u64, today: i64) -> Result<(), &'static str> {
        let w = self.wishlist_mut(id).ok_or("找不到这张心愿单")?;
        if !w.is_mine() {
            return Err("只能结束自己的心愿单");
        }
        if !w.is_open(today) {
            return Err("这张心愿单已经结束了");
        }
        w.closed_on = today;
        self.save();
        Ok(())
    }

    /// 删掉：只有还没人认领时可以（认领过的是别人的心意，只能结束）。
    pub fn delete_wish(&mut self, id: u64) -> Result<(), &'static str> {
        let w = self.wishlist(id).ok_or("找不到这张心愿单")?;
        if !w.is_mine() {
            return Err("只能删自己的心愿单");
        }
        if w.claimed_count() > 0 {
            return Err("已经有人认领了，只能结束，不能删除");
        }
        self.wishlists.retain(|w| w.id != id);
        self.save();
        Ok(())
    }

    /// 送礼时认领心愿：单子得是好友的、还开着，那件还没人送，送的东西对得上。
    fn check_claim(&self, d: &SendDraft, today: i64) -> Result<Option<(u64, usize, String)>, &'static str> {
        let Some((wid, idx)) = d.wish else {
            if d.deliver_on.is_some_and(|day| day > today) {
                return Err("只有心愿单上的礼物能约好日子送");
            }
            return Ok(None);
        };
        let w = self.wishlist(wid).ok_or("找不到这张心愿单")?;
        if w.is_mine() {
            return Err("这是你自己的心愿单");
        }
        if !w.is_open(today) {
            return Err("心愿单已经结束了");
        }
        let wi = w.items.get(idx).ok_or("找不到这件心愿")?;
        if !wi.is_open() {
            return Err("这件已经有人送了");
        }
        if !wi.fits(d.item) {
            return Err(if wi.is_exact() { "心愿单上要的是另一件" } else { "要送心愿单上说的那一类" });
        }
        if let Some(day) = d.deliver_on {
            if day > w.event_on.max(today) {
                return Err("送到的日子不能晚于 TA 的日子");
            }
        }
        Ok(Some((wid, idx, w.owner.clone())))
    }

    /// 礼物退回（撤回 / 过期）时放开它认领的那件心愿，别人还能再送。
    fn release_wish(&mut self, gift_id: u64) {
        for w in &mut self.wishlists {
            for x in &mut w.items {
                if x.gift_id == gift_id && !x.is_open() {
                    x.release();
                }
            }
        }
    }

    /// 演示：替一位好友认领我心愿单上还没人送的第一件。礼物以「猜我是谁」送来，
    /// 日子还没到就约在那天送到。返回 toast 文案。
    pub fn simulate_wish_claim(&mut self, id: u64, today: i64) -> Result<String, &'static str> {
        let w = self.wishlist(id).ok_or("找不到这张心愿单")?;
        if !w.is_mine() || !w.is_open(today) {
            return Err("这张心愿单已经结束了");
        }
        let idx = w.items.iter().position(|x| x.is_open()).ok_or("心愿都有人送啦")?;
        let wi = w.items[idx].clone();
        let event_on = w.event_on;
        if self.contacts.is_empty() {
            return Err("先加一位熟人，才有人来认领");
        }
        let who = self.contacts[(mix(id ^ (idx as u64 + 1)) % self.contacts.len() as u64) as usize].label.clone();
        let pick = wish_candidates(&wi).first().map(|m| m.item).unwrap_or(0);
        let day = event_on.max(today);
        let mut g = self.new_gift(DIR_RECEIVED, pick, day);
        g.peer = who;
        g.unlock = Unlock::GuessWho.id();
        g.clue = "看到你的心愿单啦，猜猜我是谁".into();
        g.message = "心愿单上的那件，送你".into();
        g.wish_id = Some(id);
        if day > today {
            g.booked_on = Some(today);
        }
        let gid = g.id;
        self.gifts.push(g);
        let x = &mut self.wishlist_mut(id).unwrap().items[idx];
        x.state = WISH_BY_OTHERS;
        x.gift_id = gid;
        x.claimed_on = today;
        x.given_item = pick;
        self.save();
        Ok(if day > today {
            format!("有位朋友认领了「{}」，{} 送到", wi.title(), md_cn(day))
        } else {
            format!("有位朋友认领了「{}」，礼物在礼盒里", wi.title())
        })
    }

    // ---- 熟人 ----

    pub fn contact(&self, id: usize) -> Option<&ContactLocal> {
        self.contacts.iter().find(|c| c.id == id)
    }

    /// 手动添一个熟人。重名直接拒绝。
    pub fn add_contact(&mut self, label: &str) -> Result<usize, AddContactError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(AddContactError::Empty);
        }
        if label.chars().count() > NICKNAME_MAX_CHARS {
            return Err(AddContactError::TooLong);
        }
        if self.contacts.iter().any(|c| c.label == label) {
            return Err(AddContactError::Duplicate);
        }
        let id = self.contacts.iter().map(|c| c.id + 1).max().unwrap_or(0);
        self.contacts.push(ContactLocal { id, label: label.to_string() });
        self.directory.retain(|d| d != label);
        self.save();
        Ok(id)
    }

    /// 一批名字加成熟人（导入用）。重名、太长的跳过。返回加了几位。
    pub fn adopt_names(&mut self, names: Vec<String>) -> usize {
        let mut n = 0;
        for name in names {
            let name = name.trim().to_string();
            if name.is_empty()
                || name.chars().count() > NICKNAME_MAX_CHARS
                || self.contacts.iter().any(|c| c.label == name)
            {
                continue;
            }
            let id = self.contacts.iter().map(|c| c.id + 1).max().unwrap_or(0);
            self.contacts.push(ContactLocal { id, label: name.clone() });
            self.directory.retain(|d| *d != name);
            n += 1;
        }
        if n > 0 {
            self.save();
        }
        n
    }

    /// 把 vCard 导入的名字并入通讯录池（与通讯录、熟人双向去重）。返回新并入的人数。
    pub fn merge_directory(&mut self, names: Vec<String>) -> usize {
        let mut added = 0;
        for n in names {
            let n = n.trim().to_string();
            if n.is_empty()
                || self.directory.iter().any(|d| *d == n)
                || self.contacts.iter().any(|c| c.label == n)
            {
                continue;
            }
            self.directory.push(n);
            added += 1;
        }
        added
    }

    /// 删一位熟人。礼物和契约里的称呼是快照，不受影响。返回撤销用的快照。
    pub fn remove_contact(&mut self, id: usize) -> Option<UndoSnapshot> {
        let label = self.contact(id)?.label.clone();
        let snap = UndoSnapshot {
            label: format!("已删除「{label}」"),
            contacts: self.contacts.clone(),
            directory: self.directory.clone(),
        };
        self.contacts.retain(|c| c.id != id);
        self.save();
        Some(snap)
    }

    pub fn restore(&mut self, snap: UndoSnapshot) {
        self.contacts = snap.contacts;
        self.directory = snap.directory;
        self.save();
    }

    /// 改我的称呼。可以写多个别名（用 / 分隔）。
    pub fn set_nickname(&mut self, name: &str) -> Result<(), &'static str> {
        let t = name.trim();
        if split_aliases(t).is_empty() {
            return Err("称呼不能是空的");
        }
        if t.chars().count() > NICKNAME_MAX_CHARS * 2 {
            return Err("称呼太长了");
        }
        self.settings.nickname = t.to_string();
        self.save();
        Ok(())
    }

    // ---- 持久化 ----

    /// `$MAKEPAD_HOME/liyu/state.json`（宿主会给子进程设 MAKEPAD_HOME）。
    pub fn state_file() -> Option<std::path::PathBuf> {
        std::env::var_os("MAKEPAD_HOME")
            .map(|h| std::path::Path::new(&h).join("liyu").join("state.json"))
    }

    /// 状态目录（礼卡图片、导出都放这里）。
    pub fn data_dir() -> Option<std::path::PathBuf> {
        Self::state_file().and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }

    /// vCard 导入文件：`<MAKEPAD_HOME>/liyu/contacts.vcf`。
    pub fn contacts_vcf() -> Option<std::path::PathBuf> {
        Self::data_dir().map(|d| d.join("contacts.vcf"))
    }

    /// 状态目录里没有 contacts.vcf 时写一份示例，让「导入 vCard」开箱可点。
    pub fn ensure_sample_vcard() {
        let Some(path) = Self::contacts_vcf() else {
            return;
        };
        if path.exists() {
            return;
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, SAMPLE_VCARD);
    }

    pub fn persisted(&self) -> PersistedState {
        PersistedState {
            version: Some(STATE_VERSION),
            contacts: Some(self.contacts.clone()),
            directory: Some(self.directory.clone()),
            gifts: Some(self.gifts.clone()),
            pacts: Some(self.pacts.clone()),
            ledger: Some(self.ledger.clone()),
            settings: Some(self.settings.clone()),
            next_id: Some(self.next_id),
            wishlists: Some(self.wishlists.clone()),
        }
    }

    pub fn apply_persisted(&mut self, p: PersistedState) {
        if let Some(v) = p.contacts {
            self.contacts = v;
        }
        if let Some(v) = p.directory {
            self.directory = v;
        }
        if let Some(v) = p.gifts {
            self.gifts = v;
        }
        if let Some(v) = p.pacts {
            self.pacts = v;
        }
        if let Some(v) = p.ledger {
            self.ledger = v;
        }
        if let Some(v) = p.settings {
            self.settings = v;
        }
        if let Some(v) = p.wishlists {
            self.wishlists = v;
        }
        // 自增 id 至少要比现有的都大，存档里的数字不可信时也不会撞号。
        let max_id = self
            .gifts
            .iter()
            .map(|g| g.id)
            .chain(self.pacts.iter().map(|p| p.id))
            .chain(self.ledger.iter().map(|e| e.id))
            .chain(self.wishlists.iter().map(|w| w.id))
            .max()
            .unwrap_or(0);
        self.next_id = p.next_id.unwrap_or(0).max(max_id + 1);
    }

    /// 读存档并覆盖在演示数据上；没有存档用演示数据，存档坏了也用演示数据并留一句话。
    pub fn load(today: i64) -> Self {
        let mut s = Self::demo(today);
        if let Some(path) = Self::state_file() {
            if path.exists() {
                match Self::load_from(&path) {
                    Some(p) => s.apply_persisted(p),
                    None => s.load_note = Some("数据读不出来，已用演示数据"),
                }
            }
        }
        s
    }

    /// 读取状态文件。版本不是当前版本、不是礼遇格式（没有 gifts）或解析不了都算读不出来 ——
    /// 旧版本的数据不做迁移。
    pub fn load_from(path: &std::path::Path) -> Option<PersistedState> {
        let text = std::fs::read_to_string(path).ok()?;
        let p = PersistedState::deserialize_json_lenient(&text).ok()?;
        (p.version == Some(STATE_VERSION) && p.gifts.is_some()).then_some(p)
    }

    /// 只把落盘的深浅选择读出来：注册预设比建视图早，那时还没有 `LiyuState`。
    pub fn persisted_theme() -> Option<String> {
        let path = Self::state_file()?;
        Self::load_from(&path)?.settings?.theme
    }

    pub fn save(&self) {
        if cfg!(test) {
            return;
        }
        if let Some(path) = Self::state_file() {
            let _ = self.save_to(&path);
        }
    }

    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.persisted().serialize_json())
    }

    /// 导出本机数据：落盘的那份原样写到 `liyu/export-<日期>.json`（含收件地址 —— 本机数据归用户）。
    pub fn export_data(&self) -> Option<std::path::PathBuf> {
        let dir = Self::data_dir()?;
        let path = dir.join(format!("export-{}.json", fmt_days(today_days())));
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(&path, self.persisted().serialize_json()).ok()?;
        Some(path)
    }

    /// 恢复演示数据。深浅和「看过引导」留着 —— 那是偏好，不是数据。
    pub fn reset_demo(&mut self, today: i64) {
        let theme = self.settings.theme.clone();
        let onboarded = self.settings.onboarded;
        *self = Self::demo(today);
        self.settings.theme = theme;
        self.settings.onboarded = onboarded;
        self.save();
    }
}

#[cfg(test)]
pub(crate) const TEST_TODAY: i64 = 20_720; // 2026-09-24


// ---- vCard ----

/// 演示示例 vCard：FN、带参数的 FN、续行折叠、N 兜底各一份。
pub const SAMPLE_VCARD: &str = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:周子墨\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:4.0\r\nFN:林小\r\n 满\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nN:黄;一诺;;;\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:4.0\r\nFN;CHARSET=UTF-8:吴凯文\r\nEND:VCARD\r\n";

/// 简单 vCard 3.0/4.0 解析：取每个 CONTACT 块的 FN（没有则用 N 兜底），
/// 支持行折叠（以空格/制表符开头的续行拼回上一行）。忽略照片等复杂字段。
pub fn parse_vcard(text: &str) -> Vec<String> {
    // 先展开折叠行。
    let mut lines: Vec<String> = Vec::new();
    for raw in text.lines() {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if (raw.starts_with(' ') || raw.starts_with('\t')) && !lines.is_empty() {
            let cont = raw.trim_start_matches([' ', '\t']);
            lines.last_mut().unwrap().push_str(cont);
        } else {
            lines.push(raw.to_string());
        }
    }
    let mut names = Vec::new();
    let mut in_card = false;
    let mut fn_name: Option<String> = None;
    let mut n_name: Option<String> = None;
    for line in &lines {
        if line.eq_ignore_ascii_case("BEGIN:VCARD") {
            in_card = true;
            fn_name = None;
            n_name = None;
            continue;
        }
        if line.eq_ignore_ascii_case("END:VCARD") {
            if in_card {
                if let Some(n) = fn_name.take().or_else(|| n_name.take()) {
                    let n = n.trim().to_string();
                    if !n.is_empty() {
                        names.push(n);
                    }
                }
            }
            in_card = false;
            continue;
        }
        if !in_card {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        // 属性名取第一个 ':' 之前、';' 之前的部分（丢掉 CHARSET 等参数）。
        let key = line[..colon].split(';').next().unwrap_or("");
        let value = line[colon + 1..].trim();
        if key.eq_ignore_ascii_case("FN") {
            fn_name = Some(value.to_string());
        } else if key.eq_ignore_ascii_case("N") {
            n_name = Some(parse_vcard_n(value));
        }
    }
    names
}

/// N 字段（`姓;名;中间名;前缀;后缀`）→ 显示名：中文按「姓名」拼接，
/// 纯 ASCII 按西方习惯「名 姓」。
fn parse_vcard_n(value: &str) -> String {
    let mut parts = value.split(';');
    let family = parts.next().unwrap_or("").trim();
    let given = parts.next().unwrap_or("").trim();
    if family.is_empty() {
        given.to_string()
    } else if given.is_empty() {
        family.to_string()
    } else if family.is_ascii() && given.is_ascii() {
        format!("{} {}", given, family)
    } else {
        format!("{}{}", family, given)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: i64 = TEST_TODAY;

    fn received_by_unlock(s: &LiyuState, u: Unlock) -> u64 {
        s.gifts
            .iter()
            .find(|g| !g.is_sent() && g.unlock() == u && g.state() == GiftState::Sealed)
            .map(|g| g.id)
            .expect("演示数据里应有这种玩法的待拆礼物")
    }

    fn draft(item_id: u16) -> SendDraft {
        SendDraft {
            item: item_id,
            peer: "林舟".into(),
            unlock: Unlock::Question,
            clue: "我们在哪认识的？".into(),
            answer: "图书馆".into(),
            contract: None,
            message: String::new(),
            use_balance: true,
            wish: None,
            deliver_on: None,
            pay: None,
        }
    }

    fn friend_list(s: &LiyuState, owner: &str) -> u64 {
        s.wishlists.iter().find(|w| w.owner == owner).map(|w| w.id).unwrap()
    }

    fn my_list(s: &LiyuState) -> u64 {
        s.wishlists.iter().find(|w| w.is_mine()).map(|w| w.id).unwrap()
    }

    // ---- 日期 ----

    #[test]
    fn test_today_is_2026_09_24() {
        assert_eq!(fmt_days(T), "2026-09-24");
        assert_eq!(rel_day(T, T), "今天");
        assert_eq!(rel_day(T - 1, T), "昨天");
        assert_eq!(rel_day(T - 3, T), "9 月 21 日");
    }

    #[test]
    fn civil_round_trip() {
        for d in [-1000, 0, 19_000, T, 30_000] {
            let (y, m, dd) = days_to_civil(d);
            assert_eq!(civil_to_days(y, m, dd), d);
        }
        assert_eq!(parse_iso_days("2026-09-24"), Some(T));
        assert_eq!(parse_iso_days("09/24"), None);
    }

    // ---- 目录与金额 ----

    #[test]
    fn catalog_covers_every_category_and_wish_kind() {
        assert_eq!(CATALOG.len(), 33);
        for c in Category::ALL {
            assert!(!catalog_in(Some(c)).is_empty(), "{} 没有礼物", c.label());
        }
        assert_eq!(catalog_in(None).len(), 33);
        for k in WISH_KINDS {
            assert!(CATALOG.iter().any(|c| c.kind == k), "心愿品类「{k}」在目录里没有商品");
        }
        for c in &CATALOG {
            assert!(c.price >= 2900, "{} 太便宜", c.name);
            assert!(!c.desc.is_empty() && !c.tags.is_empty(), "{} 缺描述或标签", c.name);
        }
        assert_eq!(related_items(12, 3).len(), 3);
        assert!(related_items(12, 3).iter().all(|&j| item(j).cat == Category::Digital));
        assert_eq!(item(0).price, 10900);
        assert_eq!(item(999).name, CATALOG[0].name, "越界退回第一件");
    }

    #[test]
    fn fee_rules() {
        // ¥109 折现：8.72 → 9，退 ¥100。
        assert_eq!(cashout_quote(10900), (900, 10000));
        // ¥109 换购：5.45 → 6，抵扣 ¥103。
        assert_eq!(exchange_credit(10900), (600, 10300));
        // ¥10 的手续费最低 ¥1。
        assert_eq!(fee(1000, CASHOUT_FEE_PCT), 100);
        assert_eq!(fee(1000, EXCHANGE_FEE_PCT), 100);
        // 正好整元不多收。
        assert_eq!(fee(10000, 8), 800);
    }

    #[test]
    fn yuan_formatting() {
        assert_eq!(yuan(10900), "¥109");
        assert_eq!(yuan(872), "¥8.72");
        assert_eq!(yuan(-600), "-¥6");
        assert_eq!(yuan(5), "¥0.05");
    }

    #[test]
    fn best_item_within_budget() {
        assert_eq!(best_item_within(10000), Some(11)); // 向日葵 ¥99
        assert_eq!(best_item_within(2900), Some(2));
        assert_eq!(best_item_within(2899), None);
    }

    // ---- 解谜 ----

    #[test]
    fn answers_are_normalized() {
        assert!(answer_matches("星际穿越", " 星际 穿越！", false));
        assert!(answer_matches("Interstellar", "interstellar.", false));
        assert!(answer_matches("ABC123", "ａｂｃ１２３", false), "全角转半角");
        assert!(answer_matches("月亮不睡我不睡", "月亮不睡，我不睡。", false));
        assert!(!answer_matches("星际穿越", "", false));
        assert!(!answer_matches("星际穿越", " ！ ", false));
    }

    #[test]
    fn guess_who_accepts_any_alias() {
        assert_eq!(split_aliases("林舟 / 舟舟、阿舟，Zhou"), vec!["林舟", "舟舟", "阿舟", "Zhou"]);
        assert!(answer_matches("林舟/舟舟", "林舟", true));
        assert!(answer_matches("林舟/舟舟", "舟舟", true));
        assert!(!answer_matches("林舟/舟舟", "陈晓", true));
    }

    #[test]
    fn right_answer_reveals_with_identity() {
        let mut s = LiyuState::for_tests();
        let id = received_by_unlock(&s, Unlock::GuessWho);
        assert_eq!(s.open(id, T), Some(GiftState::Opened));
        assert_eq!(s.submit_answer(id, "舟舟", T), AnswerOutcome::Right);
        let g = s.gift(id).unwrap();
        assert_eq!(g.state(), GiftState::Revealed);
        assert!(g.identity_known);
        assert_eq!(g.shown_sender(), "林舟");
        assert_eq!(g.title(), "三顿半精品咖啡礼盒");
    }

    #[test]
    fn three_wrong_answers_reveal_without_identity() {
        let mut s = LiyuState::for_tests();
        let id = received_by_unlock(&s, Unlock::Question);
        s.open(id, T);
        assert_eq!(s.submit_answer(id, "   ", T), AnswerOutcome::Empty);
        assert_eq!(s.gift(id).unwrap().attempts, 0, "空答案不扣机会");
        assert_eq!(s.submit_answer(id, "泰坦尼克号", T), AnswerOutcome::Wrong { left: 2 });
        assert_eq!(s.submit_answer(id, "盗梦空间", T), AnswerOutcome::Wrong { left: 1 });
        assert_eq!(s.submit_answer(id, "阿凡达", T), AnswerOutcome::Exhausted);
        let g = s.gift(id).unwrap();
        assert_eq!(g.state(), GiftState::Revealed);
        assert!(!g.identity_known);
        assert_eq!(g.shown_sender(), MYSTERY_FRIEND);
        // 揭晓后再答不算。
        assert_eq!(s.submit_answer(id, "星际穿越", T), AnswerOutcome::NotOpen);
    }

    #[test]
    fn unrevealed_gift_hides_everything() {
        let s = LiyuState::for_tests();
        let id = received_by_unlock(&s, Unlock::GuessWho);
        let g = s.gift(id).unwrap();
        assert_eq!(g.title(), MYSTERY_GIFT);
        assert_eq!(g.shown_sender(), MYSTERY_FRIEND);
        // 熟人统计也不能把没揭晓的礼物算到林舟头上。
        assert_eq!(s.gift_counts("林舟").1, 0);
    }

    #[test]
    fn free_gift_opens_straight_to_reveal() {
        let mut s = LiyuState::for_tests();
        let mut g = s.new_gift(DIR_RECEIVED, 3, T);
        g.peer = "陈晓".into();
        let id = g.id;
        s.gifts.push(g);
        assert_eq!(s.open(id, T), Some(GiftState::Revealed));
        assert!(s.gift(id).unwrap().identity_known);
    }

    #[test]
    fn candidates_include_real_sender_and_are_stable() {
        let s = LiyuState::for_tests();
        let id = received_by_unlock(&s, Unlock::GuessWho);
        let g = s.gift(id).unwrap();
        let a = s.guess_candidates(g);
        assert_eq!(a.len(), CANDIDATE_COUNT);
        assert!(a.contains(&"林舟".to_string()));
        assert!(!a.contains(&"舟舟".to_string()), "别名不另占一个位置");
        let mut uniq = a.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), a.len(), "候选不重复");
        assert_eq!(a, s.guess_candidates(g), "同一份礼物顺序一致");
    }

    // ---- 契约 ----

    #[test]
    fn pact_validation() {
        assert_eq!(validate_pact("  下周请我喝咖啡 ").unwrap(), "下周请我喝咖啡");
        assert!(validate_pact("").is_err());
        let long: String = "约".repeat(25);
        assert_eq!(validate_pact(&long), Err("契约最多 24 个字"));
        assert!(validate_pact(&"约".repeat(24)).is_ok());
        assert_eq!(validate_pact("收下要给我发个红包"), Err("契约只写轻约定，不涉及钱"));
        for (_, text) in PACT_PRESETS {
            assert!(validate_pact(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn accept_with_contract_needs_agreement_and_creates_pact() {
        let mut s = LiyuState::for_tests();
        let id = received_by_unlock(&s, Unlock::Question);
        s.open(id, T);
        s.submit_answer(id, "星际穿越", T);
        let pacts = s.pacts.len();
        assert_eq!(s.accept(id, &AcceptForm::default(), T), Err("先打开「我同意」，才能收下"));
        let f = AcceptForm { agree: true, ..Default::default() };
        s.accept(id, &f, T).unwrap();
        let g = s.gift(id).unwrap();
        assert_eq!(g.state(), GiftState::Accepted);
        assert!(!g.voucher.is_empty(), "电子券收下就有券码");
        assert_eq!(s.pacts.len(), pacts + 1);
        let p = s.pacts.last().unwrap();
        assert!(p.mine);
        assert_eq!(p.peer, "陈晓");
        assert_eq!(p.due_on, T + PACT_DAYS);
    }

    #[test]
    fn contract_agreement_reveals_hidden_sender() {
        let mut s = LiyuState::for_tests();
        let id = received_by_unlock(&s, Unlock::GuessWho);
        s.open(id, T);
        for w in ["a", "b", "c"] {
            s.submit_answer(id, w, T);
        }
        assert!(!s.gift(id).unwrap().identity_known);
        let f = AcceptForm {
            agree: true,
            name: "阿岚".into(),
            phone: "13800138000".into(),
            addr: "上海市徐汇区某路 1 号".into(),
        };
        s.accept(id, &f, T).unwrap();
        assert!(s.gift(id).unwrap().identity_known, "同意契约即揭晓");
        assert_eq!(s.settings.ship_phone, "13800138000", "地址记住，下次预填");
    }

    #[test]
    fn physical_gift_needs_valid_address() {
        let mut s = LiyuState::for_tests();
        let id = received_by_unlock(&s, Unlock::Passphrase);
        s.open(id, T);
        assert_eq!(s.submit_answer(id, "月亮不睡 我不睡！", T), AnswerOutcome::Right);
        let mut f = AcceptForm { agree: true, name: "阿岚".into(), phone: "1380013800".into(), addr: "某地".into() };
        assert_eq!(s.accept(id, &f, T), Err("手机号要 11 位数字"));
        f.phone = "13800138000".into();
        f.addr = " ".into();
        assert_eq!(s.accept(id, &f, T), Err("收件地址还没填"));
        f.addr = "某地".into();
        s.accept(id, &f, T).unwrap();
        assert!(s.gift(id).unwrap().voucher.is_empty(), "实物没有券码");
    }

    #[test]
    fn pact_actions() {
        let mut s = LiyuState::for_tests();
        let mine = s.pacts_of(true)[0].id;
        let theirs = s.pacts_of(false)[0].id;
        assert_eq!(s.waive_pact(mine), Err("答应了就是答应了"));
        assert!(s.nudge_pact(mine, T).is_err());
        s.nudge_pact(theirs, T).unwrap();
        assert_eq!(s.nudge_pact(theirs, T), Err("今天已经提醒过啦"));
        s.nudge_pact(theirs, T + 1).unwrap();
        s.waive_pact(theirs).unwrap();
        assert_eq!(s.pact(theirs).unwrap().state(), PactState::Waived);
        s.fulfil_pact(mine).unwrap();
        assert_eq!(s.pact(mine).unwrap().due_text(T), "已兑现");
        assert!(s.fulfil_pact(mine).is_err());
    }

    #[test]
    fn pact_due_text() {
        let s = LiyuState::for_tests();
        let p = s.pacts_of(true)[0];
        assert_eq!(p.due_text(T), "还有 1 天");
        assert_eq!(p.due_text(T + 1), "今天到期");
        assert_eq!(p.due_text(T + 3), "已逾期 2 天");
    }

    // ---- 折现 / 换购 / 余额 ----

    fn reveal(s: &mut LiyuState, u: Unlock) -> u64 {
        let id = received_by_unlock(s, u);
        s.open(id, T);
        let ans = s.gift(id).unwrap().expected_answer().to_string();
        let ans = split_aliases(&ans).remove(0);
        assert_eq!(s.submit_answer(id, &ans, T), AnswerOutcome::Right);
        id
    }

    #[test]
    fn cash_out_refunds_minus_fee_and_voids_contract() {
        let mut s = LiyuState::for_tests();
        let before = s.balance();
        let pacts = s.pacts.len();
        let id = reveal(&mut s, Unlock::GuessWho); // ¥109
        assert_eq!(s.cash_out(id, T), Ok(10000));
        assert_eq!(s.balance(), before + 10000);
        assert_eq!(s.pacts.len(), pacts, "折现不生成契约");
        assert_eq!(s.gift(id).unwrap().status_text(T), "已折现 ¥100");
        assert!(s.cash_out(id, T).is_err(), "不能折两次");
        assert!(s.accept(id, &AcceptForm::default(), T).is_err());
    }

    #[test]
    fn exchange_refunds_or_charges_difference() {
        // 更便宜：¥109 → 抵扣 ¥103 → 换 ¥35 → 退 ¥68。
        let mut s = LiyuState::for_tests();
        let id = reveal(&mut s, Unlock::GuessWho);
        let before = s.balance();
        let r = s.exchange(id, 1, &AcceptForm::default(), T).unwrap();
        assert_eq!(r, ExchangeResult { diff: 6800, from_balance: 0, external: 0 });
        assert_eq!(s.balance(), before + 6800);
        let g = s.gift(id).unwrap();
        assert_eq!(g.state(), GiftState::Exchanged);
        assert_eq!(g.final_item().name, "星巴克中杯拿铁电子券");
        assert!(!g.voucher.is_empty());

        // 更贵：¥98 → 抵扣 ¥93 → 换 ¥128 → 补 ¥35，余额 0 时全部模拟支付。
        let mut s = LiyuState::for_tests();
        assert_eq!(s.balance(), 0);
        let id = reveal(&mut s, Unlock::Question);
        let r = s.exchange(id, 10, &AcceptForm::default(), T).unwrap();
        assert_eq!(r, ExchangeResult { diff: -3500, from_balance: 0, external: 3500 });
        assert_eq!(s.balance(), 0, "余额永不为负");

        // 余额部分够：先扣余额，不够的模拟支付。
        let mut s = LiyuState::for_tests();
        s.push_ledger(T, 2000, 0, "测试", "");
        let id = reveal(&mut s, Unlock::Question);
        let r = s.exchange(id, 10, &AcceptForm::default(), T).unwrap();
        assert_eq!(r, ExchangeResult { diff: -3500, from_balance: 2000, external: 1500 });
        assert_eq!(s.balance(), 0);
    }

    #[test]
    fn exchange_to_physical_needs_address() {
        let mut s = LiyuState::for_tests();
        let id = reveal(&mut s, Unlock::Question);
        assert_eq!(s.exchange(id, 4, &AcceptForm::default(), T), Err("换一件不一样的吧"));
        assert_eq!(s.exchange(id, 11, &AcceptForm::default(), T), Err("收件人还没填"));
        assert_eq!(s.exchange_options(s.gift(id).unwrap()).len(), 11);
    }

    #[test]
    fn send_uses_balance_first_then_simulated_payment() {
        let mut s = LiyuState::for_tests();
        s.top_up(T);
        assert_eq!(s.balance(), TOP_UP_AMOUNT);
        let id = s.send_gift(&draft(0), T).unwrap(); // ¥109
        assert_eq!(s.balance(), 0);
        let e = s.ledger.last().unwrap();
        assert_eq!((e.amount, e.external), (-5000, 5900));
        assert_eq!(e.note, "余额抵 ¥50 · 模拟支付 ¥59");
        let g = s.gift(id).unwrap();
        assert!(g.is_sent());
        assert_eq!(g.state(), GiftState::Sealed);

        // 关掉余额抵扣：全部模拟支付。
        s.top_up(T);
        let mut d = draft(2);
        d.use_balance = false;
        s.send_gift(&d, T).unwrap();
        assert_eq!(s.balance(), TOP_UP_AMOUNT);
    }

    #[test]
    fn send_validation() {
        let mut s = LiyuState::for_tests();
        let mut d = draft(0);
        d.answer = "  ".into();
        assert_eq!(s.send_gift(&d, T), Err("问题的答案还没写"));
        let mut d = draft(0);
        d.unlock = Unlock::GuessWho;
        d.clue.clear();
        assert_eq!(s.validate_draft(&d), Err("写一句线索，让 TA 有迹可循"));
        let mut d = draft(0);
        d.unlock = Unlock::Passphrase;
        d.clue.clear();
        d.answer.clear();
        assert_eq!(s.validate_draft(&d), Err("暗号还没写"));
        d.answer = "芝麻开门".into();
        assert!(s.validate_draft(&d).is_ok(), "暗号提示可以不写");
        let mut d = draft(0);
        d.contract = Some("给我转账".into());
        assert_eq!(s.validate_draft(&d), Err("契约只写轻约定，不涉及钱"));
        let mut d = draft(0);
        d.message = "字".repeat(41);
        assert_eq!(s.validate_draft(&d), Err("寄语最多 40 个字"));
        let mut d = draft(0);
        d.unlock = Unlock::Free;
        d.clue.clear();
        d.answer.clear();
        assert!(s.validate_draft(&d).is_ok());
        // 猜我是谁：答案就是我的称呼。
        let mut d = draft(0);
        d.unlock = Unlock::GuessWho;
        let id = s.send_gift(&d, T).unwrap();
        assert_eq!(s.gift(id).unwrap().answer, DEFAULT_NICKNAME);
    }

    #[test]
    fn withdraw_only_while_sealed() {
        let mut s = LiyuState::for_tests();
        let id = s.send_gift(&draft(3), T).unwrap(); // ¥49 全部模拟支付
        assert_eq!(s.balance(), 0);
        s.withdraw(id, T).unwrap();
        assert_eq!(s.balance(), 4900, "撤回全额退回余额");
        assert_eq!(s.gift(id).unwrap().state(), GiftState::Withdrawn);
        assert!(s.withdraw(id, T).is_err());

        let id = s.send_gift(&draft(3), T).unwrap();
        s.simulate_step(id, T);
        assert_eq!(s.withdraw(id, T), Err("TA 已经打开了，撤不回来了"));
    }

    // ---- 过期 ----

    #[test]
    fn sweep_expires_once() {
        let mut s = LiyuState::for_tests();
        let sent = s.send_gift(&draft(0), T).unwrap();
        let passphrase = received_by_unlock(&s, Unlock::Passphrase); // today − 5
        assert_eq!(s.sweep(T), 0);
        assert_eq!(s.sweep(T + 2), 1, "收到的暗号礼物第 7 天过期");
        assert_eq!(s.gift(passphrase).unwrap().status_text(T + 2), "已过期，已退回给 TA");
        let before = s.balance();
        let n = s.sweep(T + EXPIRE_DAYS);
        assert!(n >= 1);
        assert_eq!(s.gift(sent).unwrap().state(), GiftState::Expired);
        assert_eq!(s.balance(), before + 10900, "送出的过期全额退回");
        let after = s.balance();
        assert_eq!(s.sweep(T + EXPIRE_DAYS), 0, "幂等");
        assert_eq!(s.sweep(T + 30), 0);
        assert_eq!(s.balance(), after, "只退一次");
    }

    // ---- 模拟器 ----

    #[test]
    fn simulator_reaches_terminal_in_three_steps() {
        let mut s = LiyuState::for_tests();
        for (k, unlock) in [Unlock::GuessWho, Unlock::Question, Unlock::Passphrase, Unlock::Free]
            .into_iter()
            .cycle()
            .take(24)
            .enumerate()
        {
            let mut d = draft((k % 12) as u16);
            d.unlock = unlock;
            d.contract = (k % 2 == 0).then(|| "周末陪我看一场电影".to_string());
            if unlock == Unlock::Passphrase {
                d.answer = "芝麻开门".into();
            }
            let id = s.send_gift(&d, T).unwrap();
            let mut steps = 0;
            while s.simulate_step(id, T).is_some() {
                steps += 1;
                assert!(steps <= 3, "礼物 {id} 超过 3 步");
            }
            assert!(s.gift(id).unwrap().state().is_terminal());
            assert_eq!(s.simulate_step(id, T), None, "终态后不动");
        }
    }

    #[test]
    fn simulator_is_deterministic_and_covers_outcomes() {
        let run = || {
            let mut s = LiyuState::for_tests();
            let mut outcomes = Vec::new();
            for k in 0..12u16 {
                let mut d = draft(k);
                d.contract = Some("见面先给我一个拥抱".into());
                let id = s.send_gift(&d, T).unwrap();
                while let Some(step) = s.simulate_step(id, T) {
                    outcomes.push(step);
                }
            }
            (outcomes, s.gifts.len(), s.pacts.len())
        };
        let a = run();
        assert_eq!(a, run());
        let has = |f: &dyn Fn(&SimStep) -> bool| a.0.iter().any(f);
        assert!(has(&|s| matches!(s, SimStep::Accepted { pact: true })));
        assert!(has(&|s| matches!(s, SimStep::Exchanged)));
        assert!(has(&|s| matches!(s, SimStep::CashedOut { .. })));
        assert!(has(&|s| matches!(s, SimStep::Revealed { known: false })));
    }

    #[test]
    fn simulator_return_gift_fits_refund() {
        let mut s = LiyuState::for_tests();
        let mut found = false;
        for k in 0..40 {
            let id = s.send_gift(&draft((k % 12) as u16), T).unwrap();
            let before = s.gifts.len();
            let mut last = None;
            while let Some(step) = s.simulate_step(id, T) {
                last = Some(step);
            }
            if last == Some(SimStep::CashedOut { returned: true }) {
                assert_eq!(s.gifts.len(), before + 1);
                let r = s.gifts.last().unwrap();
                let refund = s.gift(id).unwrap().refund;
                assert!(!r.is_sent());
                assert_eq!(r.unlock(), Unlock::Free);
                assert_eq!(r.message, RETURN_MESSAGE);
                assert!(r.price <= refund);
                assert_eq!(r.peer, "林舟");
                found = true;
            }
        }
        assert!(found, "40 份里应该至少有一份回礼");
    }

    #[test]
    fn simulator_ignores_received_gifts() {
        let mut s = LiyuState::for_tests();
        let id = received_by_unlock(&s, Unlock::GuessWho);
        assert_eq!(s.simulate_step(id, T), None);
    }

    // ---- 通知 ----

    #[test]
    fn demo_triggers_both_notices() {
        let mut s = LiyuState::for_tests();
        let n = s.due_notices(T);
        assert_eq!(n.len(), 3);
        assert_eq!(n[0].kind, NoticeKind::GiftExpiring);
        assert_eq!(n[0].text, "有一份神秘礼物还没拆，2 天后会退回给 TA");
        assert_eq!(n[1].kind, NoticeKind::PactDue);
        assert!(n[1].text.contains("明天到期"), "{}", n[1].text);
        assert_eq!(n[2].kind, NoticeKind::WishSoon);
        assert_eq!(n[2].text, "林舟的生日还有 3 天，心愿单上还有 4 件没人送");
        assert_eq!(n[2].target, friend_list(&s, "林舟"));
        s.settings.notify_gift = false;
        s.settings.notify_pact = false;
        s.settings.notify_wish = false;
        assert!(s.due_notices(T).is_empty(), "开关是唯一的闸");
    }

    #[test]
    fn notices_never_name_the_sender() {
        let s = LiyuState::for_tests();
        for n in s.due_notices(T) {
            if n.kind == NoticeKind::GiftExpiring {
                assert!(!n.text.contains("许宁"), "{}", n.text);
            }
        }
    }

    // ---- 熟人 ----

    #[test]
    fn contacts_add_remove_restore() {
        let mut s = LiyuState::for_tests();
        assert_eq!(s.add_contact(" "), Err(AddContactError::Empty));
        assert_eq!(s.add_contact("林舟"), Err(AddContactError::Duplicate));
        assert_eq!(s.add_contact(&"长".repeat(17)), Err(AddContactError::TooLong));
        let id = s.add_contact("周子墨").unwrap();
        assert!(!s.directory.contains(&"周子墨".to_string()));
        let snap = s.remove_contact(id).unwrap();
        assert!(s.contact(id).is_none());
        s.restore(snap);
        assert!(s.contact(id).is_some());
        assert_eq!(s.adopt_names(vec!["林小满".into(), "林舟".into(), "".into()]), 1);
    }

    #[test]
    fn gift_counts_per_contact() {
        let s = LiyuState::for_tests();
        // 许宁：送过一份香薰蜡烛；收到的暗号礼物还没拆，不算。
        assert_eq!(s.gift_counts("许宁"), (1, 0));
        // 陈晓：收到一份喜茶（已收下）；电影票还没拆。
        assert_eq!(s.gift_counts("陈晓"), (0, 1));
        assert_eq!(s.gift_counts("林舟"), (1, 0));
    }

    // ---- 持久化 ----

    #[test]
    fn serde_round_trip() {
        let mut s = LiyuState::for_tests();
        let id = reveal(&mut s, Unlock::GuessWho);
        s.cash_out(id, T).unwrap();
        s.settings.set_theme_mode(crate::theme::ThemeMode::default());
        let json = s.persisted().serialize_json();
        let back = PersistedState::deserialize_json_lenient(&json).unwrap();
        assert_eq!(back, s.persisted());
        let mut t = LiyuState::demo(T);
        t.apply_persisted(back);
        assert_eq!(t.gifts, s.gifts);
        assert_eq!(t.balance(), s.balance());
        assert_eq!(t.next_id, s.next_id);
    }

    #[test]
    fn save_and_load_file() {
        let dir = std::env::temp_dir().join(format!("liyu-test-{}", std::process::id()));
        let path = dir.join("state.json");
        let s = LiyuState::for_tests();
        s.save_to(&path).unwrap();
        let p = LiyuState::load_from(&path).expect("能读回来");
        assert_eq!(p.gifts.as_ref().map(|g| g.len()), Some(s.gifts.len()));
        // 不是礼遇格式的文件（旧版偶遇存档）算读不出来。
        std::fs::write(&path, r#"{"contacts":[{"id":0,"label":"林舟"}],"encounters":[]}"#).unwrap();
        assert!(LiyuState::load_from(&path).is_none());
        std::fs::write(&path, "not json").unwrap();
        assert!(LiyuState::load_from(&path).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn next_id_never_collides_after_load() {
        let s = LiyuState::for_tests();
        let mut p = s.persisted();
        p.next_id = Some(1);
        let mut t = LiyuState::demo(T);
        t.apply_persisted(p);
        let id = t.send_gift(&draft(0), T).unwrap();
        assert_eq!(t.gifts.iter().filter(|g| g.id == id).count(), 1);
    }

    #[test]
    fn reset_demo_keeps_preferences() {
        let mut s = LiyuState::for_tests();
        s.settings.onboarded = true;
        s.settings.theme = Some("light".into());
        s.top_up(T);
        s.reset_demo(T);
        assert_eq!(s.balance(), 0);
        assert!(s.settings.onboarded);
        assert_eq!(s.settings.theme.as_deref(), Some("light"));
    }

    // ---- 状态文案与时间线 ----

    #[test]
    fn status_texts() {
        let s = LiyuState::for_tests();
        let texts: Vec<String> = s.received(T).iter().map(|g| g.status_text(T)).collect();
        assert!(texts.contains(&"待拆 · 还剩 6 天".to_string()), "{texts:?}");
        assert!(texts.contains(&"已收下".to_string()));
        let sent: Vec<String> = s.sent().iter().map(|g| g.status_text(T)).collect();
        assert_eq!(sent, vec!["已揭晓 · 等 TA 决定", "TA 收下了"]);
    }

    #[test]
    fn timeline_is_short_and_ordered() {
        let mut s = LiyuState::for_tests();
        let mut d = draft(0);
        d.contract = Some("周末陪我看一场电影".into());
        let id = s.send_gift(&d, T).unwrap();
        while s.simulate_step(id, T).is_some() {}
        let t = s.gift(id).unwrap().timeline(T);
        assert!(t.len() >= 3 && t.len() <= 5, "{t:?}");
        assert!(t[0].1.starts_with("送出礼卡"));
    }

    #[test]
    fn alpha_key_groups_names() {
        assert_eq!(alpha_key("林舟"), 'L');
        assert_eq!(alpha_key("老陈"), 'C');
        assert_eq!(alpha_key("Anna"), 'A');
        assert_eq!(alpha_key("？"), '#');
    }

    #[test]
    fn vcard_parsing() {
        assert_eq!(parse_vcard(SAMPLE_VCARD), vec!["周子墨", "林小满", "黄一诺", "吴凯文"]);
    }

    // ---- 心愿单 ----

    #[test]
    fn vague_wish_picks_matching_goods() {
        let ids = |w: &WishItem| wish_candidates(w).iter().map(|m| m.item).collect::<Vec<_>>();
        // 55 寸以上、¥3000 以内：43 寸的不要，贵的 65/75 寸超预算；剩下的便宜的在前。
        let tv = WishItem::vague("电视", 300_000, "55 寸以上");
        assert_eq!(ids(&tv), vec![31, 12, 32]);
        assert_eq!(wish_candidates(&tv)[0].reason(), "符合 55寸 · 预算内");
        assert_eq!(wish_candidates(&tv)[2].reason(), "符合 65寸 · 预算内");
        // 不限预算、65 寸以上还要 120Hz：两台都命中的排前面（便宜的在前），只够尺寸的排后面。
        let tv = WishItem::vague("电视", 0, "65寸以上 120Hz");
        assert_eq!(ids(&tv), vec![13, 15, 32]);
        assert!(wish_candidates(&tv)[..2].iter().all(|m| m.hits.contains(&"120Hz".to_string())));
        // 什么要求都没写：同类全给，便宜的在前。
        assert_eq!(ids(&WishItem::vague("绿植", 0, "")), vec![25, 24]);
        // 命中要求多的排前面。
        assert_eq!(ids(&WishItem::vague("绿植", 0, "大盆 好养"))[0], 24);
        // 预算内一件都没有：放宽预算，标「超出预算」。
        let ear = WishItem::vague("耳机", 10_000, "降噪");
        let m = wish_candidates(&ear);
        assert_eq!(m.len(), 1);
        assert!(!m[0].within);
        assert_eq!(m[0].reason(), "符合 降噪 · 超出预算");
        // 具体的心愿只有那一件。
        assert_eq!(ids(&WishItem::exact(6)), vec![6]);
        assert_eq!(WishItem::vague("电视", 0, "").title(), "一台电视");
        assert_eq!(WishItem::vague("耳机", 100_000, "降噪").sub(), "¥1000 以内 · 降噪");
    }

    #[test]
    fn demo_wishlists_are_sorted_and_described() {
        let s = LiyuState::for_tests();
        let f: Vec<&str> = s.friend_wishlists(T).iter().map(|w| w.owner.as_str()).collect();
        assert_eq!(f, vec!["林舟", "陈晓", "许宁"], "进行中的按日子近的在前，结束的在后");
        let lin = s.wishlist(friend_list(&s, "林舟")).unwrap();
        assert_eq!(lin.when_text(T), format!("{} · 还有 3 天", {
            let (_, m, d) = days_to_civil(T + 3);
            format!("{m} 月 {d} 日")
        }));
        assert_eq!(lin.progress_text(), "5 件心愿 · 1 件已有人送");
        assert_eq!(s.wish_item_status(lin, 0, T), "还没人送");
        assert_eq!(s.wish_item_status(lin, 1, T), "已有人送");
        let xu = s.wishlist(friend_list(&s, "许宁")).unwrap();
        assert!(!xu.is_open(T));
        assert_eq!(xu.countdown(T), "已结束");
        assert_eq!(s.wish_hint("陈晓", T).as_deref(), Some("乔迁心愿单 · 还有 12 天 · 3 件没人送"));
        assert_eq!(s.wish_hint("许宁", T), None, "结束了的不提");
        // 我的心愿单：林舟那份咖啡还没拆，不透露是谁。
        let mine = s.wishlist(my_list(&s)).unwrap();
        assert_eq!(s.wish_item_status(mine, 0, T), "已被认领 · 礼物在礼盒里等你拆");
        assert_eq!(s.wish_item_status(mine, 1, T), "还没人认领");
        assert_eq!(s.current_my_wishlist(T), Some(mine.id));
    }

    #[test]
    fn sending_from_a_wishlist_claims_the_item() {
        let mut s = LiyuState::for_tests();
        let lid = friend_list(&s, "林舟");
        let mut d = draft(17);
        d.peer = String::new();
        d.wish = Some((lid, 0));
        d.pay = Some(1);
        let before = s.ledger.len();
        let id = s.send_gift(&d, T).unwrap();
        let g = s.gift(id).unwrap();
        assert_eq!(g.peer, "林舟", "收礼人就是心愿单的主人");
        assert_eq!(g.wish_id, Some(lid));
        assert_eq!(s.ledger.len(), before + 1);
        assert!(s.ledger.last().unwrap().note.contains("支付宝"), "{}", s.ledger.last().unwrap().note);
        let w = s.wishlist(lid).unwrap();
        assert_eq!(w.items[0].state, WISH_BY_ME);
        assert_eq!(w.items[0].gift_id, id);
        assert_eq!(s.wish_item_status(w, 0, T), "你送的");
        // 同一件不能再认领；已被别人认领的也不行。
        assert_eq!(s.send_gift(&d, T), Err("这件已经有人送了"));
        d.wish = Some((lid, 1));
        d.item = 0;
        assert_eq!(s.send_gift(&d, T), Err("这件已经有人送了"));
        // 具体的心愿要同一件；含糊的同类就行。
        d.wish = Some((lid, 3));
        d.item = 1;
        assert_eq!(s.send_gift(&d, T), Err("心愿单上要的是另一件"));
        d.wish = Some((lid, 2));
        d.item = 1;
        assert_eq!(s.send_gift(&d, T), Err("要送心愿单上说的那一类"));
        d.item = 9;
        assert!(s.send_gift(&d, T).is_ok(), "文具盲盒也是盲盒，超出「潮玩」要求也能送");
        // 撤回：心愿放开，别人还能送。
        s.withdraw(id, T).unwrap();
        assert_eq!(s.wishlist(lid).unwrap().items[0].state, WISH_OPEN);
        // 自己的、结束了的心愿单都不能认领。
        let mut d = draft(11);
        d.wish = Some((my_list(&s), 2));
        assert_eq!(s.send_gift(&d, T), Err("这是你自己的心愿单"));
        d.wish = Some((friend_list(&s, "许宁"), 2));
        d.item = 7;
        assert_eq!(s.send_gift(&d, T), Err("心愿单已经结束了"));
    }

    #[test]
    fn wish_gift_can_be_scheduled_for_the_day() {
        let mut s = LiyuState::for_tests();
        let lid = friend_list(&s, "陈晓");
        let mut d = draft(12);
        d.wish = Some((lid, 0));
        d.deliver_on = Some(T + 13);
        assert_eq!(s.send_gift(&d, T), Err("送到的日子不能晚于 TA 的日子"));
        d.deliver_on = Some(T + 12);
        let id = s.send_gift(&d, T).unwrap();
        let g = s.gift(id).unwrap();
        assert!(g.is_scheduled(T));
        assert_eq!(g.booked_on, Some(T));
        assert!(g.status_text(T).starts_with("待送达"), "{}", g.status_text(T));
        assert_eq!(g.timeline(T).len(), 1, "还没送到，只有下单");
        // 过了 7 天也不会过期：过期从送到那天算。
        s.sweep(T + 8);
        assert_eq!(s.gift(id).unwrap().state(), GiftState::Sealed);
        // 演示模拟器：先把它提前送到。
        assert_eq!(s.simulate_step(id, T), Some(SimStep::Delivered));
        assert!(!s.gift(id).unwrap().is_scheduled(T));
        assert_eq!(s.gift(id).unwrap().timeline(T).len(), 2);
        // 不是心愿单的礼物不能约日子。
        let mut d = draft(0);
        d.deliver_on = Some(T + 3);
        assert_eq!(s.send_gift(&d, T), Err("只有心愿单上的礼物能约好日子送"));
    }

    #[test]
    fn expiry_releases_the_wish() {
        let mut s = LiyuState::for_tests();
        let lid = friend_list(&s, "陈晓");
        let mut d = draft(22);
        d.wish = Some((lid, 4));
        s.send_gift(&d, T).unwrap();
        assert!(s.sweep(T + EXPIRE_DAYS) >= 1);
        assert_eq!(s.wishlist(lid).unwrap().items[4].state, WISH_OPEN, "没拆开退回了，心愿放开");
    }

    #[test]
    fn publish_edit_close_delete() {
        let mut s = LiyuState::for_tests();
        let mut d = WishDraft::new(T);
        d.occasion = 2;
        assert_eq!(s.publish_wish(&d, T), Err("至少放一件心愿"));
        d.items.push(WishItem::vague("", 0, ""));
        assert_eq!(s.publish_wish(&d, T), Err("选一下想要的是什么"));
        d.items[0] = WishItem::vague("电视", 300_000, "55 寸以上");
        d.items.push(WishItem::exact(19));
        d.items.push(WishItem::exact(19));
        assert_eq!(s.publish_wish(&d, T), Err("同一件放了两次"));
        d.items.pop();
        d.event_on = T - 1;
        assert_eq!(s.publish_wish(&d, T), Err("日子不能早于今天"));
        d.event_on = T + WISH_MAX_AHEAD_DAYS + 1;
        assert_eq!(s.publish_wish(&d, T), Err("最多提前半年发布"));
        d.event_on = T + 30;
        d.title = "这是一个特别特别特别特别长的标题呀".into();
        assert_eq!(s.publish_wish(&d, T), Err("标题最多 16 个字"));
        d.title.clear();
        let id = s.publish_wish(&d, T).unwrap();
        let w = s.wishlist(id).unwrap();
        assert_eq!(w.title, "阿岚的乔迁心愿单", "没写标题用默认的");
        assert!(w.is_mine());
        // 加一件：重复的不行。
        assert_eq!(s.add_to_wishlist(id, 19, T), Err("已经在心愿单上了"));
        s.add_to_wishlist(id, 23, T).unwrap();
        assert_eq!(s.wishlist(id).unwrap().items.len(), 3);
        // 没人认领的可以删。
        s.delete_wish(id).unwrap();
        assert!(s.wishlist(id).is_none());
        // 有人认领的：只能结束，编辑时也不能拿掉认领的那件。
        let mine = my_list(&s);
        assert_eq!(s.delete_wish(mine), Err("已经有人认领了，只能结束，不能删除"));
        let mut e = WishDraft::from_list(s.wishlist(mine).unwrap());
        e.items.remove(0);
        assert_eq!(s.publish_wish(&e, T), Err("已经有人认领的心愿不能删"));
        let mut e = WishDraft::from_list(s.wishlist(mine).unwrap());
        e.items.remove(2);
        e.note = "改一改".into();
        s.publish_wish(&e, T).expect("日子没改，过了也能保存");
        assert_eq!(s.wishlist(mine).unwrap().items.len(), 2);
        s.close_wish(mine, T).unwrap();
        assert!(!s.wishlist(mine).unwrap().is_open(T));
        assert_eq!(s.close_wish(mine, T), Err("这张心愿单已经结束了"));
        assert_eq!(s.current_my_wishlist(T), None);
        // 好友的心愿单不能动。
        let lin = friend_list(&s, "林舟");
        assert!(s.delete_wish(lin).is_err());
        assert!(s.close_wish(lin, T).is_err());
    }

    #[test]
    fn friend_claims_my_wish_in_the_demo() {
        let mut s = LiyuState::for_tests();
        // 日子已经过了：礼物马上进礼盒。
        let mine = my_list(&s);
        let n = s.received(T).len();
        let msg = s.simulate_wish_claim(mine, T).unwrap();
        assert!(msg.contains("一副耳机"), "{msg}");
        assert_eq!(s.received(T).len(), n + 1);
        let w = s.wishlist(mine).unwrap();
        assert_eq!(w.items[1].state, WISH_BY_OTHERS);
        assert_eq!(w.items[1].given_item, 17, "按「降噪」挑中了降噪耳机");
        assert_eq!(s.wish_item_status(w, 1, T), "已被认领 · 礼物在礼盒里等你拆");
        let g = s.gift(w.items[1].gift_id).unwrap();
        assert_eq!(g.title(), MYSTERY_GIFT, "没拆开之前不知道是什么");
        // 日子还没到：约在那天送到，之前礼盒里看不到。
        let mut d = WishDraft::new(T);
        d.event_on = T + 5;
        d.items.push(WishItem::exact(23));
        let id = s.publish_wish(&d, T).unwrap();
        let n = s.received(T).len();
        s.simulate_wish_claim(id, T).unwrap();
        assert_eq!(s.received(T).len(), n);
        assert_eq!(s.received(T + 5).len(), n + 1);
        let w = s.wishlist(id).unwrap();
        assert_eq!(s.wish_item_status(w, 0, T), format!("已被认领 · {} 送到", md_cn(T + 5)));
        assert_eq!(s.simulate_wish_claim(id, T), Err("心愿都有人送啦"));
    }

    #[test]
    fn old_versions_are_not_read() {
        let dir = std::env::temp_dir().join(format!("liyu-state-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        std::fs::write(&path, r#"{"version":3,"gifts":[],"next_id":3}"#).unwrap();
        assert!(LiyuState::load_from(&path).is_none(), "旧版本不迁移");
        std::fs::write(&path, LiyuState::for_tests().persisted().serialize_json()).unwrap();
        assert!(LiyuState::load_from(&path).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wishlists_survive_save() {
        let mut s = LiyuState::for_tests();
        let mut d = WishDraft::new(T);
        d.items.push(WishItem::vague("鲜花", 20_000, "向日葵"));
        let id = s.publish_wish(&d, T).unwrap();
        let text = s.persisted().serialize_json();
        let p = PersistedState::deserialize_json_lenient(&text).unwrap();
        let mut t = LiyuState::for_tests();
        t.apply_persisted(p);
        assert_eq!(t.wishlist(id), s.wishlist(id));
        assert!(t.take_id() > id);
    }
}
