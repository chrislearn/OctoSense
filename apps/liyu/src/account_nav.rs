//! # 账号导航(AccountNavModel)
//!
//! 「我 / 设置」二三级页面的纯状态机模块,与 UI 完全解耦(无 Makepad 依赖)。
//! 管理账号分类的选择、L1/L2/L3 层级导航、编辑草稿(快照/还原/未保存标记)、
//! 以及 desktop/mobile 布局模式切换时的状态保留。
//!
//! 集成约定(供 master 接入 lib.rs):
//! - UI 事件 → 模型方法;模型查询 → UI 渲染。模型不持有任何 UI 句柄。
//! - `AccountCategory` 的 7 个变体对应既有能力页面(个人资料 / 账号与联系方式 /
//!   收货地址 / 通用(外观) / 通知 / 数据管理 / 关于)。
//! - 布局由宿主按宽度阈值调用 `set_layout_for_width` 或 `set_layout` 驱动;
//!   模型保证跨模式切换不丢选中项、层级与草稿。

use std::collections::BTreeMap;

/// 账号页面一级分类(7 项,对应既有能力)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountCategory {
    /// 个人资料(头像、昵称、签名)
    Profile,
    /// 账号与联系方式(手机、邮箱、密码)
    AccountContact,
    /// 收货地址
    ShippingAddress,
    /// 通用(外观、语言等)
    General,
    /// 通知
    Notifications,
    /// 数据管理(缓存、导出)
    DataManagement,
    /// 关于
    About,
}

impl AccountCategory {
    /// 全部 7 个分类,按展示顺序。
    pub const ALL: [AccountCategory; 7] = [
        AccountCategory::Profile,
        AccountCategory::AccountContact,
        AccountCategory::ShippingAddress,
        AccountCategory::General,
        AccountCategory::Notifications,
        AccountCategory::DataManagement,
        AccountCategory::About,
    ];

    /// 展示用名称(中文)。
    pub fn label(self) -> &'static str {
        match self {
            AccountCategory::Profile => "个人资料",
            AccountCategory::AccountContact => "账号与联系方式",
            AccountCategory::ShippingAddress => "收货地址",
            AccountCategory::General => "通用",
            AccountCategory::Notifications => "通知",
            AccountCategory::DataManagement => "数据管理",
            AccountCategory::About => "关于",
        }
    }

    /// 是否支持字段编辑(「关于」等纯展示分类不支持)。
    pub fn supports_edit(self) -> bool {
        !matches!(self, AccountCategory::About)
    }
}

/// 当前导航层级。
///
/// - `L1`:分类列表;
/// - `L2`:某个分类的详情页(记录分类);
/// - `L3`:某个分类下某个字段的编辑页(记录分类与字段)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavLevel {
    L1,
    L2(AccountCategory),
    L3(AccountCategory, String),
}

impl NavLevel {
    /// 层级深度(1/2/3)。
    pub fn depth(&self) -> u8 {
        match self {
            NavLevel::L1 => 1,
            NavLevel::L2(_) => 2,
            NavLevel::L3(_, _) => 3,
        }
    }
}

/// 布局模式:宽屏(左分类 + 右详情)或窄屏(列表/详情分屏)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// 宽屏:左右分栏,L1/L2 同屏。
    Desktop,
    /// 窄屏:列表与详情分屏独占。
    Mobile,
}

/// desktop/mobile 判定阈值(单位:px,宽度 ≥ 阈值即 desktop)。
pub const DESKTOP_WIDTH_THRESHOLD: f64 = 768.0;

/// 单个字段的编辑草稿:进入编辑时的快照 + 当前编辑值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDraft {
    /// 字段标识(由宿主定义,如 "nickname"、"phone")。
    pub field: String,
    /// 进入编辑时保存的已提交值快照,取消编辑时还原。
    pub snapshot: Option<String>,
    /// 当前编辑中的值。
    pub current: String,
}

impl FieldDraft {
    /// 当前值与快照相比是否有改动。
    pub fn is_dirty(&self) -> bool {
        self.snapshot.as_deref() != Some(self.current.as_str())
    }
}

/// 编辑会话:分类 + 字段 + 草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftState {
    /// 草稿所属分类。
    pub category: AccountCategory,
    /// 字段草稿。
    pub draft: FieldDraft,
}

/// 账号导航状态机。
#[derive(Debug, Clone)]
pub struct AccountNavModel {
    /// 当前层级(含选中项信息)。
    level: NavLevel,
    /// 当前选中的分类(L1 上的高亮项;进入 L2/L3 后与层级内分类一致)。
    selected: Option<AccountCategory>,
    /// 布局模式。
    layout: LayoutMode,
    /// 进行中的编辑草稿(L3 才有)。
    edit: Option<DraftState>,
    /// 各分类各字段已提交(保存)的值。仅作为模型侧的可信源,
    /// 真实持久化由宿主完成;commit 时模型同步更新,便于取消还原。
    committed: BTreeMap<(AccountCategory, String), String>,
}

impl Default for AccountNavModel {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountNavModel {
    /// 新建模型:L1、无选中、Desktop 布局、无草稿。
    pub fn new() -> Self {
        Self {
            level: NavLevel::L1,
            selected: None,
            layout: LayoutMode::Desktop,
            edit: None,
            committed: BTreeMap::new(),
        }
    }

    // ---------- 查询 ----------

    /// 当前层级。
    pub fn level(&self) -> &NavLevel {
        &self.level
    }

    /// 当前层级深度(1/2/3)。
    pub fn depth(&self) -> u8 {
        self.level.depth()
    }

    /// 当前选中的分类。
    pub fn selected(&self) -> Option<AccountCategory> {
        self.selected
    }

    /// 当前布局模式。
    pub fn layout(&self) -> LayoutMode {
        self.layout
    }

    /// 进行中的草稿(L3 编辑时才有)。
    pub fn edit(&self) -> Option<&DraftState> {
        self.edit.as_ref()
    }

    /// 是否有未保存改动(草稿当前值与快照不同)。
    pub fn has_unsaved_changes(&self) -> bool {
        self.edit.as_ref().is_some_and(|e| e.draft.is_dirty())
    }

    /// 某分类某字段已提交的值(若宿主曾同步过)。
    pub fn committed_value(&self, cat: AccountCategory, field: &str) -> Option<&str> {
        self.committed
            .get(&(cat, field.to_string()))
            .map(String::as_str)
    }

    /// 宿主同步已提交值(如打开页面时从数据源灌入),供后续取消编辑时还原。
    pub fn set_committed(
        &mut self,
        cat: AccountCategory,
        field: impl Into<String>,
        value: impl Into<String>,
    ) {
        self.committed.insert((cat, field.into()), value.into());
    }

    // ---------- 导航 ----------

    /// 选中分类:从 L1(或任意层)进入该分类的 L2 详情。
    ///
    /// 若存在未保存改动,不会静默丢弃:返回 `false` 且不切换;
    /// 调用方应先提示,或显式调用 `discard_edit()` 后再选。
    pub fn select(&mut self, cat: AccountCategory) -> bool {
        if self.has_unsaved_changes() {
            return false;
        }
        self.edit = None;
        self.selected = Some(cat);
        self.level = NavLevel::L2(cat);
        true
    }

    /// 仅高亮选中项(不进入详情),用于 desktop 左栏 hover/单选场景。
    pub fn highlight(&mut self, cat: AccountCategory) {
        self.selected = Some(cat);
    }

    /// 逐级返回:L3→L2→L1;L1 再返回返回 `false`(不 panic、不越级)。
    ///
    /// L3 有未保存改动时返回 `false` 并停留原层级,由调用方决定
    /// 「提示放弃 / 显式 `discard_edit()` 后再返回」。
    pub fn pop(&mut self) -> bool {
        match self.level.clone() {
            NavLevel::L1 => false,
            NavLevel::L2(_) => {
                self.level = NavLevel::L1;
                true
            }
            NavLevel::L3(cat, _) => {
                if self.has_unsaved_changes() {
                    return false;
                }
                self.edit = None;
                self.level = NavLevel::L2(cat);
                true
            }
        }
    }

    /// 强制返回(丢弃未保存改动)。返回是否发生了层级变化。
    pub fn pop_discarding(&mut self) -> bool {
        match self.level.clone() {
            NavLevel::L1 => false,
            NavLevel::L2(_) => {
                self.level = NavLevel::L1;
                true
            }
            NavLevel::L3(cat, _) => {
                self.edit = None;
                self.level = NavLevel::L2(cat);
                true
            }
        }
    }

    // ---------- 编辑 ----------

    /// 进入某字段的编辑(L3)。
    ///
    /// 快照取当前已提交值(若宿主同步过);否则快照为 `None`(视为新增字段)。
    /// `initial` 为编辑框初始值,通常与已提交值一致。
    ///
    /// 当前已在编辑且有未保存改动时拒绝进入(返回 `false`),避免草稿被静默覆盖。
    pub fn begin_edit(&mut self, field: impl Into<String>, initial: impl Into<String>) -> bool {
        let Some(cat) = self.selected else {
            return false; // 未选分类不能编辑
        };
        if !cat.supports_edit() {
            return false;
        }
        if self.has_unsaved_changes() {
            return false;
        }
        let field = field.into();
        let initial = initial.into();
        let snapshot = self
            .committed
            .get(&(cat, field.clone()))
            .cloned()
            .or_else(|| Some(initial.clone()));
        self.edit = Some(DraftState {
            category: cat,
            draft: FieldDraft {
                field: field.clone(),
                snapshot,
                current: initial,
            },
        });
        self.level = NavLevel::L3(cat, field);
        true
    }

    /// 更新编辑中的草稿值。不在编辑状态时返回 `false`。
    pub fn update_draft(&mut self, value: impl Into<String>) -> bool {
        match self.edit.as_mut() {
            Some(e) => {
                e.draft.current = value.into();
                true
            }
            None => false,
        }
    }

    /// 取消编辑:还原草稿快照,层级回到 L2。
    ///
    /// 还原结果通过 `last_restored()` 语义由调用方读取 `committed_value`
    /// 即可(快照即已提交值,未被改动)。不在编辑状态返回 `false`。
    pub fn cancel_edit(&mut self) -> bool {
        let Some(e) = self.edit.take() else {
            return false;
        };
        self.level = NavLevel::L2(e.category);
        true
    }

    /// 丢弃当前草稿(不移动层级),供「有未保存改动仍要离开」确认后调用。
    pub fn discard_edit(&mut self) {
        if let Some(e) = self.edit.take() {
            if let NavLevel::L3(cat, _) = self.level.clone() {
                debug_assert_eq!(cat, e.category);
                self.level = NavLevel::L2(cat);
            }
        }
    }

    /// 保存编辑:把草稿写入已提交值,层级回到 L2。
    /// 返回保存的 `(category, field, value)`;不在编辑状态返回 `None`。
    pub fn commit_edit(&mut self) -> Option<(AccountCategory, String, String)> {
        let e = self.edit.take()?;
        self.committed
            .insert((e.category, e.draft.field.clone()), e.draft.current.clone());
        self.level = NavLevel::L2(e.category);
        Some((e.category, e.draft.field, e.draft.current))
    }

    // ---------- 布局 ----------

    /// 设置布局模式。切换保持选中项、层级与草稿,不丢任何状态。
    pub fn set_layout(&mut self, layout: LayoutMode) {
        self.layout = layout;
    }

    /// 按宽度自动判定布局:`width >= 768px` 为 Desktop,否则 Mobile。
    /// 返回判定后的模式(便于宿主记录)。
    pub fn set_layout_for_width(&mut self, width: f64) -> LayoutMode {
        let mode = if width >= DESKTOP_WIDTH_THRESHOLD {
            LayoutMode::Desktop
        } else {
            LayoutMode::Mobile
        };
        self.layout = mode;
        mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_in_l2(cat: AccountCategory) -> AccountNavModel {
        let mut m = AccountNavModel::new();
        assert!(m.select(cat));
        m
    }

    // ---- 分类 ----

    #[test]
    fn seven_categories_have_unique_labels() {
        assert_eq!(AccountCategory::ALL.len(), 7);
        for (i, a) in AccountCategory::ALL.iter().enumerate() {
            for b in &AccountCategory::ALL[i + 1..] {
                assert_ne!(a, b);
                assert_ne!(a.label(), b.label());
            }
        }
    }

    #[test]
    fn about_does_not_support_edit() {
        assert!(!AccountCategory::About.supports_edit());
        assert!(AccountCategory::Profile.supports_edit());
    }

    // ---- 选中与层级前进 ----

    #[test]
    fn select_enters_l2_and_records_selection() {
        let mut m = AccountNavModel::new();
        assert_eq!(m.level(), &NavLevel::L1);
        assert_eq!(m.selected(), None);
        assert!(m.select(AccountCategory::Profile));
        assert_eq!(m.level(), &NavLevel::L2(AccountCategory::Profile));
        assert_eq!(m.selected(), Some(AccountCategory::Profile));
        assert_eq!(m.depth(), 2);
    }

    #[test]
    fn switch_category_from_l2() {
        let mut m = model_in_l2(AccountCategory::Profile);
        assert!(m.select(AccountCategory::Notifications));
        assert_eq!(m.level(), &NavLevel::L2(AccountCategory::Notifications));
    }

    // ---- 返回语义 ----

    #[test]
    fn pop_walks_back_level_by_level() {
        let mut m = model_in_l2(AccountCategory::Profile);
        assert!(m.begin_edit("nickname", "小牛"));
        assert_eq!(m.depth(), 3);
        assert!(m.pop()); // L3 → L2
        assert_eq!(m.level(), &NavLevel::L2(AccountCategory::Profile));
        assert!(m.pop()); // L2 → L1
        assert_eq!(m.level(), &NavLevel::L1);
    }

    #[test]
    fn pop_at_l1_returns_false_without_panic() {
        let mut m = AccountNavModel::new();
        assert!(!m.pop());
        assert!(!m.pop()); // 重复调用仍安全
        assert_eq!(m.level(), &NavLevel::L1);
    }

    #[test]
    fn pop_never_jumps_to_unrelated_page() {
        let mut m = model_in_l2(AccountCategory::ShippingAddress);
        assert!(m.pop());
        // 回到的是 L1 分类列表,选中项保留,不是无关首页
        assert_eq!(m.level(), &NavLevel::L1);
        assert_eq!(m.selected(), Some(AccountCategory::ShippingAddress));
    }

    // ---- 编辑草稿 ----

    #[test]
    fn begin_edit_requires_selection() {
        let mut m = AccountNavModel::new();
        assert!(!m.begin_edit("nickname", "x"));
    }

    #[test]
    fn begin_edit_blocked_for_about() {
        let mut m = model_in_l2(AccountCategory::About);
        assert!(!m.begin_edit("version", "1.0"));
        assert_eq!(m.level(), &NavLevel::L2(AccountCategory::About));
    }

    #[test]
    fn edit_flow_dirty_commit_and_unsaved_flag() {
        let mut m = model_in_l2(AccountCategory::Profile);
        assert!(!m.has_unsaved_changes());
        assert!(m.begin_edit("nickname", "小牛"));
        assert!(!m.has_unsaved_changes()); // 初始值 == 快照
        assert!(m.update_draft("大牛"));
        assert!(m.has_unsaved_changes());
        let saved = m.commit_edit().expect("should save");
        assert_eq!(
            saved,
            (AccountCategory::Profile, "nickname".into(), "大牛".into())
        );
        assert!(!m.has_unsaved_changes());
        assert_eq!(m.level(), &NavLevel::L2(AccountCategory::Profile));
        assert_eq!(
            m.committed_value(AccountCategory::Profile, "nickname"),
            Some("大牛")
        );
    }

    #[test]
    fn cancel_edit_restores_snapshot_and_leaves_l3() {
        let mut m = model_in_l2(AccountCategory::AccountContact);
        m.set_committed(AccountCategory::AccountContact, "phone", "13800000000");
        assert!(m.begin_edit("phone", "13800000000"));
        assert!(m.update_draft("13911111111"));
        assert!(m.has_unsaved_changes());
        assert!(m.cancel_edit());
        assert!(!m.has_unsaved_changes());
        assert_eq!(m.level(), &NavLevel::L2(AccountCategory::AccountContact));
        // 快照未被污染
        assert_eq!(
            m.committed_value(AccountCategory::AccountContact, "phone"),
            Some("13800000000")
        );
    }

    #[test]
    fn unsaved_changes_block_select_and_pop() {
        let mut m = model_in_l2(AccountCategory::Profile);
        assert!(m.begin_edit("nickname", "小牛"));
        assert!(m.update_draft("改了"));
        assert!(!m.select(AccountCategory::General)); // 被拒绝
        assert!(!m.pop()); // 被拒绝
        assert_eq!(m.depth(), 3); // 仍在 L3
                                  // 显式丢弃后可以离开
        m.discard_edit();
        assert_eq!(m.level(), &NavLevel::L2(AccountCategory::Profile));
        assert!(m.select(AccountCategory::General));
    }

    #[test]
    fn pop_discarding_forces_back_with_unsaved() {
        let mut m = model_in_l2(AccountCategory::General);
        assert!(m.begin_edit("theme", "light"));
        assert!(m.update_draft("dark"));
        assert!(m.pop_discarding());
        assert_eq!(m.level(), &NavLevel::L2(AccountCategory::General));
        assert!(!m.has_unsaved_changes());
    }

    #[test]
    fn begin_edit_blocked_while_another_dirty_edit_open() {
        let mut m = model_in_l2(AccountCategory::Profile);
        assert!(m.begin_edit("nickname", "a"));
        assert!(m.update_draft("b"));
        assert!(!m.begin_edit("bio", "x"));
        assert_eq!(m.edit().unwrap().draft.field, "nickname");
    }

    #[test]
    fn no_edit_operations_on_idle_model() {
        let mut m = AccountNavModel::new();
        assert!(!m.update_draft("x"));
        assert!(!m.cancel_edit());
        assert!(m.commit_edit().is_none());
    }

    // ---- 布局切换 ----

    #[test]
    fn layout_switch_preserves_selection_level_and_draft() {
        let mut m = model_in_l2(AccountCategory::Profile);
        assert!(m.begin_edit("nickname", "小牛"));
        assert!(m.update_draft("大牛"));

        m.set_layout(LayoutMode::Mobile);
        assert_eq!(m.layout(), LayoutMode::Mobile);
        assert_eq!(m.depth(), 3);
        assert_eq!(m.selected(), Some(AccountCategory::Profile));
        assert!(m.has_unsaved_changes());
        assert_eq!(m.edit().unwrap().draft.current, "大牛");

        m.set_layout(LayoutMode::Desktop);
        assert_eq!(m.layout(), LayoutMode::Desktop);
        assert_eq!(m.depth(), 3);
        assert!(m.has_unsaved_changes());
    }

    #[test]
    fn width_threshold_decides_mode() {
        let mut m = AccountNavModel::new();
        assert_eq!(m.set_layout_for_width(1024.0), LayoutMode::Desktop);
        assert_eq!(m.set_layout_for_width(768.0), LayoutMode::Desktop);
        assert_eq!(m.set_layout_for_width(767.9), LayoutMode::Mobile);
        assert_eq!(m.set_layout_for_width(375.0), LayoutMode::Mobile);
    }

    #[test]
    fn cross_mode_navigation_continues_correctly() {
        let mut m = AccountNavModel::new();
        m.set_layout_for_width(375.0); // mobile 进入
        assert!(m.select(AccountCategory::DataManagement));
        m.set_layout_for_width(1280.0); // 切到 desktop
        assert!(m.begin_edit("cache_limit", "500MB"));
        assert!(m.update_draft("1GB"));
        assert!(m.commit_edit().is_some());
        m.set_layout_for_width(375.0); // 再切回 mobile
        assert_eq!(m.level(), &NavLevel::L2(AccountCategory::DataManagement));
        assert!(m.pop());
        assert_eq!(m.level(), &NavLevel::L1);
    }
}
