//! 偶遇 OuYu Phase 1 数据模型与纯逻辑（2026-09-17 新版设计）。
//!
//! 核心转向「匿名机会」：发布模糊去向后地图上只有匿名光圈，线下认出彼此后
//! 双方在「相遇」页现场确认（互认），可选领「相遇礼」券；每次相遇独立选择
//! 保存 / 隐藏 / 不保存。
//!
//! 持久化边界（ouyu/design/02-features.md G/H 节）：
//! - 长期 state.json 只存 contacts、encounters、directory、reward claim。
//! - 发布（Publish）与回声属于短时匹配数据，按 G 节「短期存储不入持久备份」
//!   的精神**不落盘**：只活在进程内存里，撤回 / 退出编辑 / 进程结束即清除，
//!   演示里用「到期清除」文案表达生产的到期语义。
//! - 旧版 state.json 的 my_windows / cards / stealth / my_pos / 亲密度等字段
//!   在加载时被忽略；首次保存后旧数据自然消失。
use makepad_widgets::makepad_micro_serde::*;

/// 发布可选日期的跨度：今天起一周内任一天（design/02-features.md A 节）。
pub const DAY_SPAN: usize = 7;
/// 粗时段（生产要求不短于 3 小时，演示只做选项）。
pub const SLOTS: [&str; 3] = ["上午", "下午", "晚间"];
/// 通用意愿。
pub const INTENTS: [&str; 3] = ["随意走走", "顺路办事", "就想出门"];
/// 匿名回声的固定三选（02 C 节：无自定义文字，防暗号辨认）。
pub const ECHOES: [&str; 3] = ["咖啡", "散步", "吃饭"];
/// 城市小签：不依赖他人行程的空状态趣味（01 趣味设计）。
pub const SIGNS: [&str; 4] = [
    "去一家没进过的书店，只翻三页。",
    "绕一点路，去看看树。",
    "今天的咖啡，可以慢慢喝。",
    "在公共街区走走，让生活留白。",
];
/// 匿名机会阈值（02 B 节设计起点：至少 5 位不同有效候选才显示熟人机会）。
pub const OPPORTUNITY_THRESHOLD: usize = 5;

// ---- 日期与周桶（成就统计基础，06 节）----
//
// 新记录写真实 ISO 日期（YYYY-MM-DD）。本地时区偏移没有可移植的平台 API，
// 演示统一用 UTC 日期（比本地日期最多差一天，不影响周桶演示语义）。

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

const WEEKDAY_NAMES: [&str; 7] = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];

/// 相对今天的偏移（0..=6）→ 日期条上的短标签，例如「今天」「周四」。
pub fn day_label_at(today: i64, offset: usize) -> &'static str {
    match offset {
        0 => "今天",
        1 => "明天",
        2 => "后天",
        n => WEEKDAY_NAMES[weekday(today + n as i64)],
    }
}

/// 相对今天的偏移 → 短标签（当天日期）。
pub fn day_label(offset: usize) -> &'static str {
    day_label_at(today_days(), offset)
}

/// 相对今天的偏移 → "9/17" 这样的日期数字，配在标签下面。
pub fn day_number_at(today: i64, offset: usize) -> String {
    let (_, m, d) = days_to_civil(today + offset as i64);
    format!("{}/{}", m, d)
}

/// 相对今天的偏移 → "9/17"。
pub fn day_number(offset: usize) -> String {
    day_number_at(today_days(), offset)
}

// ---- 匿名区域机会（02 B 节）----
//
// 发现页只展示**分档**，永远不展示候选人数、身份或距离；人数只是这里的内部
// 中间量，既不出现在 UI，也不进 AI 快照。演示里用 (区域 id, 绝对日期) 的确定
// 性散列代替真实匹配服务：同一天内多次进入结果一致，换一天才会变。

/// 一个片区某一天的机会分档。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OppLevel {
    /// 很可能
    Likely,
    /// 有可能
    Possible,
    /// 较少
    Few,
    /// 未达匿名保护阈值：不展示为熟人机会（02 B）。
    BelowThreshold,
}

impl OppLevel {
    pub fn label(self) -> &'static str {
        match self {
            OppLevel::Likely => "很可能",
            OppLevel::Possible => "有可能",
            OppLevel::Few => "较少",
            OppLevel::BelowThreshold => "暂不显示",
        }
    }

    /// 是否达到匿名保护阈值。
    pub fn shown(self) -> bool {
        !matches!(self, OppLevel::BelowThreshold)
    }
}

/// 某片区某天的机会。不含人数字段 —— 分档是这里唯一对外的东西。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AreaOpportunity {
    pub area: u16,
    /// 相对今天的偏移 0..=6。
    pub day: usize,
    /// 最可能的粗时段下标（SLOTS）。
    pub best_slot: Option<usize>,
    pub level: OppLevel,
}

/// 确定性散列（SplitMix64 的 finalizer），保证同一天同一片区结果稳定。
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// 片区类型在工作日 / 周末的基准权重。办公园区反着来，公园与商圈周末更旺。
fn kind_weight(kind: crate::areas::AreaKind, weekend: bool) -> u64 {
    use crate::areas::AreaKind::*;
    match kind {
        Commercial => if weekend { 120 } else { 90 },
        Park => if weekend { 110 } else { 60 },
        Culture => if weekend { 95 } else { 55 },
        Waterfront => if weekend { 90 } else { 50 },
        Transit => 85,
        Campus => 70,
        Neighborhood => 55,
        Office => if weekend { 25 } else { 95 },
    }
}

/// 内部候选数估算：只在本模块里存在，不会被 UI 或 AI 读到。
///
/// 标定的目标是让「达到阈值」保持稀有 —— 工作日全城十来个片区、周末二十几
/// 个。如果满城都是机会，分档就退化成装饰，阈值也就没有意义了。
fn candidate_count(area: &crate::areas::Area, abs_day: i64) -> usize {
    let h = mix((area.id as u64) << 32 ^ (abs_day as u64 & 0xffff_ffff));
    let w = kind_weight(area.kind, is_weekend(abs_day));
    let jitter = 30 + h % 211; // 30..240
    (w * jitter / 3000) as usize
}

fn level_of(count: usize) -> OppLevel {
    if count < OPPORTUNITY_THRESHOLD {
        OppLevel::BelowThreshold
    } else if count >= 7 {
        OppLevel::Likely
    } else if count >= 6 {
        OppLevel::Possible
    } else {
        OppLevel::Few
    }
}

/// 单个片区某天的机会，连内部候选数一起返回（候选数只用来排序，
/// 不出这个模块）。排行和逐个查询走同一条路，两边结果不会对不上。
fn opportunity_scored(a: &crate::areas::Area, today: i64, day: usize) -> (usize, AreaOpportunity) {
    let abs = today + day as i64;
    let c = candidate_count(a, abs);
    let h = mix((abs as u64).wrapping_mul(31).wrapping_add(a.id as u64));
    (
        c,
        AreaOpportunity {
            area: a.id,
            day,
            best_slot: (c >= OPPORTUNITY_THRESHOLD).then(|| (h >> 16) as usize % SLOTS.len()),
            level: level_of(c),
        },
    )
}

/// 某一天全部片区的机会排序（高 → 低）。纯函数版，测试与固定日期用。
pub fn opportunity_ranking_at(today: i64, day: usize) -> Vec<AreaOpportunity> {
    let mut out: Vec<(usize, AreaOpportunity)> = crate::areas::AREAS
        .iter()
        .map(|a| opportunity_scored(a, today, day))
        .collect();
    // 分档相同的按 id 升序，保证同一天内反复进入顺序一致。
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.area.cmp(&b.1.area)));
    out.into_iter().map(|(_, o)| o).collect()
}

/// 某一天（相对今天 0..=6）全部片区的机会排序。
pub fn opportunity_ranking(day: usize) -> Vec<AreaOpportunity> {
    opportunity_ranking_at(today_days(), day)
}

/// 单个片区某天的机会（区域选择器行内分档用）。
pub fn area_opportunity_at(today: i64, day: usize, area_id: u16) -> AreaOpportunity {
    match crate::areas::area(area_id) {
        Some(a) => opportunity_scored(a, today, day).1,
        None => AreaOpportunity {
            area: area_id,
            day,
            best_slot: None,
            level: OppLevel::BelowThreshold,
        },
    }
}

/// 一周七天的强度点（日期条上的小圆点）：当天达到阈值的片区数量。
pub fn week_intensity_at(today: i64) -> [usize; DAY_SPAN] {
    let mut out = [0usize; DAY_SPAN];
    for (d, slot) in out.iter_mut().enumerate() {
        *slot = opportunity_ranking_at(today, d)
            .iter()
            .filter(|o| o.level.shown())
            .count();
    }
    out
}

/// 一周七天的强度点。
pub fn week_intensity() -> [usize; DAY_SPAN] {
    week_intensity_at(today_days())
}

/// 一条回忆参与统计的有效日期：能解析的 ISO 日期；解析不了的旧记录归入今天。
fn effective_days(e: &EncounterLocal, today: i64) -> i64 {
    parse_iso_days(&e.date).unwrap_or(today)
}

/// 一周的次数桶：start 是这一周周一（天数）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeekBucket {
    pub start: i64,
    pub count: usize,
}

/// 成就页统计（06 节口径：只统计未隐藏且仍保存的回忆；隐藏退出统计、
/// 恢复计入、删除重算。熟人页 meeting_count 含隐藏，是两个不同口径）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AchievementStats {
    /// 记住的相遇（可见回忆条数）。
    pub remembered: usize,
    /// 有相遇的日子（不同有效日期数）。
    pub days: usize,
    /// 最近 N 周（含本周，本周在最后）的每周次数。
    pub weekly: Vec<WeekBucket>,
}

impl AchievementStats {
    /// 所选时段内的相遇总数。
    pub fn recent(&self) -> usize {
        self.weekly.iter().map(|w| w.count).sum()
    }
}

pub fn achievement_stats(encounters: &[EncounterLocal], weeks: usize, today: i64) -> AchievementStats {
    let n = weeks.max(1);
    let first = week_start(today) - 7 * (n as i64 - 1);
    let mut weekly: Vec<WeekBucket> = (0..n)
        .map(|i| WeekBucket { start: first + 7 * i as i64, count: 0 })
        .collect();
    let mut remembered = 0;
    let mut dates: Vec<i64> = Vec::new();
    for e in encounters.iter().filter(|e| !e.hidden) {
        remembered += 1;
        let d = effective_days(e, today);
        dates.push(d);
        let idx = (week_start(d) - first) / 7;
        if idx >= 0 && (idx as usize) < n {
            weekly[idx as usize].count += 1;
        }
    }
    dates.sort_unstable();
    dates.dedup();
    AchievementStats { remembered, days: dates.len(), weekly }
}

/// 一个里程碑（06 节：点亮 / 等自然发生，不用凑次数，无排名）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Milestone {
    pub title: &'static str,
    pub desc: &'static str,
    pub lit: bool,
}

/// 三个里程碑：第一次刚刚好（≥1 次）、生活有回响（≥3 次）、
/// 把日常过成故事（≥7 个不同日期）。不按签到 / 领券 / 消费发成就。
pub fn milestones(s: &AchievementStats) -> [Milestone; 3] {
    [
        Milestone { title: "第一次刚刚好", desc: "记住一次重逢", lit: s.remembered >= 1 },
        Milestone { title: "生活有回响", desc: "记住三次相遇", lit: s.remembered >= 3 },
        Milestone { title: "把日常过成故事", desc: "七个有相遇的日子", lit: s.days >= 7 },
    ]
}

/// 本机联系人：只有本机称呼，无授权 / 安装状态 / 永久回忆策略。
#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct ContactLocal {
    pub id: usize,
    pub label: String,
}

/// 一条私人相遇回忆：默认无地点、无精确时刻、无券 ID（02 F 节）。
/// 删除联系人但保留回忆时 contact_id 置 None，用 label_snapshot 独立展示。
#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct EncounterLocal {
    pub id: usize,
    pub contact_id: Option<usize>,
    pub label_snapshot: String,
    pub date: String,
    pub hidden: bool,
    pub note: String,
}

/// 一条回忆归到哪个月。回忆页的分组头用它，`2026-09-17` -> `2026 年 9 月`。
///
/// 认不出来的日期（旧数据、手填的怪字符串）归到「更早」那一组，而不是丢掉 ——
/// 分组是为了好找，不是为了筛掉谁。
pub fn month_head(date: &str) -> String {
    match parse_iso_days(date) {
        Some(d) => {
            let (y, m, _) = days_to_civil(d);
            format!("{} 年 {} 月", y, m)
        }
        None => "更早".to_string(),
    }
}

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

/// 回忆页的搜索：**只搜称呼与备注**，且**隐藏的记录不进搜索**
/// （design/02-features.md F 节：隐藏记录不进入提醒、搜索、AI 或推荐）。
///
/// 空查询返回未隐藏的全部 —— 「已隐藏」那一段由页面另外列，不从这里出。
pub fn search_memories<'a>(
    encounters: &'a [EncounterLocal],
    query: &str,
    filter: Option<&str>,
) -> Vec<&'a EncounterLocal> {
    let q = query.trim().to_lowercase();
    encounters
        .iter()
        .filter(|e| !e.hidden)
        .filter(|e| filter.is_none_or(|f| e.label_snapshot == f))
        .filter(|e| {
            q.is_empty()
                || e.label_snapshot.to_lowercase().contains(&q)
                || e.note.to_lowercase().contains(&q)
        })
        .collect()
}

/// 每次相遇的独立选择（02 F 节：默认保存，只影响本次）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoryChoice {
    #[default]
    Save,
    Hidden,
    Skip,
}

/// 发布状态：草稿 → 已发布 → 撤回 / 到期。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishStatus {
    Draft,
    Published,
    Withdrawn,
    Expired,
}

/// 模糊去向发布。每人只有一个有效发布，修改覆盖旧值。
/// 短时数据：不落盘（见模块头注释）。
#[derive(Clone, Debug, PartialEq)]
pub struct Publish {
    /// 相对今天的偏移 0..=6（一周内任一天）。
    pub day: usize,
    pub slot: usize,
    /// 区域库里的片区 id —— 不是下标，换库不会错位。
    pub area: u16,
    pub intent: usize,
    pub status: PublishStatus,
}

impl Publish {
    /// 预览 / 已发布的展示文案：无昵称、头像、时间戳。
    pub fn text(&self) -> String {
        self.text_at(today_days())
    }

    /// 固定「今天」的版本，测试用。
    pub fn text_at(&self, today: i64) -> String {
        format!(
            "{}{} · {} · {}",
            day_label_at(today, self.day.min(DAY_SPAN - 1)),
            SLOTS[self.slot.min(SLOTS.len() - 1)],
            crate::areas::area_name(self.area),
            INTENTS[self.intent.min(INTENTS.len() - 1)],
        )
    }
}

/// 赞助店家（虚构示例）。结果屏最多给 3 家（02-features E 节）。
///
/// 距离只给档位（「步行可达」），不给米数 —— 米数等于在告诉你对方此刻离你多远。
#[derive(Clone, Copy, Debug)]
pub struct Merchant {
    pub name: &'static str,
    pub address: &'static str,
    pub hours: &'static str,
    pub walk: &'static str,
}

/// 三家虚构示例店。真实版本由商户侧配置，这里只为把结果屏铺满。
pub const MERCHANTS: [Merchant; 3] = [
    Merchant {
        name: "禾间小馆",
        address: "朝阳区 · 三里屯一带（示例地址）",
        hours: "每天 11:00 - 22:00",
        walk: "步行可达",
    },
    Merchant {
        name: "长夏咖啡",
        address: "朝阳区 · 三里屯一带（示例地址）",
        hours: "每天 09:00 - 20:00",
        walk: "步行可达",
    },
    Merchant {
        name: "元宵书局 · café",
        address: "东城区 · 崇文门一带（示例地址）",
        hours: "周二至周日 10:00 - 21:00",
        walk: "需要坐几站",
    },
];

/// 券的有效天数。
pub const REWARD_VALID_DAYS: i64 = 7;

/// 相遇礼券（虚构示例）。
///
/// 券上没有联系人、没有坐标、没有「和谁在哪天相遇」。`expires_on` 是券自身的
/// 有效期（02-features H 节 `RewardClaim.expires_at`），不是相遇日期字段。
/// `claimed` 留着只为兼容旧状态文件 —— 新流程里确认成功即出券，没有领取这一步。
#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct RewardClaim {
    pub venue: String,
    pub offer: String,
    pub claimed: bool,
    pub redeemed: bool,
    pub expires_on: Option<i64>,
    pub token: Option<String>,
    pub terms: Option<String>,
}

/// 券在券包里的三个分区（02-features 七.3）。过期与否按当天算，不落盘 ——
/// 落盘的话，一台好几天没开的手机再打开时，券的分区就是错的。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewardState {
    Available,
    Redeemed,
    Expired,
}

impl RewardState {
    pub fn label(self) -> &'static str {
        match self {
            RewardState::Available => "可用",
            RewardState::Redeemed => "已核销",
            RewardState::Expired => "已过期",
        }
    }
}

impl RewardClaim {
    /// 今天看这张券属于哪一区。已核销优先于过期：核销过的券不该又变成「过期」。
    pub fn state(&self, today: i64) -> RewardState {
        if self.redeemed {
            RewardState::Redeemed
        } else if self.expires_on.map(|d| d < today).unwrap_or(false) {
            RewardState::Expired
        } else {
            RewardState::Available
        }
    }

    /// 「还剩 2 天」/「今天最后一天」。过期与已核销的券不提醒。
    pub fn remaining_label(&self, today: i64) -> Option<String> {
        let d = self.expires_on? - today;
        match d {
            _ if self.redeemed => None,
            0 => Some("今天最后一天".to_string()),
            1..=3 => Some(format!("还剩 {} 天", d)),
            _ => None,
        }
    }

    /// 「有效期至 9 月 24 日」。没有 expires_on 的旧券只说「有效期以券面为准」。
    pub fn expiry_label(&self) -> String {
        match self.expires_on {
            Some(d) => {
                let (_, m, day) = days_to_civil(d);
                format!("有效期至 {} 月 {} 日", m, day)
            }
            None => "有效期以券面为准".to_string(),
        }
    }
}

/// 互认窗口：10 分钟（与门槛屏上的说法一致）。
pub const RECOG_WINDOW_SECS: u64 = 600;

/// 倒计时文案：`9:58`。中性语气，不做红色跳动（design/05）。
pub fn countdown_label(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// 现场互认会话状态（02 D 节）。
///
/// 用户只看到四个态：选人（没有会话）→ 定位门槛 → 等待 → 结果。
/// 旧版的 `Session` / `SelfDone` 合并成 `Waiting`：本人确认在点「确认相遇」
/// 那一刻就完成了，再让人点一次是多出来的。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecogStage {
    /// 定位门槛：**会话尚未建立**。
    LocationGate,
    /// 定位被拒：会话仍未建立，只能再开定位或退成普通回忆。
    NoLocation,
    /// 会话已建立，本人确认已随之提交，等对方在现场也确认。
    Waiting,
    /// 双方确认 + 同地校验通过。
    Success,
    /// 互认成立但同地不成立：有回忆，没有相遇礼。
    Ordinary,
    /// 倒计时归零 / 对方一直没确认。
    Expired,
    /// 双方信息尚未一致（不揭示谁填了谁）。
    Mismatch,
    /// 相遇成立，但相遇礼限额 / 无库存。
    NoStock,
}

impl RecogStage {
    /// 是不是第 ④ 屏（结果屏）。六条异常路径与成功路径停在同一屏。
    pub fn is_result(self) -> bool {
        matches!(
            self,
            RecogStage::Success
                | RecogStage::Ordinary
                | RecogStage::Expired
                | RecogStage::Mismatch
                | RecogStage::NoStock
        )
    }

    /// 会话是否已经建立。未授权定位时恒为 false（硬门槛，02 D）。
    pub fn session_open(self) -> bool {
        !matches!(self, RecogStage::LocationGate | RecogStage::NoLocation)
    }
}

/// 一次现场互认会话（运行时状态，不持久化；会话结束即清除）。
#[derive(Clone, Debug)]
pub struct RecogSession {
    pub contact_id: usize,
    pub label: String,
    /// 4 位会话短码（虚构）。
    pub code: String,
    pub stage: RecogStage,
    /// 交叉互认已尝试次数（最多 3 次）。
    pub attempts: u32,
    pub choice: MemoryChoice,
    /// 成功页离页时按选择写入，只写一次。
    pub memory_written: bool,
    /// 会话建立后走了多少秒（由 UI 每秒 tick 一次）。
    pub elapsed: u64,
}

impl RecogSession {
    /// 刚选完人、点了「确认相遇」：停在定位门槛，会话还没建。
    pub fn new(contact_id: usize, label: String, seed: usize) -> Self {
        Self {
            contact_id,
            label,
            code: format!("{:04}", 1000 + seed % 9000),
            stage: RecogStage::LocationGate,
            attempts: 0,
            choice: MemoryChoice::Save,
            memory_written: false,
            elapsed: 0,
        }
    }

    /// 会话是否已建立。未授权定位就不建会话（硬门槛）。
    pub fn session_open(&self) -> bool {
        self.stage.session_open()
    }

    /// 「开启定位并确认」：建会话 + 提交本人确认，一步到等待态。
    pub fn grant_location(&mut self) -> bool {
        if matches!(self.stage, RecogStage::LocationGate | RecogStage::NoLocation) {
            self.stage = RecogStage::Waiting;
            self.elapsed = 0;
            true
        } else {
            false
        }
    }

    /// 拒绝定位：停在门槛，会话不建。
    pub fn deny_location(&mut self) -> bool {
        if self.stage == RecogStage::LocationGate {
            self.stage = RecogStage::NoLocation;
            true
        } else {
            false
        }
    }

    /// 对方也确认，同地校验通过 → 成功。
    pub fn peer_confirm(&mut self) -> bool {
        if self.stage == RecogStage::Waiting {
            self.stage = RecogStage::Success;
            true
        } else {
            false
        }
    }

    /// 信息不一致：计数 +1（最多 3 次），停在结果屏。返回是否仍可重试。
    pub fn peer_mismatch(&mut self) -> bool {
        if self.stage == RecogStage::Waiting && self.attempts < 3 {
            self.attempts += 1;
            self.stage = RecogStage::Mismatch;
            self.can_retry()
        } else {
            false
        }
    }

    /// 还剩几次重试。
    pub fn retries_left(&self) -> u32 {
        3u32.saturating_sub(self.attempts)
    }

    pub fn can_retry(&self) -> bool {
        self.retries_left() > 0
    }

    /// 「再试一次」：回到等待态，倒计时重新起算。
    pub fn retry(&mut self) -> bool {
        if self.stage == RecogStage::Mismatch && self.can_retry() {
            self.stage = RecogStage::Waiting;
            self.elapsed = 0;
            true
        } else {
            false
        }
    }

    /// 同地不成立：相遇成立，没有券。**不能谎称验证失败**。
    pub fn not_same_place(&mut self) -> bool {
        if self.stage == RecogStage::Waiting {
            self.stage = RecogStage::Ordinary;
            true
        } else {
            false
        }
    }

    /// 限额 / 无库存：相遇成立，没有券。与同地不成立必须分开说。
    pub fn no_stock(&mut self) -> bool {
        if self.stage == RecogStage::Waiting {
            self.stage = RecogStage::NoStock;
            true
        } else {
            false
        }
    }

    /// 倒计时走一秒。归零就过期；返回是否发生了状态变化。
    pub fn tick(&mut self) -> bool {
        if self.stage != RecogStage::Waiting {
            return false;
        }
        self.elapsed = self.elapsed.saturating_add(1);
        if self.elapsed >= RECOG_WINDOW_SECS {
            self.stage = RecogStage::Expired;
        }
        true
    }

    /// 还可确认多久（秒）。
    pub fn remaining_secs(&self) -> u64 {
        RECOG_WINDOW_SECS.saturating_sub(self.elapsed)
    }

    /// 进度环的完成度 0.0..=1.0。
    pub fn progress(&self) -> f32 {
        (self.elapsed as f32 / RECOG_WINDOW_SECS as f32).clamp(0.0, 1.0)
    }
}

/// 本机设置。两类通知默认关闭（02-features 七.4：opt-in）。
///
/// **明确不做**「附近有熟人」「有人和你去了同一个地方」这类通知 —— 它们会把
/// 匿名机会变成实时位置广播，直接违反 02 B 节。以后想加功能的人请先读那一节。
#[derive(Clone, Debug, PartialEq, SerJson, DeJson)]
pub struct Settings {
    /// 行程 30 分钟后到期。
    pub notify_publish: bool,
    /// 手里的券还剩 2 天。
    pub notify_reward: bool,
    /// 本机是否已给过定位授权（只影响门槛屏的默认文案，不缓存任何坐标）。
    pub location_granted: bool,
    /// 开场三屏看完（或跳过）了没有。跳过也算看完 —— 入口在「我」页留着。
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            notify_publish: false,
            notify_reward: false,
            location_granted: false,
            onboarded: false,
        }
    }
}

/// 只有两类通知（docs/02-redesign-plan.md 七.4），而且两类都默认关闭。
///
/// **明确不做**：「附近有熟人」「有人和你去了同一个地方」—— 这类通知会把一次
/// 匿名机会直接变成实时位置广播，违反 02 B。以后要加通知，先回来读这一行。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    /// 你发布的那段去向快结束了。
    PublishExpiring,
    /// 手里有张券快过期了。
    RewardExpiring,
}

impl NoticeKind {
    pub fn title(self) -> &'static str {
        match self {
            NoticeKind::PublishExpiring => "去向快到期了",
            NoticeKind::RewardExpiring => "券快过期了",
        }
    }
}

/// 一条待发的通知。文案里不含任何人、任何坐标 —— 说的全是你自己的东西。
#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
}

/// 三个粗时段各自的结束时刻（分钟）。发布到期就是它所在的时段结束。
const SLOT_END_MIN: [u32; 3] = [12 * 60, 18 * 60, 22 * 60];
/// 提前多久提醒去向到期。
pub const PUBLISH_NOTICE_LEAD_MIN: u32 = 30;
/// 券剩几天时提醒。
pub const REWARD_NOTICE_LEAD_DAYS: i64 = 2;

/// 此刻该发哪些通知。纯函数：今天、此刻几点、状态全从外面传进来，
/// 所以能直接写单测，不用等到 11:30、也不用等两天。
///
/// 关着的那一类一条都不发 —— 开关是唯一的闸，不在 UI 侧再判一次。
pub fn due_notices(s: &OuyuState, today: i64, now_min: u32) -> Vec<Notice> {
    let mut out = Vec::new();
    if s.settings.notify_publish {
        if let Some(p) = &s.publish {
            // 只提醒今天的那一条：明天下午的去向今天不到期。
            if p.status == PublishStatus::Published && p.day == 0 {
                let end = SLOT_END_MIN[p.slot.min(2)];
                let lead = end.saturating_sub(PUBLISH_NOTICE_LEAD_MIN);
                if now_min >= lead && now_min < end {
                    out.push(Notice {
                        kind: NoticeKind::PublishExpiring,
                        // 标题已经说了是「去向」，正文就别再重复一遍 ——
                        // 句子越短，窄屏上标点落在行首的机会越小。
                        text: format!(
                            "今天{}那条还有 {} 分钟结束，到期自动退出",
                            SLOTS[p.slot.min(2)],
                            end - now_min
                        ),
                    });
                }
            }
        }
    }
    if s.settings.notify_reward {
        for r in &s.wallet {
            if r.state(today) != RewardState::Available {
                continue;
            }
            let Some(exp) = r.expires_on else { continue };
            let left = exp - today;
            if (0..=REWARD_NOTICE_LEAD_DAYS).contains(&left) {
                out.push(Notice {
                    kind: NoticeKind::RewardExpiring,
                    text: if left == 0 {
                        format!("「{}」那张券今天最后一天。", r.venue)
                    } else {
                        format!("「{}」那张券还剩 {} 天。", r.venue, left)
                    },
                });
                break;
            }
        }
    }
    out
}

/// 应用状态。长期部分见 PersistedState；publish / echo 为运行时短时数据。
pub struct OuyuState {
    pub contacts: Vec<ContactLocal>,
    pub encounters: Vec<EncounterLocal>,
    pub directory: Vec<String>,
    /// 券包。最新的在最后；三个分区按当天现算（见 RewardClaim::state）。
    pub wallet: Vec<RewardClaim>,
    pub settings: Settings,
    next_encounter_id: usize,

    /// 当前 Tab（运行时）。
    pub tab: usize,
    /// 有效发布（最多一个）；None = 未发布。
    pub publish: Option<Publish>,
    /// 匿名回声（ECHOES 下标）；与行程一同到期清除（撤回时清空）。
    pub echo: Option<usize>,
    /// 最近发布过的片区 id，最新在前，最多 5 条。**只在本机**，不上传、
    /// 不进 AI 快照、不进券——纯粹是让下次填地址少翻一次列表。
    pub recent_areas: Vec<u16>,
}

impl Default for OuyuState {
    fn default() -> Self {
        Self::load()
    }
}

impl OuyuState {
    pub fn demo() -> Self {
        // 演示回忆落在最近两周内，成就曲线开箱即有形状（真实 ISO 日期）。
        let today = today_days();
        Self {
            contacts: vec![
                ContactLocal { id: 0, label: "林舟".into() },
                ContactLocal { id: 1, label: "陈晓".into() },
                ContactLocal { id: 2, label: "许宁".into() },
            ],
            encounters: vec![
                EncounterLocal {
                    id: 0,
                    contact_id: Some(0),
                    label_snapshot: "林舟".into(),
                    date: fmt_days(today - 5),
                    hidden: false,
                    note: "这次聊得很开心。".into(),
                },
                EncounterLocal {
                    id: 1,
                    contact_id: Some(1),
                    label_snapshot: "陈晓".into(),
                    date: fmt_days(today - 12),
                    hidden: false,
                    note: "一起走了走。".into(),
                },
            ],
            directory: vec!["周子墨".into(), "林小满".into(), "黄一诺".into(), "吴凯文".into()],
            wallet: Vec::new(),
            settings: Settings::default(),
            next_encounter_id: 2,
            tab: 0,
            publish: None,
            echo: None,
            recent_areas: Vec::new(),
        }
    }

    pub fn contact(&self, id: usize) -> Option<&ContactLocal> {
        self.contacts.iter().find(|c| c.id == id)
    }

    /// 相遇次数口径（02 F 节）：统计目前保存的回忆，含隐藏；删除后减少。
    pub fn meeting_count(&self, contact_id: usize) -> usize {
        self.encounters
            .iter()
            .filter(|e| e.contact_id == Some(contact_id))
            .count()
    }

    /// 按本次选择写入一条回忆。返回是否实际写入（Skip 不写、不累计次数）。
    /// 写入后立即持久化。
    pub fn push_memory(&mut self, contact_id: usize, label: &str, choice: MemoryChoice) -> bool {
        if choice == MemoryChoice::Skip {
            return false;
        }
        let id = self.next_encounter_id;
        self.next_encounter_id += 1;
        self.encounters.push(EncounterLocal {
            id,
            contact_id: Some(contact_id),
            label_snapshot: label.to_string(),
            date: today_iso(),
            hidden: choice == MemoryChoice::Hidden,
            note: String::new(),
        });
        self.save();
        true
    }

    /// 成功页离页时按会话选择写入，只写一次（02 F 节）。
    pub fn write_session_memory(&mut self, session: &mut RecogSession) {
        if session.memory_written {
            return;
        }
        session.memory_written = true;
        self.push_memory(session.contact_id, &session.label.clone(), session.choice);
    }

    /// 逐条隐藏 / 恢复（只影响该条，可恢复）。
    pub fn set_hidden(&mut self, encounter_id: usize, hidden: bool) -> bool {
        if let Some(e) = self.encounters.iter_mut().find(|e| e.id == encounter_id) {
            e.hidden = hidden;
            self.save();
            true
        } else {
            false
        }
    }

    /// 删除前的一份快照。5 秒内点「撤销」就整个放回去。
    ///
    /// 存的是两张表的副本而不是一条逆操作 —— 删联系人会连带改动回忆里的
    /// `contact_id`，逐条推演逆操作迟早会和主流程走散。这点数据量，整份拷贝
    /// 最省心也最不会错。
    pub fn snapshot_for_undo(&self, label: impl Into<String>) -> UndoSnapshot {
        UndoSnapshot {
            label: label.into(),
            contacts: self.contacts.clone(),
            encounters: self.encounters.clone(),
        }
    }

    /// 把快照放回去。
    pub fn restore(&mut self, u: UndoSnapshot) {
        self.contacts = u.contacts;
        self.encounters = u.encounters;
        self.save();
    }

    /// 导出本机数据：把落盘的那份原样写到 `ouyu/export-<日期>.json`。
    /// 导出的内容和 state.json 一致 —— 不多带坐标、不多带相遇地点。
    pub fn export_data(&self) -> Option<std::path::PathBuf> {
        let dir = Self::state_file()?.parent()?.to_path_buf();
        let path = dir.join(format!("export-{}.json", fmt_days(today_days())));
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(&path, self.persisted().serialize_json()).ok()?;
        Some(path)
    }

    /// 清除本机数据：联系人、回忆、券包、最近片区、通讯录候选全部清空，
    /// 设置留着（清完还要能看见「已清除」这句话）。不可逆，UI 必须二次确认。
    pub fn clear_local_data(&mut self) {
        self.contacts.clear();
        self.encounters.clear();
        self.directory.clear();
        self.wallet.clear();
        self.recent_areas.clear();
        self.next_encounter_id = 0;
        self.publish = None;
        self.echo = None;
        self.save();
    }

    /// 删除单条回忆（计数相应减少）。
    pub fn delete_encounter(&mut self, encounter_id: usize) -> bool {
        let before = self.encounters.len();
        self.encounters.retain(|e| e.id != encounter_id);
        let removed = self.encounters.len() != before;
        if removed {
            self.save();
        }
        removed
    }

    /// 删除某联系人的全部回忆（含隐藏），联系人保留。
    pub fn delete_memories_of(&mut self, contact_id: usize) -> usize {
        let before = self.encounters.len();
        self.encounters.retain(|e| e.contact_id != Some(contact_id));
        let removed = before - self.encounters.len();
        if removed > 0 {
            self.save();
        }
        removed
    }

    /// 删除联系人（不改系统通讯录）。also_memories=false 时保留回忆：
    /// contact_id 置 None，用 label_snapshot 独立展示（02 F 节）。
    pub fn delete_contact(&mut self, contact_id: usize, also_memories: bool) -> bool {
        let before = self.contacts.len();
        self.contacts.retain(|c| c.id != contact_id);
        if self.contacts.len() == before {
            return false;
        }
        if also_memories {
            self.encounters.retain(|e| e.contact_id != Some(contact_id));
        } else {
            for e in self.encounters.iter_mut() {
                if e.contact_id == Some(contact_id) {
                    e.contact_id = None;
                }
            }
        }
        self.save();
        true
    }

    /// 发布 / 修改去向：每人只有一个有效发布，修改覆盖旧值。
    /// 不落盘（短时数据，见模块头注释）。
    pub fn publish(&mut self, day: usize, slot: usize, area: u16, intent: usize) {
        self.publish = Some(Publish {
            day,
            slot,
            area,
            intent,
            status: PublishStatus::Published,
        });
        self.remember_area(area);
    }

    /// 记一笔「最近去过」（本机，去重，最多 5 条）。
    pub fn remember_area(&mut self, area: u16) {
        self.recent_areas.retain(|&a| a != area);
        self.recent_areas.insert(0, area);
        self.recent_areas.truncate(5);
        self.save();
    }

    /// 撤回即退出：停止参与机会，回声与行程一同清除。
    pub fn withdraw(&mut self) {
        self.publish = None;
        self.echo = None;
    }

    /// 留一个轻轻的回声（固定三选，无发送者 / 数量 / 已读）。
    pub fn set_echo(&mut self, echo: usize) {
        self.echo = Some(echo.min(ECHOES.len() - 1));
    }

    /// 确认成功即出券 —— 没有「领取」这一步（02 D 节）。
    ///
    /// 券上只有店家、面额、条款、核销码和有效期；没有联系人、没有坐标，
    /// 也没有「和谁在哪天相遇」这个字段。
    pub fn issue_reward(&mut self, today: i64, seed: usize) {
        self.wallet.push(RewardClaim {
            venue: MERCHANTS[0].name.into(),
            offer: "¥60 双人满 ¥120 可用".into(),
            claimed: true,
            redeemed: false,
            expires_on: Some(today + REWARD_VALID_DAYS),
            token: Some(format!("{:06}", 100000 + seed % 900000)),
            terms: Some(
                "消费满 ¥120 券后 ¥60 · 不兑现不叠加 · 每对联系人 7 天限领一次".into(),
            ),
        });
        self.save();
    }

    /// 最新发的那张券（结果屏显示的就是它）。
    pub fn latest_reward(&self) -> Option<&RewardClaim> {
        self.wallet.last()
    }

    /// 手里还有没有能用的券。有就不再发新的。
    pub fn has_available_reward(&self, today: i64) -> bool {
        self.wallet.iter().any(|r| r.state(today) == RewardState::Available)
    }

    /// 按分区取券（券包三个分区）。
    pub fn rewards_in(&self, today: i64, state: RewardState) -> Vec<&RewardClaim> {
        self.wallet.iter().rev().filter(|r| r.state(today) == state).collect()
    }

    /// 模拟核销（幂等：已核销再调不改变状态）。
    /// 核销券包里的第 `idx` 张。
    pub fn redeem_at(&mut self, idx: usize) -> bool {
        match self.wallet.get_mut(idx) {
            Some(r) if !r.redeemed => {
                r.redeemed = true;
                self.save();
                true
            }
            _ => false,
        }
    }

    /// 核销最新那张（结果屏上的「到店核销」）。
    pub fn redeem_reward(&mut self) -> bool {
        match self.wallet.last_mut() {
            Some(r) if !r.redeemed => {
                r.redeemed = true;
                self.save();
                true
            }
            _ => false,
        }
    }

    /// 改一条回忆的备注。备注是这个人自己写的，不参与任何统计和推荐，
    /// 只在回忆详情里显示、在搜索里被搜到。
    pub fn set_note(&mut self, encounter_id: usize, note: &str) -> bool {
        let Some(e) = self.encounters.iter_mut().find(|e| e.id == encounter_id) else {
            return false;
        };
        let note = note.trim();
        if e.note == note {
            return false;
        }
        e.note = note.to_string();
        self.save();
        true
    }

    /// 手动添一个熟人。
    ///
    /// 重名直接拒绝而不是加个「(2)」：称呼是这个人自己起的，两个一模一样的
    /// 称呼他自己也分不清，不如当场说出来让他改一个。
    pub fn add_contact(&mut self, label: &str) -> Result<usize, AddContactError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(AddContactError::Empty);
        }
        if label.chars().count() > 16 {
            return Err(AddContactError::TooLong);
        }
        if self.contacts.iter().any(|c| c.label == label) {
            return Err(AddContactError::Duplicate);
        }
        let id = self.contacts.iter().map(|c| c.id + 1).max().unwrap_or(0);
        self.contacts.push(ContactLocal { id, label: label.to_string() });
        // 通讯录池里同名的那条就不用再提示「可添加」了。
        self.directory.retain(|d| d != label);
        self.save();
        Ok(id)
    }

    /// 把 `from` 并进 `into`：`from` 名下的回忆全部转过去（连同称呼快照），
    /// 然后删掉 `from`。
    ///
    /// 回忆一条都不丢 —— 合并的是同一个人的两个记法，不是两个人。
    /// 返回转过去的回忆条数。
    pub fn merge_contacts(&mut self, from_id: usize, into_id: usize) -> Option<usize> {
        if from_id == into_id {
            return None;
        }
        let into_label = self.contact(into_id)?.label.clone();
        self.contact(from_id)?;
        let mut moved = 0;
        for e in self.encounters.iter_mut() {
            if e.contact_id == Some(from_id) {
                e.contact_id = Some(into_id);
                e.label_snapshot = into_label.clone();
                moved += 1;
            }
        }
        self.contacts.retain(|c| c.id != from_id);
        self.save();
        Some(moved)
    }

    /// 把 vCard 导入的名字并入本机通讯录池（与通讯录、熟人双向去重）。
    /// 返回新并入的人数。
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
}

/// 手动添加熟人时能出的三种错。每一种在界面上都有一句自己的话，
/// 不共用一句「添加失败」。
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

/// 删除前的两张表快照（见 `snapshot_for_undo`）。
pub struct UndoSnapshot {
    /// Toast 上那句话，例如「已删除 1 条回忆」。
    pub label: String,
    contacts: Vec<ContactLocal>,
    encounters: Vec<EncounterLocal>,
}

/// 落盘的部分：contacts、encounters、directory、reward claim（02 G 节边界）。
/// 发布 / 回声 / 会话等短时数据不在这里。
#[derive(Clone, Debug, Default, PartialEq, SerJson, DeJson)]
pub struct PersistedState {
    pub contacts: Vec<ContactLocal>,
    pub encounters: Vec<EncounterLocal>,
    pub directory: Option<Vec<String>>,
    /// 旧版（批次 3 及以前）只存一张券。读到就并进 wallet，写出时不再填。
    pub reward: Option<RewardClaim>,
    pub wallet: Option<Vec<RewardClaim>>,
    pub settings: Option<Settings>,
    pub next_encounter_id: Option<usize>,
    pub recent_areas: Option<Vec<u16>>,
}

/// 旧版（Phase 0）state.json 里仍能认出的联系人字段：name → label。
/// 其余旧字段（my_windows/cards/stealth/my_pos/intimacy/旧 encounters）
/// 按要求直接忽略；lenient 模式下未知字段被跳过。
#[derive(DeJson)]
struct LegacyContact {
    id: usize,
    name: String,
}

#[derive(DeJson)]
struct LegacyState {
    contacts: Option<Vec<LegacyContact>>,
}

impl OuyuState {
    /// 进程内状态文件：`$MAKEPAD_HOME/ouyu/state.json`（宿主会给子进程设 MAKEPAD_HOME）。
    pub fn state_file() -> Option<std::path::PathBuf> {
        std::env::var_os("MAKEPAD_HOME")
            .map(|h| std::path::Path::new(&h).join("ouyu").join("state.json"))
    }

    /// vCard 导入文件：`<MAKEPAD_HOME>/ouyu/contacts.vcf`。
    pub fn contacts_vcf() -> Option<std::path::PathBuf> {
        Self::state_file().and_then(|p| p.parent().map(|d| d.join("contacts.vcf")))
    }

    /// 演示便利：状态目录里没有 contacts.vcf 时写一份示例（4 个虚构联系人，
    /// 含续行折叠与 N 兜底两种写法），让「导入 vCard」按钮开箱可点。
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
            contacts: self.contacts.clone(),
            encounters: self.encounters.clone(),
            directory: Some(self.directory.clone()),
            reward: None,
            wallet: Some(self.wallet.clone()),
            settings: Some(self.settings.clone()),
            next_encounter_id: Some(self.next_encounter_id),
            recent_areas: Some(self.recent_areas.clone()),
        }
    }

    pub fn apply_persisted(&mut self, p: PersistedState) {
        self.contacts = p.contacts;
        self.encounters = p.encounters;
        if let Some(dir) = p.directory {
            self.directory = dir;
        }
        // 旧文件里的那一张券并进券包；两边都有时以 wallet 为准。
        self.wallet = p.wallet.unwrap_or_else(|| p.reward.into_iter().collect());
        if let Some(st) = p.settings {
            self.settings = st;
        }
        self.next_encounter_id = p.next_encounter_id.unwrap_or_else(|| {
            self.encounters.iter().map(|e| e.id + 1).max().unwrap_or(0)
        });
        if let Some(r) = p.recent_areas {
            self.recent_areas = r.into_iter().filter(|&a| crate::areas::area(a).is_some()).collect();
        }
    }

    /// 从 state_file 加载并覆盖在 demo 底座上；文件不存在或损坏时用 demo 数据。
    pub fn load() -> Self {
        let mut s = Self::demo();
        if let Some(path) = Self::state_file() {
            if let Some(p) = Self::load_from(&path) {
                s.apply_persisted(p);
            }
        }
        s
    }

    pub fn save(&self) {
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

    /// 读取状态文件。三种结果：
    /// - 新格式：完整解析；
    /// - 旧格式（Phase 0 结伴卡时代）：只迁出联系人（name→label），行踪历史 /
    ///   结伴卡 / 隐身 / 亲密度等旧字段直接忽略，下次保存后自然消失；
    /// - 读不到或完全无法解析：None（回退 demo 数据）。
    pub fn load_from(path: &std::path::Path) -> Option<PersistedState> {
        let text = std::fs::read_to_string(path).ok()?;
        if let Ok(p) = PersistedState::deserialize_json_lenient(&text) {
            return Some(p);
        }
        if let Ok(legacy) = LegacyState::deserialize_json_lenient(&text) {
            if let Some(contacts) = legacy.contacts {
                return Some(PersistedState {
                    contacts: contacts
                        .into_iter()
                        .map(|c| ContactLocal { id: c.id, label: c.name })
                        .collect(),
                    ..Default::default()
                });
            }
        }
        None
    }
}

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

    /// 2026-09-19 是个周六，用固定日期跑排序，结果与「今天」无关。
    const SAT: i64 = 20715;

    #[test]
    fn the_ranking_covers_every_area_exactly_once() {
        let r = opportunity_ranking_at(SAT, 0);
        assert_eq!(r.len(), crate::areas::AREAS.len());
        let mut ids: Vec<u16> = r.iter().map(|o| o.area).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), crate::areas::AREAS.len());
    }

    #[test]
    fn the_same_day_always_ranks_the_same_way() {
        // 同一天内反复进发现页，顺序必须一致，否则「排行」就没有意义。
        let a = opportunity_ranking_at(SAT, 2);
        let b = opportunity_ranking_at(SAT, 2);
        assert_eq!(a, b);
    }

    #[test]
    fn a_different_day_gives_a_different_ranking() {
        let a: Vec<u16> = opportunity_ranking_at(SAT, 0).iter().map(|o| o.area).collect();
        let b: Vec<u16> = opportunity_ranking_at(SAT, 3).iter().map(|o| o.area).collect();
        assert_ne!(a, b);
    }

    #[test]
    fn the_ranking_is_sorted_by_level_high_to_low() {
        let order = |l: OppLevel| match l {
            OppLevel::Likely => 0,
            OppLevel::Possible => 1,
            OppLevel::Few => 2,
            OppLevel::BelowThreshold => 3,
        };
        for day in 0..DAY_SPAN {
            let r = opportunity_ranking_at(SAT, day);
            for w in r.windows(2) {
                assert!(order(w[0].level) <= order(w[1].level), "第 {day} 天的排序乱了");
            }
        }
    }

    #[test]
    fn below_threshold_areas_never_leak_a_best_slot() {
        // 未达阈值就完全不展示为熟人机会：连时段都不给（02 B）。
        for day in 0..DAY_SPAN {
            for o in opportunity_ranking_at(SAT, day) {
                if !o.level.shown() {
                    assert_eq!(o.best_slot, None);
                }
            }
        }
    }

    #[test]
    fn most_areas_stay_below_the_threshold_on_any_given_day() {
        // 如果几乎所有片区都「有机会」，分档就退化成装饰了。
        for day in 0..DAY_SPAN {
            let r = opportunity_ranking_at(SAT, day);
            let shown = r.iter().filter(|o| o.level.shown()).count();
            // 「有机会」必须稀有：满城都是机会的话，分档和阈值都没意义了。
            assert!(shown * 5 < r.len() * 2, "第 {day} 天有 {shown} 个片区达到阈值，太多了");
            assert!(shown > 0, "第 {day} 天一个片区都不亮，排行页会一直是空的");
        }
    }

    #[test]
    fn a_single_lookup_matches_what_the_ranking_says() {
        // 发布向导里的行内分档与发现页的排行是两条调用，结果不能对不上。
        for day in 0..DAY_SPAN {
            for o in opportunity_ranking_at(SAT, day) {
                assert_eq!(area_opportunity_at(SAT, day, o.area), o);
            }
        }
    }

    #[test]
    fn offices_are_busier_on_weekdays_and_parks_on_weekends() {
        let weekend = SAT;
        let weekday = SAT + 2; // 周一
        assert!(is_weekend(weekend) && !is_weekend(weekday));
        let count = |today: i64, kind: crate::areas::AreaKind| {
            crate::areas::AREAS
                .iter()
                .filter(|a| a.kind == kind)
                .filter(|a| level_of(candidate_count(a, today)).shown())
                .count()
        };
        assert!(count(weekday, crate::areas::AreaKind::Office) >= count(weekend, crate::areas::AreaKind::Office));
        assert!(count(weekend, crate::areas::AreaKind::Park) >= count(weekday, crate::areas::AreaKind::Park));
    }

    #[test]
    fn a_publish_line_names_the_area_by_id_not_by_index() {
        let p = Publish {
            day: 0,
            slot: 1,
            area: crate::areas::AREAS[0].id,
            intent: 0,
            status: PublishStatus::Published,
        };
        let line = p.text_at(SAT);
        assert!(line.starts_with("今天下午 · "));
        assert!(line.contains(crate::areas::AREAS[0].name));
        // 一行文案里不能出现昵称、日期或时间戳。
        assert!(!line.contains('-') && !line.contains(':'));
    }

    #[test]
    fn day_labels_run_today_tomorrow_then_weekdays() {
        assert_eq!(day_label_at(SAT, 0), "今天");
        assert_eq!(day_label_at(SAT, 1), "明天");
        assert_eq!(day_label_at(SAT, 2), "后天");
        assert_eq!(day_label_at(SAT, 3), "周二");
    }

    #[test]
    fn recent_areas_dedupe_and_cap_at_five() {
        let mut st = OuyuState::demo();
        for a in [1u16, 2, 3, 4, 5, 6] {
            st.remember_area(a);
        }
        assert_eq!(st.recent_areas.len(), 5);
        assert_eq!(st.recent_areas[0], 6);
        st.remember_area(3);
        assert_eq!(st.recent_areas[0], 3);
        assert_eq!(st.recent_areas.iter().filter(|&&a| a == 3).count(), 1);
    }

    fn state() -> OuyuState {
        OuyuState::demo()
    }

    // ---- 相遇次数口径 ----

    #[test]
    fn meeting_count_includes_hidden() {
        let mut s = state();
        assert_eq!(s.meeting_count(0), 1);
        // 隐藏仍计入次数。
        assert!(s.push_memory(0, "林舟", MemoryChoice::Hidden));
        assert_eq!(s.meeting_count(0), 2);
        // 不保存不累计。
        assert!(!s.push_memory(0, "林舟", MemoryChoice::Skip));
        assert_eq!(s.meeting_count(0), 2);
    }

    #[test]
    fn meeting_count_decreases_after_delete() {
        let mut s = state();
        let id = s.encounters[0].id;
        assert!(s.delete_encounter(id));
        assert_eq!(s.meeting_count(0), 0);
        assert!(!s.delete_encounter(id)); // 再删返回 false
    }

    // ---- 隐藏 / 恢复 / 删除 ----

    #[test]
    fn hide_restore_delete() {
        let mut s = state();
        let id = s.encounters[0].id;
        assert!(s.set_hidden(id, true));
        assert!(s.encounters[0].hidden);
        assert_eq!(s.meeting_count(0), 1); // 隐藏仍计数
        assert!(s.set_hidden(id, false));
        assert!(!s.encounters[0].hidden);
        assert!(!s.set_hidden(999, true));
    }

    #[test]
    fn hidden_choice_writes_hidden_record() {
        let mut s = state();
        assert!(s.push_memory(2, "许宁", MemoryChoice::Hidden));
        let e = s.encounters.last().unwrap();
        assert!(e.hidden);
        assert_eq!(e.label_snapshot, "许宁");
        assert_eq!(e.contact_id, Some(2));
        // 新记录写真实 ISO 日期（不再是「今天」这类字符串）。
        assert_eq!(e.date, today_iso());
        assert!(parse_iso_days(&e.date).is_some());
        assert!(e.note.is_empty());
    }

    // ---- 日期与周桶 ----

    #[test]
    fn civil_roundtrip() {
        for (y, m, d) in [(1970, 1, 1), (2026, 9, 17), (2000, 2, 29), (2024, 2, 29), (1999, 12, 31)] {
            let days = civil_to_days(y, m, d);
            assert_eq!(days_to_civil(days), (y, m, d), "{y}-{m}-{d}");
        }
        assert_eq!(civil_to_days(1970, 1, 1), 0);
    }

    #[test]
    fn parse_iso_strict() {
        assert!(parse_iso_days("2026-09-17").is_some());
        assert!(parse_iso_days("09/12").is_none());
        assert!(parse_iso_days("今天").is_none());
        assert!(parse_iso_days("2026-9-7").is_none());
        assert!(parse_iso_days("2026-13-01").is_none());
        assert!(parse_iso_days("2026-09-32").is_none());
        assert!(parse_iso_days("2026-09-17x").is_none());
    }

    #[test]
    fn week_start_is_monday() {
        // 1970-01-01 周四；1970-01-05 是周一。
        assert_eq!(week_start(0), -3);
        assert_eq!(week_start(3), -3); // 周日
        assert_eq!(week_start(4), 4); // 周一
        assert_eq!(week_start(10), 4); // 同一个周日
        assert_eq!(week_start(11), 11); // 下一个周一
    }

    #[test]
    fn fmt_helpers() {
        assert_eq!(fmt_days(civil_to_days(2026, 9, 7)), "2026-09-07");
        assert_eq!(fmt_md(civil_to_days(2026, 9, 7)), "09/07");
    }

    // ---- 成就统计口径（06 节）----

    fn enc(id: usize, date: &str, hidden: bool) -> EncounterLocal {
        EncounterLocal {
            id,
            contact_id: Some(0),
            label_snapshot: "甲".into(),
            date: date.into(),
            hidden,
            note: String::new(),
        }
    }

    #[test]
    fn stats_only_count_visible_saved() {
        let today = civil_to_days(2026, 9, 17); // 周四
        let list = vec![
            enc(0, "2026-09-16", false),
            enc(1, "2026-09-10", false),
            enc(2, "2026-09-10", true), // 隐藏：退出统计
        ];
        let s = achievement_stats(&list, 8, today);
        assert_eq!(s.remembered, 2);
        assert_eq!(s.days, 2);
        assert_eq!(s.recent(), 2);
        assert_eq!(s.weekly.len(), 8);
        // 09-16 在本周（周一 09-14），09-10 在上一周。
        assert_eq!(s.weekly[7].count, 1);
        assert_eq!(s.weekly[6].count, 1);
    }

    #[test]
    fn stats_unparseable_dates_fall_into_this_week() {
        let today = civil_to_days(2026, 9, 17);
        let list = vec![enc(0, "今天", false), enc(1, "09/06", false)];
        let s = achievement_stats(&list, 4, today);
        assert_eq!(s.weekly[3].count, 2); // 都归入本周
        assert_eq!(s.days, 1); // 有效日期都是今天
    }

    #[test]
    fn stats_week_window_changes_sum() {
        let today = civil_to_days(2026, 9, 17);
        let list = vec![enc(0, "2026-08-10", false), enc(1, "2026-09-16", false)];
        let s8 = achievement_stats(&list, 8, today);
        assert_eq!(s8.recent(), 2); // 08-10 在近 8 周内
        let s4 = achievement_stats(&list, 4, today);
        assert_eq!(s4.recent(), 1); // 但不在近 4 周内（窗口从 08-24 起）
    }

    #[test]
    fn stats_hide_restore_delete_recalculate() {
        let mut s = state();
        let today = today_days();
        let stats = achievement_stats(&s.encounters, 8, today);
        assert_eq!(stats.remembered, 2);
        // 隐藏退出统计。
        assert!(s.set_hidden(0, true));
        let stats = achievement_stats(&s.encounters, 8, today);
        assert_eq!(stats.remembered, 1);
        // 恢复计入。
        assert!(s.set_hidden(0, false));
        assert_eq!(achievement_stats(&s.encounters, 8, today).remembered, 2);
        // 删除重算；熟人页口径（含隐藏）不受隐藏影响。
        assert_eq!(s.meeting_count(0), 1);
        assert!(s.delete_encounter(0));
        let stats = achievement_stats(&s.encounters, 8, today);
        assert_eq!(stats.remembered, 1);
        assert_eq!(s.meeting_count(0), 0);
    }

    // ---- 里程碑 ----

    #[test]
    fn milestones_thresholds() {
        let today = civil_to_days(2026, 9, 17);
        let empty: Vec<EncounterLocal> = vec![];
        let m = milestones(&achievement_stats(&empty, 8, today));
        assert!(!m[0].lit && !m[1].lit && !m[2].lit);

        let one = vec![enc(0, "2026-09-16", false)];
        let m = milestones(&achievement_stats(&one, 8, today));
        assert!(m[0].lit && !m[1].lit && !m[2].lit);
        assert_eq!(m[0].title, "第一次刚刚好");

        let three: Vec<_> = (0..3).map(|i| enc(i, "2026-09-16", false)).collect();
        let m = milestones(&achievement_stats(&three, 8, today));
        assert!(m[1].lit);
        assert!(!m[2].lit); // 同一天三次 ≠ 七个日子

        let seven_days: Vec<_> = (0..7)
            .map(|i| enc(i, &fmt_days(today - i as i64), false))
            .collect();
        let m = milestones(&achievement_stats(&seven_days, 8, today));
        assert!(m[2].lit);
        assert_eq!(m[2].desc, "七个有相遇的日子");
    }

    // ---- 删除联系人 ----

    #[test]
    fn delete_contact_keeps_memories_by_default() {
        let mut s = state();
        assert!(s.delete_contact(0, false));
        assert!(s.contact(0).is_none());
        // 回忆保留：contact_id 置 None，label_snapshot 独立展示。
        assert_eq!(s.encounters.len(), 2);
        let e = &s.encounters[0];
        assert_eq!(e.contact_id, None);
        assert_eq!(e.label_snapshot, "林舟");
        assert_eq!(s.meeting_count(0), 0); // 已删联系人次数归零（无关联）
        assert!(!s.delete_contact(0, false)); // 已不存在
    }

    #[test]
    fn delete_contact_with_memories() {
        let mut s = state();
        assert!(s.delete_contact(0, true));
        assert_eq!(s.encounters.len(), 1);
        assert_eq!(s.encounters[0].label_snapshot, "陈晓");
    }

    #[test]
    fn delete_memories_of_keeps_contact() {
        let mut s = state();
        assert_eq!(s.delete_memories_of(0), 1);
        assert_eq!(s.meeting_count(0), 0);
        assert!(s.contact(0).is_some()); // 联系人保留
        assert_eq!(s.delete_memories_of(0), 0);
    }

    // ---- 发布 ----

    #[test]
    fn publish_is_unique_and_overwrites() {
        let mut s = state();
        s.publish(0, 1, 0, 0);
        assert_eq!(s.publish.as_ref().unwrap().text(), "今天下午 · 三里屯一带 · 随意走走");
        // 修改覆盖旧值，仍只有一个有效发布。
        s.publish(1, 2, 2, 1);
        let p = s.publish.as_ref().unwrap();
        assert_eq!(p.text(), "明天晚间 · 国贸公共街区 · 顺路办事");
        assert_eq!(p.status, PublishStatus::Published);
    }

    #[test]
    fn withdraw_clears_publish_and_echo() {
        let mut s = state();
        s.publish(0, 0, 1, 2);
        s.set_echo(0);
        assert_eq!(s.echo, Some(0));
        s.withdraw();
        assert!(s.publish.is_none());
        assert!(s.echo.is_none()); // 回声与行程一同清除
    }

    // ---- 现场互认状态机 ----

    #[test]
    fn recog_happy_path() {
        let mut s = RecogSession::new(0, "林舟".into(), 42);
        assert_eq!(s.code, "1042");
        // 一开始停在定位门槛，会话尚未建立。
        assert_eq!(s.stage, RecogStage::LocationGate);
        assert!(!s.session_open());
        // 没过定位门槛，对方确认无效 —— 硬门槛。
        assert!(!s.peer_confirm());
        assert!(s.grant_location());
        assert!(s.session_open());
        assert_eq!(s.stage, RecogStage::Waiting);
        assert!(s.peer_confirm());
        assert_eq!(s.stage, RecogStage::Success);
        assert!(s.stage.is_result());
    }

    #[test]
    fn location_is_a_hard_gate() {
        let mut s = RecogSession::new(0, "林舟".into(), 3);
        assert!(s.deny_location());
        assert_eq!(s.stage, RecogStage::NoLocation);
        // 未授权就没有会话：对方怎么点都不成立。
        assert!(!s.session_open());
        assert!(!s.peer_confirm());
        assert!(!s.peer_mismatch());
        assert!(!s.tick());
        // 之后再开定位仍然可以走下去。
        assert!(s.grant_location());
        assert_eq!(s.stage, RecogStage::Waiting);
    }

    #[test]
    fn recog_mismatch_retries_up_to_three() {
        let mut s = RecogSession::new(0, "林舟".into(), 7);
        s.grant_location();
        assert!(s.peer_mismatch()); // 1
        assert_eq!(s.stage, RecogStage::Mismatch); // 停在结果屏
        assert_eq!(s.retries_left(), 2);
        assert!(s.retry());
        assert!(s.peer_mismatch()); // 2
        assert!(s.retry());
        assert!(!s.peer_mismatch()); // 3 → 达上限
        assert_eq!(s.attempts, 3);
        assert!(!s.can_retry());
        assert!(!s.retry()); // 用尽后不再给重试
    }

    #[test]
    fn countdown_runs_out_into_expired() {
        let mut s = RecogSession::new(0, "林舟".into(), 9);
        s.grant_location();
        assert_eq!(s.remaining_secs(), RECOG_WINDOW_SECS);
        assert_eq!(countdown_label(s.remaining_secs()), "10:00");
        for _ in 0..(RECOG_WINDOW_SECS - 1) {
            assert!(s.tick());
        }
        assert_eq!(s.stage, RecogStage::Waiting);
        assert_eq!(countdown_label(s.remaining_secs()), "0:01");
        s.tick();
        assert_eq!(s.stage, RecogStage::Expired);
        assert_eq!(s.remaining_secs(), 0);
        assert!((s.progress() - 1.0).abs() < 1e-6);
        assert!(!s.tick()); // 过期后不再走表
    }

    #[test]
    fn same_place_and_out_of_stock_are_different_outcomes() {
        // 两条异常必须分得开：无库存不能谎称「同地不成立」。
        let mut a = RecogSession::new(0, "林舟".into(), 1);
        a.grant_location();
        assert!(a.not_same_place());
        assert_eq!(a.stage, RecogStage::Ordinary);

        let mut b = RecogSession::new(0, "林舟".into(), 2);
        b.grant_location();
        assert!(b.no_stock());
        assert_eq!(b.stage, RecogStage::NoStock);
        assert_ne!(a.stage, b.stage);
        // 六条路径都落在同一张结果屏上。
        for st in [
            RecogStage::Success,
            RecogStage::Ordinary,
            RecogStage::Expired,
            RecogStage::Mismatch,
            RecogStage::NoStock,
        ] {
            assert!(st.is_result());
        }
        assert!(!RecogStage::Waiting.is_result());
        assert!(!RecogStage::LocationGate.is_result());
    }

    #[test]
    fn session_memory_written_once_on_leave() {
        let mut st = state();
        let mut s = RecogSession::new(0, "林舟".into(), 5);
        s.grant_location();
        s.peer_confirm();
        s.choice = MemoryChoice::Save;
        let before = st.encounters.len();
        st.write_session_memory(&mut s);
        st.write_session_memory(&mut s); // 重复离页不再写
        assert_eq!(st.encounters.len(), before + 1);
        assert!(!st.encounters.last().unwrap().hidden);
        // 本次选不保存：标记已处理但不写入。
        let mut s2 = RecogSession::new(0, "林舟".into(), 6);
        s2.choice = MemoryChoice::Skip;
        st.write_session_memory(&mut s2);
        assert!(s2.memory_written);
        assert_eq!(st.encounters.len(), before + 1);
    }

    // ---- 相遇礼 ----

    #[test]
    fn reward_is_issued_on_success_and_redeemed_once() {
        let mut s = state();
        assert!(!s.redeem_reward()); // 没有券不能核销
        let today = civil_to_days(2026, 9, 17);
        s.issue_reward(today, 42);
        let r = s.latest_reward().cloned().unwrap();
        // 确认成功即出券 —— 没有「领取」这一步。
        assert!(!r.redeemed);
        assert_eq!(r.offer, "¥60 双人满 ¥120 可用");
        assert_eq!(r.expires_on, Some(today + REWARD_VALID_DAYS));
        assert_eq!(r.expiry_label(), "有效期至 9 月 24 日");
        assert!(s.redeem_reward());
        assert!(s.latest_reward().unwrap().redeemed);
        assert!(!s.redeem_reward()); // 幂等
    }

    #[test]
    fn a_coupon_carries_no_contact_place_or_date_of_the_encounter() {
        // 红线：券上不得出现联系人、坐标、相遇日期（03 红线表第 7 行）。
        let mut s = state();
        s.issue_reward(civil_to_days(2026, 9, 17), 8);
        let r = s.latest_reward().cloned().unwrap();
        let text = format!(
            "{} {} {} {} {}",
            r.venue,
            r.offer,
            r.terms.clone().unwrap_or_default(),
            r.token.clone().unwrap_or_default(),
            r.expiry_label()
        );
        for name in s.contacts.iter().map(|c| c.label.clone()) {
            assert!(!text.contains(&name), "券上出现了联系人「{name}」");
        }
        assert!(!text.contains("9 月 17 日"), "券上出现了相遇日期");
        for a in crate::areas::AREAS.iter() {
            assert!(!text.contains(a.name), "券上出现了相遇片区「{}」", a.name);
        }
    }

    #[test]
    fn an_old_state_file_without_the_new_coupon_fields_still_loads() {
        // 新增字段全是 Option：旧 state.json 不能因为一次改版被整个丢掉。
        let json = r#"{"contacts":[],"encounters":[],"reward":{"venue":"禾间小馆","offer":"¥60","claimed":true,"redeemed":false}}"#;
        let p = PersistedState::deserialize_json(json).expect("旧券结构应仍可读");
        let r = p.reward.expect("券应还在");
        assert_eq!(r.expires_on, None);
        assert_eq!(r.expiry_label(), "有效期以券面为准");
    }

    // ---- 券包 ----

    #[test]
    fn the_wallet_sorts_into_available_redeemed_and_expired() {
        let today = civil_to_days(2026, 9, 17);
        let mut s = state();
        s.issue_reward(today - 10, 1); // 十天前发的，早过期了
        s.issue_reward(today, 2); // 今天发的，可用
        s.issue_reward(today, 3); // 今天发的，等下核销
        assert!(s.redeem_at(2));

        assert_eq!(s.rewards_in(today, RewardState::Available).len(), 1);
        assert_eq!(s.rewards_in(today, RewardState::Redeemed).len(), 1);
        assert_eq!(s.rewards_in(today, RewardState::Expired).len(), 1);
        // 核销过的券不会又被算成过期。
        let mut old = s.wallet[0].clone();
        old.redeemed = true;
        assert_eq!(old.state(today), RewardState::Redeemed);
    }

    #[test]
    fn an_expiring_coupon_says_how_long_is_left() {
        let today = civil_to_days(2026, 9, 17);
        let mut s = state();
        s.issue_reward(today - 5, 1); // 还剩 2 天
        let r = s.latest_reward().unwrap();
        assert_eq!(r.remaining_label(today).as_deref(), Some("还剩 2 天"));
        s.issue_reward(today - 7, 2); // 今天到期
        assert_eq!(
            s.latest_reward().unwrap().remaining_label(today).as_deref(),
            Some("今天最后一天")
        );
        // 刚发的券不催。
        s.issue_reward(today, 3);
        assert_eq!(s.latest_reward().unwrap().remaining_label(today), None);
    }

    #[test]
    fn deleting_can_be_undone_within_the_window() {
        let mut s = state();
        let before_contacts = s.contacts.len();
        let before_memories = s.encounters.len();
        let snap = s.snapshot_for_undo("已删除 林舟");
        assert!(s.delete_contact(0, true));
        assert!(s.contacts.len() < before_contacts);
        s.restore(snap);
        assert_eq!(s.contacts.len(), before_contacts);
        assert_eq!(s.encounters.len(), before_memories);
    }

    #[test]
    fn clearing_local_data_leaves_nothing_behind() {
        let mut s = state();
        s.issue_reward(today_days(), 1);
        s.publish(0, 1, crate::areas::AREAS[0].id, 0);
        s.clear_local_data();
        assert!(s.contacts.is_empty());
        assert!(s.encounters.is_empty());
        assert!(s.wallet.is_empty());
        assert!(s.recent_areas.is_empty());
        assert!(s.publish.is_none());
        // 清完再写一条回忆，id 不会和旧的撞上。
        s.push_memory(0, "林舟", MemoryChoice::Save);
        assert_eq!(s.encounters.len(), 1);
    }

    #[test]
    fn an_old_single_coupon_file_becomes_a_one_card_wallet() {
        let json = r#"{"contacts":[],"encounters":[],"reward":{"venue":"禾间小馆","offer":"¥60","claimed":true,"redeemed":false}}"#;
        let p = PersistedState::deserialize_json(json).expect("旧结构应仍可读");
        let mut s = state();
        s.apply_persisted(p);
        assert_eq!(s.wallet.len(), 1);
        assert_eq!(s.wallet[0].venue, "禾间小馆");
        // 写出去的时候不再填旧字段。
        assert!(s.persisted().reward.is_none());
    }

    #[test]
    fn notifications_are_off_until_asked_for() {
        // 02-features 七.4：两类通知都是 opt-in。
        let s = state();
        assert!(!s.settings.notify_publish);
        assert!(!s.settings.notify_reward);
    }

    // ---- 持久化 ----

    #[test]
    fn persist_roundtrip() {
        let mut s = state();
        s.push_memory(2, "许宁", MemoryChoice::Hidden);
        s.issue_reward(today_days(), 1);
        let p = s.persisted();
        let json = p.serialize_json();
        let back = PersistedState::deserialize_json(&json).expect("反序列化应成功");
        assert_eq!(p, back);
        // 短时数据不在持久化里。按键名比对 —— 设置里的 notify_publish
        // 也含 publish 三个字，用子串会误报。
        assert!(!json.contains("\"publish\""));
        assert!(!json.contains("\"echo\""));
    }

    #[test]
    fn persist_file_roundtrip() {
        let path = std::env::temp_dir().join(format!("ouyu-test-{}/state.json", std::process::id()));
        let mut s = state();
        s.delete_contact(1, false);
        s.save_to(&path).expect("写盘应成功");
        let loaded = OuyuState::load_from(&path).expect("读盘应成功");
        assert_eq!(s.persisted(), loaded);
        let mut t = state();
        t.apply_persisted(loaded);
        assert!(t.contact(1).is_none());
        assert_eq!(t.encounters[1].contact_id, None); // 保留的回忆靠快照
        assert_eq!(t.encounters[1].label_snapshot, "陈晓");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn persist_bad_json_is_none() {
        let path = std::env::temp_dir().join(format!("ouyu-bad-{}.json", std::process::id()));
        std::fs::write(&path, "{oops").unwrap();
        assert!(OuyuState::load_from(&path).is_none());
        let _ = std::fs::remove_file(&path);
        assert!(OuyuState::load_from(&path).is_none());
    }

    #[test]
    fn persist_legacy_file_migrates_contacts_only() {
        // Phase 0 旧格式：my_windows/cards/stealth/my_pos/亲密度全部忽略，
        // 只迁出联系人 name → label；保存后旧字段自然消失。
        let json = r#"{
            "contacts":[{"id":0,"name":"老王","intimacy":80,"last_met":"3个月前","area":0,"free_start":14.0,"free_end":17.0,"in_circle":true}],
            "my_windows":[{"day":"今天","start":14.0,"end":18.0,"area":0}],
            "cards":[{"contact":0,"template":0,"time":15.5,"status":{"Sent":[]},"venue":null}],
            "encounters":[{"date":"9/20","who":"老王","place":"壹碗面","note":""}],
            "directory":["张伟"],
            "stealth":true,
            "my_pos_x":123.0,
            "my_pos_y":456.0
        }"#;
        let path = std::env::temp_dir().join(format!("ouyu-legacy-{}.json", std::process::id()));
        std::fs::write(&path, json).unwrap();
        let p = OuyuState::load_from(&path).expect("旧文件应能迁移");
        assert_eq!(p.contacts.len(), 1);
        assert_eq!(p.contacts[0].label, "老王");
        assert!(p.encounters.is_empty()); // 旧记录（含地点）不带入新模型
        let _ = std::fs::remove_file(&path);
        // 保存后的新文件不再含任何旧字段。
        let mut s = state();
        s.apply_persisted(p);
        let out = s.persisted().serialize_json();
        for key in ["my_windows", "cards", "stealth", "my_pos", "intimacy", "place"] {
            assert!(!out.contains(key), "key {key} should be gone");
        }
    }

    #[test]
    fn persist_new_file_ignores_unknown_fields() {
        // 新版文件混入未知字段（比如未来版本）也能加载。
        let json = r#"{"contacts":[],"encounters":[],"directory":[],"reward":null,"next_encounter_id":0,"future_field":123}"#;
        let path = std::env::temp_dir().join(format!("ouyu-future-{}.json", std::process::id()));
        std::fs::write(&path, json).unwrap();
        let p = OuyuState::load_from(&path).expect("未知字段应被忽略");
        assert!(p.contacts.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    // ---- vCard ----

    #[test]
    fn vcard_basic() {
        let names = parse_vcard("BEGIN:VCARD\nVERSION:3.0\nFN:周子墨\nEND:VCARD\n");
        assert_eq!(names, vec!["周子墨".to_string()]);
    }

    #[test]
    fn vcard_multiple_and_params() {
        let text = "BEGIN:VCARD\nVERSION:4.0\nFN;CHARSET=UTF-8:吴凯文\nEND:VCARD\n\
                    BEGIN:VCARD\nVERSION:3.0\nFN:赵四\nTEL:123\nEND:VCARD\n";
        let mut names = parse_vcard(text);
        names.sort();
        assert_eq!(names, vec!["吴凯文".to_string(), "赵四".to_string()]);
    }

    #[test]
    fn vcard_line_folding() {
        let names = parse_vcard("BEGIN:VCARD\nFN:林小\n 满\nEND:VCARD\n");
        assert_eq!(names, vec!["林小满".to_string()]);
        let names = parse_vcard("BEGIN:VCARD\nFN:林\n\t小满\nEND:VCARD\n");
        assert_eq!(names, vec!["林小满".to_string()]);
    }

    #[test]
    fn vcard_n_fallback() {
        let names = parse_vcard("BEGIN:VCARD\nN:黄;一诺;;;\nEND:VCARD\n");
        assert_eq!(names, vec!["黄一诺".to_string()]);
        let names = parse_vcard("BEGIN:VCARD\nN:Doe;John;;;\nEND:VCARD\n");
        assert_eq!(names, vec!["John Doe".to_string()]);
        let names = parse_vcard("BEGIN:VCARD\nN:黄;一诺;;;\nFN:黄一诺（小号）\nEND:VCARD\n");
        assert_eq!(names, vec!["黄一诺（小号）".to_string()]);
    }

    #[test]
    fn vcard_sample_file_parses() {
        assert_eq!(
            parse_vcard(SAMPLE_VCARD),
            vec![
                "周子墨".to_string(),
                "林小满".to_string(),
                "黄一诺".to_string(),
                "吴凯文".to_string()
            ]
        );
    }

    // ---- 批次 5：分组、搜索、字母索引 ----

    #[test]
    fn a_memory_falls_into_the_month_of_its_date() {
        assert_eq!(month_head("2026-09-17"), "2026 年 9 月");
        assert_eq!(month_head("2025-01-03"), "2025 年 1 月");
        // 认不出来的日期归到「更早」，不是被丢掉。
        assert_eq!(month_head(""), "更早");
        assert_eq!(month_head("去年夏天"), "更早");
    }

    #[test]
    fn the_alphabet_index_reads_the_surname_not_the_nickname() {
        assert_eq!(alpha_key("林舟"), 'L');
        assert_eq!(alpha_key("老陈"), 'C');
        assert_eq!(alpha_key("小林"), 'L');
        assert_eq!(alpha_key("阿黄"), 'H');
        assert_eq!(alpha_key("Anna"), 'A');
        assert_eq!(alpha_key("bob"), 'B');
        // 认不出来的不猜，进 # 那一格。
        assert_eq!(alpha_key("喵喵"), '#');
        assert_eq!(alpha_key(""), '#');
        // 「小」本身是姓氏表里没有的字，剥掉之后也认不出，仍进 #。
        assert_eq!(alpha_key("小"), '#');
    }

    #[test]
    fn hidden_memories_never_show_up_in_search() {
        // 02 F：隐藏记录不进入提醒、搜索、AI 或推荐。
        let mut s = state();
        s.encounters[0].note = "在书店门口".into();
        let id = s.encounters[0].id;
        let hit = search_memories(&s.encounters, "书店", None);
        assert_eq!(hit.len(), 1);

        assert!(s.set_hidden(id, true));
        assert!(
            search_memories(&s.encounters, "书店", None).is_empty(),
            "隐藏之后还能搜到，这一条红线就破了"
        );
        // 连称呼也搜不到。
        let label = s.encounters.iter().find(|e| e.id == id).unwrap().label_snapshot.clone();
        assert!(search_memories(&s.encounters, &label, None)
            .iter()
            .all(|e| e.id != id));
    }

    #[test]
    fn search_only_looks_at_the_name_and_the_note() {
        let mut s = state();
        s.encounters[0].note = "聊了很久".into();
        assert_eq!(search_memories(&s.encounters, "聊了", None).len(), 1);
        // 日期不是搜索目标 —— 按日期找东西是分组头的活。
        let date = s.encounters[0].date.clone();
        assert!(search_memories(&s.encounters, &date, None).is_empty());
    }

    #[test]
    fn search_and_the_person_filter_stack() {
        let mut s = state();
        s.push_memory(0, "林舟", MemoryChoice::Save);
        let all = search_memories(&s.encounters, "", None).len();
        let only = search_memories(&s.encounters, "", Some("林舟")).len();
        assert!(only <= all);
        assert!(search_memories(&s.encounters, "", Some("林舟"))
            .iter()
            .all(|e| e.label_snapshot == "林舟"));
    }

    // ---- 批次 5：备注、手动添加、合并 ----

    #[test]
    fn a_note_can_be_written_and_rewritten() {
        let mut s = state();
        let id = s.encounters[0].id;
        assert!(s.set_note(id, "  在地铁口碰上的  "));
        assert_eq!(s.encounters[0].note, "在地铁口碰上的");
        // 写成一样的不算改动。
        assert!(!s.set_note(id, "在地铁口碰上的"));
        assert!(!s.set_note(9999, "不存在的那条"));
    }

    #[test]
    fn adding_a_contact_rejects_blank_and_duplicate_names() {
        let mut s = state();
        let n = s.contacts.len();
        assert_eq!(s.add_contact("   "), Err(AddContactError::Empty));
        assert_eq!(
            s.add_contact("这个称呼实在是太长了根本写不完还在写"),
            Err(AddContactError::TooLong)
        );
        let existing = s.contacts[0].label.clone();
        assert_eq!(s.add_contact(&existing), Err(AddContactError::Duplicate));
        assert_eq!(s.contacts.len(), n);

        let id = s.add_contact(" 沈思远 ").unwrap();
        assert_eq!(s.contacts.len(), n + 1);
        assert_eq!(s.contact(id).unwrap().label, "沈思远");
    }

    #[test]
    fn adding_a_contact_takes_them_out_of_the_directory_pool() {
        let mut s = state();
        let name = s.directory[0].clone();
        s.add_contact(&name).unwrap();
        assert!(!s.directory.contains(&name), "加过的人不该还挂在「可添加」里");
    }

    #[test]
    fn merging_two_contacts_keeps_every_memory() {
        let mut s = state();
        let keep = s.contacts[0].id;
        let gone = s.add_contact("老陈").unwrap();
        s.push_memory(gone, "老陈", MemoryChoice::Save);
        s.push_memory(gone, "老陈", MemoryChoice::Hidden);
        let before = s.encounters.len();
        let keep_label = s.contact(keep).unwrap().label.clone();
        let keep_before = s.meeting_count(keep);

        let moved = s.merge_contacts(gone, keep).unwrap();
        assert_eq!(moved, 2);
        assert_eq!(s.encounters.len(), before, "合并不该丢掉任何一条回忆");
        assert!(s.contact(gone).is_none());
        assert_eq!(s.meeting_count(keep), keep_before + 2);
        // 称呼快照一起转过去，回忆页上不会留着一个已经不存在的名字。
        assert!(s
            .encounters
            .iter()
            .all(|e| e.label_snapshot != "老陈"));
        assert!(s
            .encounters
            .iter()
            .filter(|e| e.contact_id == Some(keep))
            .all(|e| e.label_snapshot == keep_label));
    }

    #[test]
    fn merging_refuses_nonsense() {
        let mut s = state();
        let a = s.contacts[0].id;
        assert_eq!(s.merge_contacts(a, a), None, "自己并进自己");
        assert_eq!(s.merge_contacts(a, 9999), None, "并进一个不存在的人");
        assert_eq!(s.merge_contacts(9999, a), None, "从一个不存在的人并出来");
    }

    // ---- 批次 5：通知 ----

    #[test]
    fn notifications_stay_silent_until_you_turn_them_on() {
        let mut s = state();
        s.publish(0, 1, crate::areas::AREAS[0].id, 0);
        s.issue_reward(SAT, 1);
        // 两个开关都是关的 —— 默认关闭是 02 七.4 写死的。
        assert!(!s.settings.notify_publish);
        assert!(!s.settings.notify_reward);
        assert!(due_notices(&s, SAT, 17 * 60 + 45).is_empty());
    }

    #[test]
    fn the_trip_notice_fires_only_in_the_last_half_hour() {
        let mut s = state();
        s.settings.notify_publish = true;
        s.publish(0, 1, crate::areas::AREAS[0].id, 0); // 下午，18:00 结束
        assert!(due_notices(&s, SAT, 15 * 60).is_empty(), "还早");
        assert!(due_notices(&s, SAT, 18 * 60).is_empty(), "已经过了");
        let n = due_notices(&s, SAT, 17 * 60 + 45);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].kind, NoticeKind::PublishExpiring);
        assert!(n[0].text.contains("15 分钟"));
    }

    #[test]
    fn the_trip_notice_ignores_a_trip_that_is_not_today() {
        let mut s = state();
        s.settings.notify_publish = true;
        s.publish(3, 1, crate::areas::AREAS[0].id, 0);
        assert!(due_notices(&s, SAT, 17 * 60 + 45).is_empty());
    }

    #[test]
    fn the_coupon_notice_fires_in_the_last_two_days() {
        let mut s = state();
        s.settings.notify_reward = true;
        s.issue_reward(SAT, 1);
        let exp = s.wallet[0].expires_on.unwrap();
        assert!(due_notices(&s, SAT, 9 * 60).is_empty(), "刚发的券不提醒");
        let n = due_notices(&s, exp - 2, 9 * 60);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].kind, NoticeKind::RewardExpiring);
        // 最后一天换一句话。
        let n = due_notices(&s, exp, 9 * 60);
        assert!(n[0].text.contains("今天最后一天"));
        // 过期之后不再提醒 —— 提醒一张已经没用的券只会让人白跑一趟。
        assert!(due_notices(&s, exp + 1, 9 * 60).is_empty());
    }

    #[test]
    fn a_redeemed_coupon_stops_nagging() {
        let mut s = state();
        s.settings.notify_reward = true;
        s.issue_reward(SAT, 1);
        let exp = s.wallet[0].expires_on.unwrap();
        assert_eq!(due_notices(&s, exp - 1, 9 * 60).len(), 1);
        assert!(s.redeem_at(0));
        assert!(due_notices(&s, exp - 1, 9 * 60).is_empty());
    }

    #[test]
    fn notifications_never_mention_another_person_or_a_place_of_theirs() {
        // 02 B 的红线：通知不能变成实时位置广播。
        let mut s = state();
        s.settings.notify_publish = true;
        s.settings.notify_reward = true;
        s.publish(0, 1, crate::areas::AREAS[0].id, 0);
        s.issue_reward(SAT, 1);
        let exp = s.wallet[0].expires_on.unwrap();
        let mut all: Vec<Notice> = due_notices(&s, SAT, 17 * 60 + 45);
        all.extend(due_notices(&s, exp - 1, 17 * 60 + 45));
        assert!(!all.is_empty());
        for n in &all {
            for bad in ["附近", "熟人", "有人", "米", "公里", "正在"] {
                assert!(!n.text.contains(bad), "通知里出现了「{bad}」：{}", n.text);
            }
            for c in &s.contacts {
                assert!(!n.text.contains(&c.label), "通知里出现了联系人：{}", n.text);
            }
        }
    }

    #[test]
    fn merge_directory_dedups() {
        let mut s = state();
        let added = s.merge_directory(vec![
            "沈思远".into(), // 新
            "周子墨".into(), // 通讯录已有
            "林舟".into(),   // 熟人已有
            "  ".into(),     // 空白忽略
            "沈思远".into(), // 同批重复
        ]);
        assert_eq!(added, 1);
        assert_eq!(s.directory.last().unwrap(), "沈思远");
    }
}

