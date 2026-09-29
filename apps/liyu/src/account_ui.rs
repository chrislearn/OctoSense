//! 「我 / 设置」二三级界面的可复用布局与详情组件（账号区）。
//!
//! 三种壳按形态各取其一，行 / 操作区两种原语两边共用：
//!
//! - 宽屏（桌面 / 平板）：[`AccountSplitView`] —— 左侧分类列表、右侧当前分类
//!   的详情区，一屏看全，不用前进返回。
//! - 窄屏（手机）：[`AccountListView`] → [`AccountDetailView`] →
//!   [`AccountEditView`]，逐层前进 / 返回；三层同挂在一个 Overlay 容器里，
//!   由集成方按当前层级切换 `visible`。
//!
//! 两种原语：
//!
//! - [`AccountInfoRow`]：标签 + 当前值 + 右侧操作按钮（「修改 / 管理 /
//!   选择图片…」按字段类型）。头像行 [`AccountAvatarRow`] 复用「个人资料」页
//!   既有的选图 / 预览 / 裁切 / 上传控件结构（`LiyuThumb` + 「选择图片…」+
//!   「裁切位置 / 旋转 90° / 确认上传 / 取消」），不另造一套。
//! - [`AccountEditActions`]：编辑页底部统一的「取消 / 保存」操作区；手机号 /
//!   邮箱这类绑定字段把保存钮文案换成「确认绑定」由集成方改 `text`。
//!
//! 集成方式（master 串行接线，本文件不改任何既有文件）：
//!
//! 1. `lib.rs` 加 `mod account_ui;`；
//! 2. 在 `LIYU_MODULE::register`（以及 main.rs 的 `App::script_mod`、theme.rs
//!    的 `restyle` 重跑处）里 `canvas::script_mod(vm)` 之后、
//!    `crate::script_mod(vm)` 之前加一行 `account_ui::script_mod(vm);` —— 预设
//!    全部落在共享的 `mod.widgets` 命名空间里，注册一次到处可用；
//! 3. 页面 DSL 里直接实例化，例如 `AccountSplitView { }`、`AccountInfoRow { }`；
//! 4. 分类列表数据驱动：Rust 侧遍历 [`ACCOUNT_CATEGORIES`]，按
//!    [`AccountCategory::title`] 填 `acc_cat0..acc_cat6`（CheckBoxFlat 用
//!    `set_selected` / `sync_tabs` 同款接线），点中哪个就把右侧
//!    `acc_detail_body` 换成对应分类的内容（详情区是占位容器，内容由 master
//!    按分类填）；
//! 5. 按钮接线：编辑页操作区按钮 id 统一为 `acc_edit_cancel` /
//!    `acc_edit_save`（宽屏面板里的前缀 `acc_panel_`，见
//!    [`ACCOUNT_EDIT_CANCEL_ID`] / [`ACCOUNT_EDIT_SAVE_ID`]），handler 按 id
//!    取 `button(id!...)`。
//!
//! 本组件只做布局与结构，不自带页面路由：它塞进既有「我」页的内容区，全应用
//! 侧栏 / 底部导航由 LiyuView 的外壳负责，这里不遮挡、不替换。

use makepad_widgets::*;

script_mod! {
    use mod.prelude.liyu.*
    use mod.widgets.*

    // ---------------------------------------------------------------
    // 分类行（左右两壳共用）：整行可点，选中时高亮块 + 亮字。
    // 与 LiyuTab 同一个 CheckBoxFlat 套路，但窄一档（账号区是二级导航，
    // 不该和全局侧栏同字号抢层级）。
    // ---------------------------------------------------------------
    mod.widgets.AccountCategoryRow = mod.widgets.CheckBoxFlat{
        width: Fill
        height: 38
        padding: Inset{left: 12.0, right: 12.0}
        align: Align{x: 0.0, y: 0.5}
        label_walk +: { margin: Inset{left: 0.0} }
        draw_bg +: {
            border_radius: r.button
            color: #0000
            color_hover: liyu.hl_soft
            color_down: liyu.hl
            color_focus: #0000
            color_active: liyu.hl
            border_size: 0.0
            border_color: #0000
            pixel: fn(){
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let r = min(self.border_radius, self.rect_size.y * 0.5)
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, r)
                let fill = self.color
                    .mix(self.color_focus, self.focus)
                    .mix(self.color_active, self.active)
                    .mix(self.color_hover, self.hover)
                    .mix(self.color_down, self.down)
                return sdf.fill(fill)
            }
        }
        draw_text +: {
            color: liyu.ink_2
            color_hover: liyu.ink
            color_down: liyu.ink
            color_focus: liyu.ink_2
            color_active: liyu.ink
            text_style +: { font_size: 13.5 }
        }
    }

    // ---------------------------------------------------------------
    // 资料行：标签 + 当前值 + 右侧操作钮，整行可点（Overlay：下层显示、
    // 上层铺满的透明按钮接点击，同 LiyuSetRow / LiyuGiftRow 的套路）。
    //
    // 三类字段共用一个预设，右侧只露该露的：
    // - 文本字段（显示名 / 签名 …）：air_value + air_edit（「修改」）；
    // - 管理类（收货地址 / 通知开关 …）：air_sub 说明 + air_manage（「管理」）
    //   + air_arrow 箭头；
    // - 头像 / 图片字段：用 AccountAvatarRow（见下），不强塞进这一条。
    // 集成方按字段类型 set_text / 切 visible，按钮 id 固定不变。
    // ---------------------------------------------------------------
    mod.widgets.AccountInfoRow = mod.widgets.View{
        width: Fill height: Fit
        flow: Overlay
        air_body := mod.widgets.View {
            width: Fill height: Fit
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            padding: Inset{left: 12.0, right: 12.0, top: 11.0, bottom: 11.0}
            spacing: 10.0
            air_col := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 2.0
                air_label := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    max_lines: 1
                    text_overflow: Ellipsis
                    text: ""
                    draw_text +: { color: liyu.ink text_style +: { font_size: 15.0 } }
                }
                air_sub := Label {
                    flow: Right{wrap: true}
                    visible: false
                    width: Fill
                    text: ""
                    draw_text +: { wrap: Words color: liyu.ink_3 text_style +: { font_size: 12.0 } }
                }
            }
            air_value := Label {
                flow: Right{wrap: true}
                width: Fit
                max_lines: 1
                text_overflow: Ellipsis
                text: ""
                draw_text +: { color: liyu.ink_2 text_style +: { font_size: 13.0 } }
            }
            air_edit := mod.widgets.LiyuBtnSm { visible: false width: Fit text: "修改" }
            air_manage := mod.widgets.LiyuBtnSm { visible: false width: Fit text: "管理" }
            air_arrow := mod.widgets.LiyuIcon {
                icon_walk: Walk{ width: 13.0 height: Fit }
                draw_icon +: { svg: crate_resource("self:resources/icons/chevron-right.svg") color: liyu.ink_arrow }
            }
        }
        air_hit := mod.widgets.ButtonFlat {
            width: Fill height: Fill
            text: ""
            margin: 0.0
            padding: 0.0
            draw_bg +: {
                border_size: 0.0
                border_radius: r.button
                color: #0000
                color_hover: liyu.wash_2
                color_down: liyu.wash_3
                color_focus: liyu.wash_1
            }
        }
    }

    // ---------------------------------------------------------------
    // 头像资料行：预览图 + 状态文 + 「选择图片…」；点开过选图后再露
    // 裁切 / 旋转 / 确认上传 / 取消 这一排。结构照搬「个人资料」页的
    // pf_avatar_* 控件组合（lib.rs），账号区不再另造一套头像控件；
    // 图像处理仍走 avatar.rs 的 AvatarEditSession。
    // ---------------------------------------------------------------
    mod.widgets.AccountAvatarRow = mod.widgets.View{
        width: Fill height: Fit
        flow: Down
        spacing: 8.0
        padding: Inset{left: 12.0, right: 12.0, top: 11.0, bottom: 11.0}
        aar_label := Label {
            flow: Right{wrap: true}
            width: Fill
            text: "头像"
            draw_text +: { color: liyu.ink_3 text_style +: { font_size: 12.0 } }
        }
        aar_row := mod.widgets.View {
            width: Fill height: Fit
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            spacing: 10.0
            aar_thumb := mod.widgets.LiyuThumb { width: 64 height: 64 }
            aar_col := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 6.0
                aar_status := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    text: "还没有头像，显示默认占位图"
                    draw_text +: { wrap: Words color: liyu.ink_2 text_style +: { font_size: 13.0 } }
                }
                aar_pick_row := mod.widgets.View {
                    width: Fill height: Fit
                    flow: Right
                    align: Align{x: 0.0, y: 0.5}
                    spacing: 8.0
                    aar_pick := mod.widgets.LiyuBtn { width: Fit text: "选择图片…" }
                    aar_delete := mod.widgets.LiyuBtnSm { visible: false width: Fit text: "删除头像，恢复默认" }
                }
            }
        }
        // 编辑排：选图载入后才出现（裁切位置九宫格循环 / 旋转 90° / 确认上传 / 取消）。
        aar_edit := mod.widgets.View {
            visible: false
            width: Fill height: Fit
            flow: Down
            spacing: 8.0
            aar_edit_note := Label {
                flow: Right{wrap: true}
                width: Fill
                text: "已载入原图：选裁切位置（九宫格循环）、旋转，确认后上传。"
                draw_text +: { wrap: Words color: liyu.ink_2 text_style +: { font_size: 12.5 } }
            }
            aar_edit_row := mod.widgets.View {
                width: Fill height: Fit
                flow: Right{wrap: true}
                wrap_spacing: 8.0
                spacing: 8.0
                aar_anchor := mod.widgets.LiyuBtnSm { width: Fit text: "裁切位置：居中" }
                aar_rotate := mod.widgets.LiyuBtnSm { width: Fit text: "旋转 90°" }
                aar_confirm := mod.widgets.LiyuBtnPrimary { width: Fit text: "确认上传" }
                aar_cancel := mod.widgets.LiyuBtnSm { width: Fit text: "取消" }
            }
        }
    }

    // ---------------------------------------------------------------
    // 编辑页底部统一操作区：左「取消」（次钮）右「保存」（主钮）。
    // 手机号 / 邮箱这类绑定字段由集成方把 acc_edit_save 的 text 换成
    // 「确认绑定」，布局不变。两个按钮的 id 全应用唯一（宽屏面板里的
    // 那对用 acc_panel_ 前缀），handler 按 id 接线。
    // ---------------------------------------------------------------
    mod.widgets.AccountEditActions = mod.widgets.View{
        width: Fill height: Fit
        flow: Right
        align: Align{x: 1.0, y: 0.5}
        spacing: 10.0
        padding: Inset{top: 4.0}
        acc_edit_cancel := mod.widgets.LiyuBtn { width: Fit text: "取消" }
        acc_edit_save := mod.widgets.LiyuBtnPrimary { width: Fit text: "保存" }
    }

    // ---------------------------------------------------------------
    // 详情页头部（窄屏 / 宽屏共用）：「‹ 返回」+ 标题。
    // 宽屏分栏里返回钮常显隐（visible: false），窄屏前进后由集成方露出。
    // ---------------------------------------------------------------
    mod.widgets.AccountDetailHead = mod.widgets.View{
        width: Fill height: Fit
        flow: Right
        align: Align{x: 0.0, y: 0.5}
        spacing: 8.0
        acc_back := mod.widgets.LiyuBtnSm { visible: false width: Fit text: "‹ 返回" }
        acc_title := mod.widgets.LiyuH2 { width: Fill text: "" }
    }

    // ---------------------------------------------------------------
    // 宽屏壳：左 232px 分类列表 + 右详情区。
    //
    // 七个分类行 acc_cat0..acc_cat6 与 ACCOUNT_CATEGORIES 一一对应，
    // 标题由 Rust 侧数据驱动填入；acc_detail_body 是占位容器，当前分类
    // 的内容由 master 按分类往里放（资料行、表单、列表……）。
    // ---------------------------------------------------------------
    mod.widgets.AccountSplitView = mod.widgets.View{
        width: Fill height: Fill
        flow: Right
        spacing: 16.0
        acc_side := mod.widgets.View {
            width: 232 height: Fill
            flow: Down
            spacing: 10.0
            acc_head := mod.widgets.LiyuH2 { text: "账号与设置" }
            acc_cats := mod.widgets.LiyuCard {
                width: Fill height: Fit
                flow: Down
                padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 6.0}
                spacing: 2.0
                acc_cat0 := mod.widgets.AccountCategoryRow { text: "" }
                acc_cat1 := mod.widgets.AccountCategoryRow { text: "" }
                acc_cat2 := mod.widgets.AccountCategoryRow { text: "" }
                acc_cat3 := mod.widgets.AccountCategoryRow { text: "" }
                acc_cat4 := mod.widgets.AccountCategoryRow { text: "" }
                acc_cat5 := mod.widgets.AccountCategoryRow { text: "" }
                acc_cat6 := mod.widgets.AccountCategoryRow { text: "" }
            }
        }
        acc_main := mod.widgets.LiyuScrollY {
            width: Fill height: Fill
            flow: Down
            spacing: 12.0
            acc_detail_head := mod.widgets.AccountDetailHead { }
            acc_detail_body := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 12.0
                acc_detail_placeholder := mod.widgets.LiyuMuted { text: "选择左侧分类查看与修改" }
            }
        }
    }

    // ---------------------------------------------------------------
    // 窄屏一级：分类列表（整屏）。行沿用 AccountCategoryRow，
    // id 与宽屏错开（acc_mcat*），两边可同时挂在树里按形态切 visible。
    // ---------------------------------------------------------------
    mod.widgets.AccountListView = mod.widgets.LiyuScrollY{
        width: Fill height: Fill
        flow: Down
        spacing: 12.0
        acc_list_head := mod.widgets.LiyuH2 { text: "账号与设置" }
        acc_list := mod.widgets.LiyuCard {
            width: Fill height: Fit
            flow: Down
            padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 6.0}
            spacing: 2.0
            acc_mcat0 := mod.widgets.AccountCategoryRow { text: "" }
            acc_mcat1 := mod.widgets.AccountCategoryRow { text: "" }
            acc_mcat2 := mod.widgets.AccountCategoryRow { text: "" }
            acc_mcat3 := mod.widgets.AccountCategoryRow { text: "" }
            acc_mcat4 := mod.widgets.AccountCategoryRow { text: "" }
            acc_mcat5 := mod.widgets.AccountCategoryRow { text: "" }
            acc_mcat6 := mod.widgets.AccountCategoryRow { text: "" }
        }
    }

    // ---------------------------------------------------------------
    // 窄屏二级：分类详情（「‹ 返回」+ 标题 + 占位容器）。
    // ---------------------------------------------------------------
    mod.widgets.AccountDetailView = mod.widgets.LiyuScrollY{
        width: Fill height: Fill
        flow: Down
        spacing: 12.0
        acc_mdetail_head := mod.widgets.AccountDetailHead { }
        acc_mdetail_body := mod.widgets.View {
            width: Fill height: Fit
            flow: Down
            spacing: 12.0
            acc_mdetail_placeholder := mod.widgets.LiyuMuted { text: "" }
        }
    }

    // ---------------------------------------------------------------
    // 窄屏三级：字段编辑页。「‹ 返回」+ 字段名标题 + 说明 + 输入框 +
    // 底部统一操作区（acc_edit_cancel / acc_edit_save）。
    // ---------------------------------------------------------------
    mod.widgets.AccountEditView = mod.widgets.LiyuScrollY{
        width: Fill height: Fill
        flow: Down
        spacing: 14.0
        acc_edit_head := mod.widgets.AccountDetailHead { }
        acc_edit_card := mod.widgets.LiyuCard {
            width: Fill height: Fit
            flow: Down
            padding: 14.0
            spacing: 10.0
            acc_edit_note := mod.widgets.LiyuMuted { text: "" }
            acc_edit_input := mod.widgets.LiyuInput { empty_text: "" }
        }
        acc_edit_actions := mod.widgets.AccountEditActions { }
    }
}

// ---------------------------------------------------------------
// 分类数据：列表数据驱动（DSL 只负责排布，标题 / 顺序以这里为准）。
// ---------------------------------------------------------------

/// 「我 / 设置」的二级分类。顺序即列表顺序，与 DSL 里的
/// `acc_cat0..acc_cat6` / `acc_mcat0..acc_mcat6` 一一对应。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountCategory {
    /// 显示名、头像、签名。
    Profile,
    /// 手机号 / 邮箱绑定。
    Contact,
    /// 收货地址管理。
    Address,
    /// 通用（深浅模式、语言）。
    General,
    /// 通知开关。
    Notifications,
    /// 数据管理（导出 / 恢复演示数据）。
    Data,
    /// 关于（版本 / 协议）。
    About,
}

impl AccountCategory {
    /// 全部七个分类，按界面顺序。
    pub const ALL: [AccountCategory; 7] = [
        AccountCategory::Profile,
        AccountCategory::Contact,
        AccountCategory::Address,
        AccountCategory::General,
        AccountCategory::Notifications,
        AccountCategory::Data,
        AccountCategory::About,
    ];

    /// 分类行上的标题。
    pub fn title(self) -> &'static str {
        match self {
            AccountCategory::Profile => "个人资料",
            AccountCategory::Contact => "账号与联系方式",
            AccountCategory::Address => "收货地址",
            AccountCategory::General => "通用",
            AccountCategory::Notifications => "通知",
            AccountCategory::Data => "数据管理",
            AccountCategory::About => "关于",
        }
    }

    /// 详情区默认的副标题（占位容器第一行说明）。
    pub fn subtitle(self) -> &'static str {
        match self {
            AccountCategory::Profile => "显示名、头像与签名",
            AccountCategory::Contact => "手机号与邮箱绑定",
            AccountCategory::Address => "管理收货地址",
            AccountCategory::General => "深浅模式与语言",
            AccountCategory::Notifications => "消息通知开关",
            AccountCategory::Data => "导出与恢复本机数据",
            AccountCategory::About => "版本与协议",
        }
    }
}

/// 七个分类（界面顺序）。集成方遍历来填 `acc_cat0..` / `acc_mcat0..`。
pub const ACCOUNT_CATEGORIES: [AccountCategory; 7] = AccountCategory::ALL;

/// 宽屏分类行的 DSL id（与 `acc_cat0..acc_cat6` 对应）。
pub const SPLIT_CATEGORY_IDS: [LiveId; 7] = [
    live_id!(acc_cat0),
    live_id!(acc_cat1),
    live_id!(acc_cat2),
    live_id!(acc_cat3),
    live_id!(acc_cat4),
    live_id!(acc_cat5),
    live_id!(acc_cat6),
];

/// 窄屏分类行的 DSL id（与 `acc_mcat0..acc_mcat6` 对应）。
pub const LIST_CATEGORY_IDS: [LiveId; 7] = [
    live_id!(acc_mcat0),
    live_id!(acc_mcat1),
    live_id!(acc_mcat2),
    live_id!(acc_mcat3),
    live_id!(acc_mcat4),
    live_id!(acc_mcat5),
    live_id!(acc_mcat6),
];

/// 编辑页「取消」按钮的统一 id。
pub const ACCOUNT_EDIT_CANCEL_ID: LiveId = live_id!(acc_edit_cancel);
/// 编辑页「保存 / 确认绑定」按钮的统一 id。
pub const ACCOUNT_EDIT_SAVE_ID: LiveId = live_id!(acc_edit_save);

/// 资料行按字段类型该露哪个右侧操作（集成方按此切 visible / 文案）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountRowAction {
    /// 文本字段：「修改」。
    Edit,
    /// 管理类入口：「管理」+ 箭头。
    Manage,
    /// 头像 / 图片：「选择图片…」（用 AccountAvatarRow，不用普通资料行）。
    PickImage,
}

impl AccountRowAction {
    /// 操作钮上的文案。
    pub fn label(self) -> &'static str {
        match self {
            AccountRowAction::Edit => "修改",
            AccountRowAction::Manage => "管理",
            AccountRowAction::PickImage => "选择图片…",
        }
    }
}

/// 宽屏阈值与主壳一致（< PHONE_MAX 走窄屏三件套，否则走分栏）。
/// 与 lib.rs 的 PHONE_MAX 同值；集成时以主壳的 shaping 为准，这里只是
/// 给独立使用本模块的调用方一个同口径的参考。
pub const ACCOUNT_PHONE_MAX: f64 = 720.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn count_occurrences(hay: &str, needle: &str) -> usize {
        hay.matches(needle).count()
    }

    fn module_source() -> String {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("src/account_ui.rs");
        std::fs::read_to_string(path).expect("account_ui.rs 应可读")
    }

    // ---- 纯数据断言（不依赖 VM，编译期就能跑）----

    #[test]
    fn categories_are_seven_and_in_order() {
        assert_eq!(ACCOUNT_CATEGORIES.len(), 7);
        let titles: Vec<&str> = ACCOUNT_CATEGORIES.iter().map(|c| c.title()).collect();
        assert_eq!(
            titles,
            vec![
                "个人资料",
                "账号与联系方式",
                "收货地址",
                "通用",
                "通知",
                "数据管理",
                "关于"
            ]
        );
        for c in ACCOUNT_CATEGORIES {
            assert!(!c.subtitle().is_empty(), "{:?} 缺副标题", c);
        }
    }

    #[test]
    fn category_ids_match_category_count_and_are_distinct() {
        assert_eq!(SPLIT_CATEGORY_IDS.len(), ACCOUNT_CATEGORIES.len());
        assert_eq!(LIST_CATEGORY_IDS.len(), ACCOUNT_CATEGORIES.len());
        for i in 0..SPLIT_CATEGORY_IDS.len() {
            assert_ne!(
                SPLIT_CATEGORY_IDS[i], LIST_CATEGORY_IDS[i],
                "宽窄两壳的分类行 id 必须错开（两边同时挂树）"
            );
        }
        for w in SPLIT_CATEGORY_IDS.windows(2) {
            assert_ne!(w[0], w[1]);
        }
        for w in LIST_CATEGORY_IDS.windows(2) {
            assert_ne!(w[0], w[1]);
        }
    }

    #[test]
    fn edit_action_ids_are_stable_and_distinct() {
        assert_eq!(ACCOUNT_EDIT_CANCEL_ID, live_id!(acc_edit_cancel));
        assert_eq!(ACCOUNT_EDIT_SAVE_ID, live_id!(acc_edit_save));
        assert_ne!(ACCOUNT_EDIT_CANCEL_ID, ACCOUNT_EDIT_SAVE_ID);
    }

    #[test]
    fn row_action_labels_cover_the_three_field_kinds() {
        assert_eq!(AccountRowAction::Edit.label(), "修改");
        assert_eq!(AccountRowAction::Manage.label(), "管理");
        assert_eq!(AccountRowAction::PickImage.label(), "选择图片…");
    }

    // ---- 布局结构断言（DSL 文本层面：预设存在、行数对、id 对、色板引用对）----

    #[test]
    fn dsl_defines_all_five_shells_and_primitives() {
        let src = module_source();
        for preset in [
            "AccountSplitView",
            "AccountListView",
            "AccountDetailView",
            "AccountEditView",
            "AccountInfoRow",
            "AccountAvatarRow",
            "AccountEditActions",
            "AccountDetailHead",
            "AccountCategoryRow",
        ] {
            let def = format!("mod.widgets.{preset} = mod.widgets.");
            assert!(
                src.contains(&def),
                "缺预设定义 {def}（或没从既有 widgets 派生）"
            );
        }
    }

    #[test]
    fn dsl_has_seven_category_rows_per_shell() {
        let src = module_source();
        for prefix in ["acc_cat", "acc_mcat"] {
            for i in 0..7 {
                let id = format!("{prefix}{i} := mod.widgets.AccountCategoryRow");
                assert!(src.contains(&id), "缺分类行 {id}");
            }
            let instances = (0..7)
                .filter(|i| src.contains(&format!("{prefix}{i} := mod.widgets.AccountCategoryRow")))
                .count();
            assert_eq!(instances, 7, "{prefix}* 应恰好七行");
        }
    }

    #[test]
    fn dsl_edit_actions_use_the_unified_ids() {
        let src = module_source();
        assert!(
            src.contains("acc_edit_cancel := mod.widgets.LiyuBtn { width: Fit text: \"取消\" }")
        );
        assert!(src
            .contains("acc_edit_save := mod.widgets.LiyuBtnPrimary { width: Fit text: \"保存\" }"));
        // 操作区在编辑页里挂着。
        assert!(src.contains("acc_edit_actions := mod.widgets.AccountEditActions"));
    }

    #[test]
    fn dsl_reuses_theme_roles_and_existing_widgets_only() {
        let src = module_source();
        // 主题色角色引用（不许硬编码新色值；#0000 透明是全应用既有的例外）。
        for role in [
            "liyu.ink",
            "liyu.ink_2",
            "liyu.ink_3",
            "liyu.hl",
            "liyu.ink_arrow",
        ] {
            assert!(src.contains(role), "应引用主题角色 {role}");
        }
        for radius in ["r.button"] {
            assert!(src.contains(radius), "圆角应取 {radius}");
        }
        // 十六进制字面量只允许 #0000（透明）。跳过测试规则自身的行。
        for line in src.lines() {
            if line.contains("code.find(") || line.contains("line.find(") {
                continue;
            }
            let code = line.split("//").next().unwrap_or("");
            if let Some(at) = code.find("#x") {
                panic!("硬编码色值出现在: {}", &code[at..]);
            }
        }
        // 既有组件复用：不重复造卡 / 输入 / 钮 / 缩略图。
        for reused in [
            "mod.widgets.LiyuCard",
            "mod.widgets.LiyuScrollY",
            "mod.widgets.LiyuH2",
            "mod.widgets.LiyuMuted",
            "mod.widgets.LiyuInput",
            "mod.widgets.LiyuThumb",
            "mod.widgets.LiyuBtn",
            "mod.widgets.LiyuBtnSm",
            "mod.widgets.LiyuBtnPrimary",
            "mod.widgets.LiyuIcon",
        ] {
            assert!(src.contains(reused), "应复用 {reused}");
        }
    }

    #[test]
    fn avatar_row_reuses_the_existing_pick_crop_upload_controls() {
        let src = module_source();
        // 与「个人资料」页 pf_avatar_* 同款控件组合：预览图 + 选择图片… +
        // 裁切位置 / 旋转 90° / 确认上传 / 取消。
        for (id, kind) in [
            ("aar_thumb", "mod.widgets.LiyuThumb"),
            ("aar_pick", "mod.widgets.LiyuBtn"),
            ("aar_anchor", "mod.widgets.LiyuBtnSm"),
            ("aar_rotate", "mod.widgets.LiyuBtnSm"),
            ("aar_confirm", "mod.widgets.LiyuBtnPrimary"),
            ("aar_cancel", "mod.widgets.LiyuBtnSm"),
        ] {
            let needle = format!("{id} := {kind}");
            assert!(src.contains(&needle), "头像行缺控件 {needle}");
        }
        for text in ["选择图片…", "裁切位置：居中", "旋转 90°", "确认上传"] {
            assert!(src.contains(text), "头像行缺文案 {text}");
        }
    }

    #[test]
    fn detail_areas_are_placeholder_containers_with_stable_ids() {
        let src = module_source();
        for id in ["acc_detail_body", "acc_mdetail_body"] {
            assert!(
                src.contains(&format!("{id} := mod.widgets.View")),
                "详情占位容器 {id} 缺失"
            );
        }
        // 分屏前进 / 返回的钩子：详情头里带返回钮（窄屏露出）。
        assert!(src.contains("acc_back := mod.widgets.LiyuBtnSm"));
    }

    // ---- VM 级断言：预设真能在两套主题下干净求值、能从 widgets 命名空间取到 ----

    #[test]
    fn presets_evaluate_cleanly_in_both_themes() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            for mode in crate::theme::ThemeMode::ALL {
                crate::theme::set_mode(mode);
                vm.bx.captured_errors = Some(Vec::new());
                vm.with_reload(|vm| {
                    crate::theme::install(vm);
                    crate::canvas::script_mod(vm);
                    script_mod(vm);
                });
                let errors = vm.take_errors();
                assert!(errors.is_empty(), "{mode:?} 下账号预设求值出错: {errors:?}");
                for (preset, got) in [
                    (
                        "AccountSplitView",
                        script_eval!(vm, { mod.widgets.AccountSplitView }).as_object(),
                    ),
                    (
                        "AccountListView",
                        script_eval!(vm, { mod.widgets.AccountListView }).as_object(),
                    ),
                    (
                        "AccountDetailView",
                        script_eval!(vm, { mod.widgets.AccountDetailView }).as_object(),
                    ),
                    (
                        "AccountEditView",
                        script_eval!(vm, { mod.widgets.AccountEditView }).as_object(),
                    ),
                    (
                        "AccountInfoRow",
                        script_eval!(vm, { mod.widgets.AccountInfoRow }).as_object(),
                    ),
                    (
                        "AccountAvatarRow",
                        script_eval!(vm, { mod.widgets.AccountAvatarRow }).as_object(),
                    ),
                    (
                        "AccountEditActions",
                        script_eval!(vm, { mod.widgets.AccountEditActions }).as_object(),
                    ),
                ] {
                    assert!(got.is_some(), "{mode:?} 下 mod.widgets.{preset} 没注册");
                }
            }
            crate::theme::set_mode(crate::theme::ThemeMode::Dark);
        });
    }
}
