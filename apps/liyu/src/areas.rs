//! 城市区域库 —— 发布「哪一带」的唯一数据源。
//!
//! 偶遇不接真实地图（docs/02 决策 3）：不接地图，就必须靠一份好的区域库把
//! 「填地址」做简单。这里收的是**片区**，不是地点。
//!
//! # 收录标准（隐私约束，改动前先读 design/02-features.md A 节）
//!
//! - 只收 **≥1km 的公共片区**：商圈、公园绿地、滨水步道、文化街区、办公园区、
//!   校园周边、交通枢纽、大型生活片区。
//! - **不收**单个楼宇、门店、住宅小区、医院、诊所、学校内部、宗教场所、政府机构。
//! - 名称一律用「…一带 / …周边 / …公共街区 / …城区」的模糊措辞，不给精确地标。
//!   `alias` 里可以放常见俗称（供搜索命中），但**不会显示给任何人**，也不进发布文案。
//!
//! 换城市 / 扩城市只需往 `AREAS` 里加数据，UI 不用动。

/// 片区类型：只用于筛选与图标，不进入匹配输入。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaKind {
    /// 商圈、步行街
    Commercial,
    /// 公园绿地
    Park,
    /// 滨水、河湖步道
    Waterfront,
    /// 历史文化街区
    Culture,
    /// 办公园区、产业区
    Office,
    /// 高校周边（校园外的公共街区，不含校内）
    Campus,
    /// 交通枢纽周边
    Transit,
    /// 大型生活片区
    Neighborhood,
}

impl AreaKind {
    pub fn label(self) -> &'static str {
        match self {
            AreaKind::Commercial => "商圈",
            AreaKind::Park => "公园",
            AreaKind::Waterfront => "滨水",
            AreaKind::Culture => "文化",
            AreaKind::Office => "园区",
            AreaKind::Campus => "校园",
            AreaKind::Transit => "枢纽",
            AreaKind::Neighborhood => "生活",
        }
    }

    /// 筛选条的顺序（「全部」在 UI 侧另加）。
    pub const ALL: [AreaKind; 8] = [
        AreaKind::Commercial,
        AreaKind::Park,
        AreaKind::Waterfront,
        AreaKind::Culture,
        AreaKind::Office,
        AreaKind::Campus,
        AreaKind::Transit,
        AreaKind::Neighborhood,
    ];
}

/// 一个公共片区。`id` 是持久标识，发布与「最近去过」都存它，不存下标。
#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub id: u16,
    pub name: &'static str,
    pub district: &'static str,
    pub kind: AreaKind,
    /// 俗称，只用于搜索命中，不展示、不进发布文案。
    pub alias: &'static str,
    /// 拼音首字母，用于「sltyd」这类快速搜索。
    pub py: &'static str,
}

/// 城市名（MVP 单城市，02 概念：MVP = 一个城市）。
pub const CITY: &str = "北京";

use AreaKind::*;

macro_rules! areas {
    ($($id:expr, $name:expr, $district:expr, $kind:expr, $alias:expr, $py:expr;)*) => {
        pub const AREAS: &[Area] = &[
            $(Area { id: $id, name: $name, district: $district, kind: $kind, alias: $alias, py: $py },)*
        ];
    };
}

areas! {
    // ---- 朝阳区 ----
    1,  "三里屯一带",         "朝阳区", Commercial,   "太古里 工体 酒吧街",   "slty slt sltyd";
    2,  "国贸公共街区",       "朝阳区", Office,       "CBD 建外 金台夕照",    "gmgggj gm cbd";
    3,  "朝阳公园周边",       "朝阳区", Park,         "蓝色港湾",             "cygyzb cygy";
    4,  "望京一带",           "朝阳区", Office,       "阜通 望京西",          "wjyd wj";
    5,  "亚运村一带",         "朝阳区", Park,         "奥体 鸟巢 水立方",     "yycyd yyc";
    6,  "双井一带",           "朝阳区", Commercial,   "富力城 九龙山",        "sjyd sj";
    7,  "东大桥一带",         "朝阳区", Commercial,   "芳草地 日坛",          "ddqyd ddq";
    8,  "酒仙桥艺术区一带",   "朝阳区", Culture,      "798 大山子",           "jxqysqyd 798";
    9,  "大望路一带",         "朝阳区", Commercial,   "万达广场 华贸",        "dwlyd dwl";
    10, "太阳宫一带",         "朝阳区", Neighborhood, "芳园里",               "tygyd tyg";
    11, "劲松一带",           "朝阳区", Neighborhood, "潘家园",               "jsyd js";
    12, "十里堡一带",         "朝阳区", Commercial,   "朝阳路",               "slpyd slp";
    13, "朝阳大悦城一带",     "朝阳区", Commercial,   "青年路 褡裢坡",        "cydycyd dyc";
    14, "四惠一带",           "朝阳区", Transit,      "四惠东 通惠河",        "shyd sh";
    15, "燕莎一带",           "朝阳区", Commercial,   "亮马桥 三元桥 使馆区", "ysyd ys lmq";
    16, "团结湖一带",         "朝阳区", Park,         "农展馆",               "tjhyd tjh";
    17, "常营一带",           "朝阳区", Neighborhood, "北京像素",             "cyyd cy";
    18, "奥林匹克森林公园周边", "朝阳区", Park,       "奥森",                 "olpkslgyzb aosen";
    19, "亮马河滨水步道一带", "朝阳区", Waterfront,   "亮马河 蓝港",          "lmhbsbdyd lmh";
    20, "垡头一带",           "朝阳区", Neighborhood, "欢乐谷",               "ftyd ft";

    // ---- 海淀区 ----
    31, "中关村一带",         "海淀区", Office,       "海淀黄庄 科学院",      "zgcyd zgc";
    32, "五道口一带",         "海淀区", Campus,       "成府路 华清",          "wdkyd wdk";
    33, "西二旗一带",         "海淀区", Office,       "后厂村",               "xeqyd xeq";
    34, "上地一带",           "海淀区", Office,       "上地信息产业基地",     "sdyd sd";
    35, "颐和园周边",         "海淀区", Park,         "北宫门 青龙桥",        "yhyzb yhy";
    36, "圆明园周边",         "海淀区", Park,         "清华西门",             "ymyzb ymy";
    37, "学院路一带",         "海淀区", Campus,       "北航 北科大",          "xylyd xyl";
    38, "万柳一带",           "海淀区", Neighborhood, "巴沟 万泉河",          "wlyd wl";
    39, "公主坟一带",         "海淀区", Transit,      "翠微 城乡",            "gzfyd gzf";
    40, "西直门一带",         "海淀区", Transit,      "北京北站 动物园",      "xzmyd xzm";
    41, "清河一带",           "海淀区", Neighborhood, "清河站 五彩城",        "qhyd qh";
    42, "苏州街一带",         "海淀区", Commercial,   "海淀公园",             "szjyd szj";
    43, "香山周边",           "海淀区", Park,         "植物园 卧佛寺",        "xszb xs";
    44, "紫竹院周边",         "海淀区", Park,         "国图 白石桥",          "zzyzb zzy";
    45, "五棵松一带",         "海淀区", Commercial,   "华熙 卓展",            "wksyd wks";
    46, "金源一带",           "海淀区", Commercial,   "远大路 世纪金源",      "jyyd jy";
    47, "中关村软件园一带",   "海淀区", Office,       "软件园",               "zgcrjyyd rjy";
    48, "北太平庄一带",       "海淀区", Campus,       "北师大 蓟门桥",        "btpzyd btpz";
    49, "魏公村一带",         "海淀区", Campus,       "民大 国图",            "wgcyd wgc";
    50, "永定河休闲森林公园周边", "海淀区", Waterfront, "永定河",             "ydhxxslgyzb ydh";

    // ---- 东城区 ----
    61, "王府井一带",         "东城区", Commercial,   "东单 金鱼胡同",        "wfjyd wfj";
    62, "南锣鼓巷一带",       "东城区", Culture,      "鼓楼 什刹海东",        "nlgxyd nlgx";
    63, "故宫周边",           "东城区", Culture,      "景山 东华门 五四大街", "ggzb gg";
    64, "天坛周边",           "东城区", Park,         "天坛公园 永定门",      "ttzb tt";
    65, "东直门一带",         "东城区", Transit,      "东直门枢纽 香河园",    "dzmyd dzm";
    66, "崇文门一带",         "东城区", Commercial,   "新世界 花市",          "cwmyd cwm";
    67, "雍和宫一带",         "东城区", Culture,      "国子监 五道营",        "yhgyd yhg";
    68, "前门大栅栏一带",     "东城区", Culture,      "前门 鲜鱼口 珠市口",   "qmdzlyd qmdzl";
    69, "簋街一带",           "东城区", Commercial,   "北新桥 东直门内",      "gjyd gj";
    70, "朝阳门一带",         "东城区", Office,       "银河 悠唐",            "cymyd cym";
    71, "地坛公园周边",       "东城区", Park,         "安定门",               "dtgyzb dtgy";

    // ---- 西城区 ----
    81, "西单一带",           "西城区", Commercial,   "灵境胡同 大悦城",      "xdyd xd";
    82, "金融街一带",         "西城区", Office,       "复兴门 太平桥",        "jrjyd jrj";
    83, "什刹海一带",         "西城区", Waterfront,   "后海 烟袋斜街 银锭桥", "sshyd ssh hh";
    84, "德胜门一带",         "西城区", Transit,      "积水潭 新街口",        "dsmyd dsm";
    85, "月坛一带",           "西城区", Park,         "三里河",               "ytyd yt";
    86, "广安门一带",         "西城区", Neighborhood, "牛街 白广路",          "gamyd gam";
    87, "菜市口一带",         "西城区", Culture,      "南横街 湖广会馆",      "cskyd csk";
    88, "白塔寺一带",         "西城区", Culture,      "阜成门 宫门口",        "btsyd bts";
    89, "陶然亭公园周边",     "西城区", Park,         "陶然亭",               "trtgyzb trt";
    90, "宣武门一带",         "西城区", Commercial,   "西单南 达智桥",        "xwmyd xwm";
    91, "北京展览馆周边",     "西城区", Culture,      "动物园 西外大街",      "bjzlgzb bjzlg";

    // ---- 丰台区 ----
    101, "丽泽一带",          "丰台区", Office,       "丽泽商务区 金中都",    "lzyd lz";
    102, "方庄一带",          "丰台区", Neighborhood, "蒲黄榆 时代广场",      "fzyd fz";
    103, "马家堡一带",        "丰台区", Neighborhood, "角门 草桥",            "mjbyd mjb";
    104, "总部基地一带",      "丰台区", Office,       "科技园 花乡",          "zbjdyd zbjd";
    105, "北京南站周边",      "丰台区", Transit,      "南站 陶然桥",          "bjnzzb bjnz";
    106, "赵公口一带",        "丰台区", Transit,      "刘家窑",               "zgkyd zgk";
    107, "青塔一带",          "丰台区", Neighborhood, "西局 卢沟桥东",        "qtyd qt";
    108, "园博园周边",        "丰台区", Park,         "园博湖 永定河西",      "ybyzb yby";
    109, "南苑一带",          "丰台区", Neighborhood, "大红门 和义",          "nyyd ny";

    // ---- 石景山区 ----
    121, "苹果园一带",        "石景山区", Transit,    "苹果园枢纽",           "pgyyd pgy";
    122, "八角一带",          "石景山区", Park,       "游乐园 雕塑公园",      "bjyd bj";
    123, "首钢园一带",        "石景山区", Culture,    "首钢 三高炉",          "sgyyd sgy";
    124, "鲁谷一带",          "石景山区", Neighborhood, "衙门口",             "lgyd lg";
    125, "石景山万达一带",    "石景山区", Commercial, "古城 老山",            "sjswdyd sjs";

    // ---- 通州区 ----
    141, "北京城市副中心",    "通州区", Office,       "运河商务区 副中心",    "bjcsfzx fzx";
    142, "通州万达一带",      "通州区", Commercial,   "新华大街 北苑",        "tzwdyd tzwd";
    143, "大运河森林公园周边", "通州区", Waterfront,  "运河 月岛",            "dyhslgyzb dyh";
    144, "梨园一带",          "通州区", Neighborhood, "九棵树",               "lyyd ly";
    145, "宋庄艺术区一带",    "通州区", Culture,      "宋庄 小堡",            "szysqyd sz";

    // ---- 昌平区 ----
    161, "回龙观一带",        "昌平区", Neighborhood, "龙泽 霍营",            "hlgyd hlg";
    162, "天通苑一带",        "昌平区", Neighborhood, "立水桥",               "ttyyd tty";
    163, "沙河一带",          "昌平区", Waterfront,   "沙河水库 高教园",      "shyd shahe";
    164, "昌平城区",          "昌平区", Neighborhood, "鼓楼西街 政府街",      "cpcq cp";
    165, "未来科学城一带",    "昌平区", Office,       "科学城 滨水公园",      "wlkscyd wlkc";

    // ---- 大兴区 ----
    181, "亦庄一带",          "大兴区", Office,       "经开区 荣京",          "yzyd yz";
    182, "黄村一带",          "大兴区", Commercial,   "兴华大街 清源",        "hcyd hc";
    183, "西红门一带",        "大兴区", Commercial,   "荟聚",                 "xhmyd xhm";
    184, "大兴机场周边",      "大兴区", Transit,      "新机场 临空区",        "dxjczb dxjc";
    185, "旧宫一带",          "大兴区", Neighborhood, "小红门",               "jgyd jg";

    // ---- 顺义区 ----
    201, "后沙峪一带",        "顺义区", Neighborhood, "空港 罗马湖",          "hsyyd hsy";
    202, "顺义城区",          "顺义区", Commercial,   "府前街 石园",          "sycq sy";
    203, "国展新馆周边",      "顺义区", Transit,      "新国展 天竺",          "gzxgzb gzxg";
    204, "温榆河公园周边",    "顺义区", Waterfront,   "温榆河",               "wyhgyzb wyh";

    // ---- 房山区 ----
    221, "良乡一带",          "房山区", Campus,       "大学城 拱辰",          "lxyd lx";
    222, "长阳一带",          "房山区", Neighborhood, "长阳半岛 篮球公园",    "cyyd changyang";
    223, "房山城区",          "房山区", Neighborhood, "城关 燕山",            "fscq fs";

    // ---- 门头沟区 ----
    241, "门头沟城区",        "门头沟区", Neighborhood, "新城 石门营",        "mtgcq mtg";
    242, "永定河门城湖一带",  "门头沟区", Waterfront, "门城湖 滨河路",        "ydhmchyd mch";

    // ---- 远郊 ----
    261, "怀柔城区",          "怀柔区", Neighborhood, "青春路 迎宾路",        "hrcq hr";
    262, "雁栖湖周边",        "怀柔区", Waterfront,   "雁栖湖",               "yxhzb yxh";
    271, "密云城区",          "密云区", Neighborhood, "鼓楼 新西路",          "mycq my";
    272, "密云水库周边",      "密云区", Waterfront,   "水库 白河",            "myskzb mysk";
    281, "平谷城区",          "平谷区", Neighborhood, "府前街 新平北路",      "pgcq pg";
    291, "延庆城区",          "延庆区", Neighborhood, "妫川广场",             "yqcq yq";
    292, "妫河一带",          "延庆区", Waterfront,   "妫河森林公园",         "ghyd gh";
}

/// 按 id 取片区。
pub fn area(id: u16) -> Option<&'static Area> {
    AREAS.iter().find(|a| a.id == id)
}

/// 取不到就退回第一条，保证 UI 永远有东西可显示。
pub fn area_or_first(id: u16) -> &'static Area {
    area(id).unwrap_or(&AREAS[0])
}

/// 发布文案里用的片区名。
pub fn area_name(id: u16) -> &'static str {
    area_or_first(id).name
}

/// 行政区顺序（按 `AREAS` 里首次出现的次序，列表分组用）。
pub fn districts() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for a in AREAS {
        if !out.contains(&a.district) {
            out.push(a.district);
        }
    }
    out
}

/// 搜索：名称 / 行政区 / 俗称的中文子串，或拼音首字母前缀。
///
/// 空查询返回全部（按库内顺序，即行政区分组顺序）。命中顺序：名称前缀 →
/// 拼音前缀 → 其它，让「三里屯」「sltyd」都把三里屯排在最前。
pub fn search(query: &str, kind: Option<AreaKind>) -> Vec<&'static Area> {
    let q = query.trim().to_lowercase();
    let mut scored: Vec<(u8, usize, &'static Area)> = Vec::new();
    for (i, a) in AREAS.iter().enumerate() {
        if let Some(k) = kind {
            if a.kind != k {
                continue;
            }
        }
        if q.is_empty() {
            scored.push((2, i, a));
            continue;
        }
        let rank = if a.name.starts_with(&q) {
            0
        } else if a.py.split(' ').any(|p| p.starts_with(&q)) {
            1
        } else if a.name.contains(&q) || a.district.contains(&q) || a.alias.to_lowercase().contains(&q) {
            2
        } else {
            continue;
        };
        scored.push((rank, i, a));
    }
    scored.sort_by_key(|(r, i, _)| (*r, *i));
    scored.into_iter().map(|(_, _, a)| a).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_area_has_a_stable_unique_id() {
        let mut ids: Vec<u16> = AREAS.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "区域 id 必须唯一：发布与「最近去过」存的是 id");
        assert!(AREAS.len() >= 100, "区域库太小，填地址会重新变成将就");
    }

    #[test]
    fn area_names_stay_vague_enough_to_be_a_district_not_a_place() {
        for a in AREAS {
            let ok = ["一带", "周边", "公共街区", "城区", "副中心"]
                .iter()
                .any(|suffix| a.name.ends_with(suffix));
            assert!(ok, "{} 的措辞太精确了，片区名必须模糊", a.name);
        }
    }

    #[test]
    fn the_library_holds_no_building_clinic_or_campus_interior() {
        // 02 A：住宅小区、医院、学校内部等敏感具体去向不做标签。
        for a in AREAS {
            for bad in ["医院", "诊所", "小区", "大厦", "写字楼", "中学", "幼儿园"] {
                assert!(!a.name.contains(bad), "{} 违反收录标准", a.name);
            }
        }
    }

    #[test]
    fn searching_by_chinese_alias_or_pinyin_all_reach_the_same_area() {
        for q in ["三里屯", "太古里", "sltyd", "slt"] {
            let hits = search(q, None);
            assert_eq!(hits[0].name, "三里屯一带", "查询 {q} 没有把三里屯排在最前");
        }
    }

    #[test]
    fn an_empty_query_lists_everything_grouped_by_district() {
        let all = search("", None);
        assert_eq!(all.len(), AREAS.len());
        assert_eq!(all[0].district, "朝阳区");
    }

    #[test]
    fn a_kind_filter_narrows_without_breaking_search() {
        let parks = search("", Some(AreaKind::Park));
        assert!(!parks.is_empty());
        assert!(parks.iter().all(|a| a.kind == AreaKind::Park));
        let hits = search("公园", Some(AreaKind::Park));
        assert!(hits.iter().any(|a| a.name == "朝阳公园周边"));
    }
}
