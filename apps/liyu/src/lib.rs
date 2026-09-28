//! 礼遇 LiYu —— OctoSense 桌面里的演示社交应用：匿名悬念送礼。
//!
//! 核心循环：挑礼物 → 设解密游戏与契约 → 生成神秘礼卡匿名发出 → 收礼人解谜
//! → 开心收下（履约）或换购 / 折现（变现）→ 用余额回一份「反击礼物」。
//! 规则与数据模型见 data.rs，页面规格见 liyu/docs/03-pages.md。
pub use makepad_widgets;
use makepad_widgets::*;
use makepad_widgets::makepad_draw::turtle::RowAlign;
use makepad_widgets::makepad_draw::image_cache::ImageBuffer;
use makepad_app_module::{
    AppModule, ExecOutcome, InstanceHandles, InstanceParts, OpenSchema,
    ServiceExecutor, ValidatedOpen,
    makepad_ai_services::wire::{ServiceCall, ServiceManifest, ToolOutcome, ToolResult},
};

pub mod contacts;
pub mod ai;
mod avatar;
pub mod ai_draft;
pub mod canvas;
pub mod data;
mod commerce_client;
mod gift_client;
mod profile_client;
pub mod share;
pub mod theme;
mod wishui;
mod account_nav;
mod account_ui;

use canvas::LiyuShareCard;
use data::*;
use makepad_widgets::makepad_platform::file_dialogs::{FileDialog, FileDialogAction};
use commerce_client::{Commerce, ProductDetail};
use gift_client::GiftClient;
use profile_client::Profile;
use share::ShareStyle;
use theme::{Pal, ThemeMode};

script_mod! {
    #(account_ui::script_mod(vm))
    use mod.prelude.liyu.*
    use mod.widgets.*

    mod.widgets.LiyuView = set_type_default() do #(LiyuView::register_widget(vm)) {
        ..mod.widgets.RectView
        width: Fill height: Fill
        show_bg: true
        draw_bg.color: liyu.bg
        flow: Right

        // ---- 左侧 208px 侧栏（平板 / 桌面）----
        sidebar := RoundedView {
            width: 208 height: Fill
            flow: Down
            padding: Inset{left: 12.0, right: 12.0, top: 20.0, bottom: 14.0}
            spacing: 6.0
            draw_bg +: {
                color: liyu.bg_chrome
                border_radius: 0.0
                border_size: 0.0
            }
            logo := View {
                width: Fill height: Fit
                flow: Down
                spacing: 2.0
                margin: Inset{left: 6.0, bottom: 16.0}
                title := Label {
                    flow: Right{wrap: true}
                    text: "礼遇 LiYu"
                    draw_text +: { color: liyu.ink text_style +: { font_size: 18.0 } }
                }
                slogan := Label {
                    flow: Right{wrap: true}
                    text: "猜得到的心意"
                    draw_text +: { color: liyu.ink_2 text_style +: { font_size: 12.5 } }
                }
            }
            tab_gift := LiyuTabIcon { text: "挑礼" draw_icon +: { svg: crate_resource("self:resources/icons/nav-gift.svg") } }
            tab_box := LiyuTabIcon { text: "礼盒" draw_icon +: { svg: crate_resource("self:resources/icons/nav-box.svg") } }
            tab_pact := LiyuTabIcon { text: "契约" draw_icon +: { svg: crate_resource("self:resources/icons/nav-pact.svg") } }
            tab_contacts := LiyuTabIcon { text: "熟人" draw_icon +: { svg: crate_resource("self:resources/icons/nav-contacts.svg") } }
            tab_me := LiyuTabIcon { text: "我" draw_icon +: { svg: crate_resource("self:resources/icons/nav-me.svg") } }
            sb_spacer := View { width: Fill height: Fill }
        }

        // ---- 内容壳 ----
        shell := RoundedView {
            width: Fill height: Fill
            // 外层 Overlay：导入菜单浮层叠在内容之上、不占布局。
            flow: Overlay
            draw_bg +: {
                color: liyu.bg
                border_color: liyu.line_soft
                border_size: 0.0
                border_radius: 0.0
            }
            shell_body := View {
                width: Fill height: Fill
                flow: Down

                // ---- 顶栏：标题居中；左「‹ 返回」（覆盖页）、右当页主动作 ----
                topbar := View {
                    width: Fill height: 58
                    flow: Overlay
                    tb_mid := View {
                        width: Fill height: Fill
                        align: Align{x: 0.5, y: 0.5}
                        tb_title := Label {
                            flow: Right{wrap: true}
                            width: Fit height: Fit
                            text: "礼遇 LiYu"
                            draw_text +: { color: liyu.warm text_style +: { font_size: 15.0 } }
                        }
                    }
                    tb_bar := View {
                        width: Fill height: Fill
                        flow: Right
                        align: Align{x: 0.0, y: 0.5}
                        padding: Inset{left: 16.0, right: 12.0}
                        spacing: 8.0
                        tb_back := LiyuLink {
                            visible: false
                            width: Fit
                            text: "返回"
                            draw_icon +: { svg: crate_resource("self:resources/icons/chevron-left.svg") }
                        }
                        tb_gap := View { width: Fill height: Fit }
                        tb_action := LiyuBtnPrimarySm { visible: false width: Fit text: "发起神秘送礼" }
                        tb_add := LiyuAddBtn {
                            visible: false
                            draw_icon +: { svg: crate_resource("self:resources/icons/plus.svg") }
                        }
                    }
                }

                // ---- 内容区 + 右列辅助栏 ----
                main := View {
                    width: Fill height: Fill
                    flow: Right
                    spacing: 0.0
                    padding: Inset{left: 0.0, right: 0.0, top: 8.0, bottom: 16.0}

                    content := View {
                        width: Fill height: Fill
                        flow: Overlay

                        pages := View {
                            width: Fill height: Fill
                            flow: Down

                        // ---- 通知条（礼物快过期 / 契约快到期）----
                        // 排在页面上方、占自己的高度，把页面往下推，不盖住内容。
                        notice_layer := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            padding: Inset{left: 16.0, right: 16.0, bottom: 8.0}
                            nt_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                padding: 14.0
                                spacing: 10.0
                                draw_bg +: { color: liyu.card border_color: liyu.line_notice border_size: 1.0 }
                                nt_icon := LiyuIcon {
                                    icon_walk: Walk{ width: 18.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/bell.svg") color: liyu.warm }
                                }
                                nt_col := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 3.0
                                    nt_title := Label {
                                        flow: Right{wrap: true}
                                        width: Fill
                                        text: ""
                                        draw_text +: { wrap: Words color: liyu.ink text_style +: { font_size: 13.0 line_spacing: 1.35 } }
                                    }
                                    nt_text := Label {
                                        flow: Right{wrap: true}
                                        width: Fill
                                        text: ""
                                        draw_text +: { wrap: Words color: liyu.ink_2 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                                    }
                                }
                                nt_go := LiyuBtnSm { width: Fit text: "去看看" }
                                nt_close := LiyuIconBtn {
                                    draw_icon +: { svg: crate_resource("self:resources/icons/close.svg") }
                                }
                            }
                        }

                        account_workspace := View {
                            width: Fill height: Fill flow: Right
 me_list := LiyuScrollY { width: 200 height: Fill flow: Down spacing: 12.0
 account_identity := LiyuCard { width: Fill height: Fit flow: Down spacing: 8.0 align: Align{x: 0.5, y: 0.0}
 account_avatar := LiyuThumb { width: 56 height: 56 }
 account_name := Label { width: Fill height: Fit align: Align{x: 0.5, y: 0.5} text: "" draw_text +: { color: liyu.ink wrap: None max_lines: 1 text_overflow: Ellipsis text_style +: { font_size: 13.0 } } }
 }
 account_info_head := LiyuMuted { text: "用户信息" }
 account_categories := LiyuCard { width: Fill height: Fit flow: Down spacing: 2.0
 acc_mcat0 := mod.widgets.AccountCategoryRow { text: "个人资料" }
 acc_mcat1 := mod.widgets.AccountCategoryRow { text: "账号与联系方式" }
 acc_mcat2 := mod.widgets.AccountCategoryRow { text: "收货地址" }
 }
 account_data_head := LiyuMuted { text: "我的数据" }
 account_shortcuts := LiyuCard { width: Fill height: Fit flow: Down spacing: 2.0
 account_wishes := mod.widgets.AccountCategoryRow { width: Fill text: "心愿单" }
 account_received := mod.widgets.AccountCategoryRow { width: Fill text: "收到的礼物" }
 account_sent := mod.widgets.AccountCategoryRow { width: Fill text: "送出的礼物" }
 account_drafts := mod.widgets.AccountCategoryRow { width: Fill text: "送礼草稿" }
 account_wallet := mod.widgets.AccountCategoryRow { width: Fill text: "钱包与流水" }
 }
 account_settings_head := LiyuMuted { text: "应用设置" }
 account_settings_menu := LiyuCard { width: Fill height: Fit flow: Down spacing: 2.0
 acc_mcat3 := mod.widgets.AccountCategoryRow { text: "通用" }
 acc_mcat4 := mod.widgets.AccountCategoryRow { text: "通知" }
 acc_mcat5 := mod.widgets.AccountCategoryRow { text: "数据管理" }
 acc_mcat6 := mod.widgets.AccountCategoryRow { text: "关于" }
 }
 }
                            workspace_pages := View { width: Fill height: Fill flow: Down
                        // ================= 挑礼 =================
                        page_gift := LiyuScrollY {
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            gf_seg := LiyuSegTrack {
                                gs_shop := LiyuSeg { text: "挑礼物" }
                                gs_wish := LiyuSeg { text: "熟人的心愿单" }
                            }
                            gf_shop := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 14.0
                                gf_chips := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    k0 := LiyuChip { text: "全部" }
                                    k1 := LiyuChip { text: "咖啡茶饮" }
                                    k2 := LiyuChip { text: "电影演出" }
                                    k3 := LiyuChip { text: "潮流小物" }
                                    k4 := LiyuChip { text: "盲盒" }
                                    k5 := LiyuChip { text: "甜点鲜花" }
                                    k6 := LiyuChip { text: "数码家电" }
                                    k7 := LiyuChip { text: "家居生活" }
                                    k8 := LiyuChip { text: "母婴亲子" }
                                }
                                // 商品宫格：列数和卡宽由 apply_shaping 按可用宽度算。
                                gf_grid := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 12.0
                                    spacing: 12.0
                                    pc0 := LiyuProductCard { }
                                    pc1 := LiyuProductCard { }
                                    pc2 := LiyuProductCard { }
                                    pc3 := LiyuProductCard { }
                                    pc4 := LiyuProductCard { }
                                    pc5 := LiyuProductCard { }
                                    pc6 := LiyuProductCard { }
                                    pc7 := LiyuProductCard { }
                                    pc8 := LiyuProductCard { }
                                    pc9 := LiyuProductCard { }
                                    pc10 := LiyuProductCard { }
                                    pc11 := LiyuProductCard { }
                                    pc12 := LiyuProductCard { }
                                    pc13 := LiyuProductCard { }
                                    pc14 := LiyuProductCard { }
                                    pc15 := LiyuProductCard { }
                                    pc16 := LiyuProductCard { }
                                    pc17 := LiyuProductCard { }
                                    pc18 := LiyuProductCard { }
                                    pc19 := LiyuProductCard { }
                                    pc20 := LiyuProductCard { }
                                    pc21 := LiyuProductCard { }
                                    pc22 := LiyuProductCard { }
                                    pc23 := LiyuProductCard { }
                                    pc24 := LiyuProductCard { }
                                    pc25 := LiyuProductCard { }
                                    pc26 := LiyuProductCard { }
                                    pc27 := LiyuProductCard { }
                                    pc28 := LiyuProductCard { }
                                    pc29 := LiyuProductCard { }
                                    pc30 := LiyuProductCard { }
                                    pc31 := LiyuProductCard { }
                                    pc32 := LiyuProductCard { }
                                }
                                gf_note := LiyuMuted {
                                    text: "点一件看详情。礼卡不出现礼物名和价格，解开谜题才揭晓。"
                                }
                            }
                            gf_wish := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                spacing: 12.0
                                gw_mine := LiyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 6.0
                                    spacing: 0.0
                                    row_mywish := LiyuSetRow { }
                                }
                                gw_head := LiyuGroupHead { text: "熟人的心愿单" }
                                fw0 := LiyuWishCard { }
                                fw1 := LiyuWishCard { }
                                fw2 := LiyuWishCard { }
                                fw3 := LiyuWishCard { }
                                fw4 := LiyuWishCard { }
                                fw5 := LiyuWishCard { }
                                gw_empty := LiyuEmpty {
                                    visible: false
                                    em_icon := LiyuIcon {
                                        icon_walk: Walk{ width: 28.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-gift.svg") color: liyu.ink_ghost }
                                    }
                                }
                                gw_note := LiyuMuted {
                                    text: "谁送了哪一件，揭晓前心愿单主人都不知道。你送出的那件，别人只会看到「已有人送」，不会撞礼。"
                                }
                            }
                        }

                        // ================= 礼盒 =================
                        page_box := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            bx_mode := LiyuBadgeBlue { text: "" }
                            bx_seg := LiyuSegTrack {
                                bs_recv := LiyuSeg { text: "收到的" }
                                bs_sent := LiyuSeg { text: "送出的" }
                            }
                            bx_list := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 6.0
                                spacing: 2.0
                                b0 := LiyuBoxRow { }
                                b1 := LiyuBoxRow { }
                                b2 := LiyuBoxRow { }
                                b3 := LiyuBoxRow { }
                                b4 := LiyuBoxRow { }
                                b5 := LiyuBoxRow { }
                                b6 := LiyuBoxRow { }
                                b7 := LiyuBoxRow { }
                                b8 := LiyuBoxRow { }
                                b9 := LiyuBoxRow { }
                                b10 := LiyuBoxRow { }
                                b11 := LiyuBoxRow { }
                                bx_empty := LiyuEmpty {
                                    visible: false
                                    em_icon := LiyuIcon {
                                        icon_walk: Walk{ width: 28.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-box.svg") color: liyu.ink_ghost }
                                    }
                                }
                                bx_more := LiyuMuted { visible: false margin: Inset{left: 12.0, top: 4.0, bottom: 6.0} text: "" }
                            }
                        }

                        // Online gift detail is separate from the legacy local demo gift state.
                        page_online_gift := LiyuScrollY {
                            visible: false width: Fill height: Fill flow: Down spacing: 14.0
                            og_mode := LiyuBadgeBlue { text: "服务器礼物" }
                            og_title := LiyuH2 { text: "" }
                            og_body := LiyuCard { width: Fill height: Fit flow: Down padding: 14.0 spacing: 10.0
                                og_summary := LiyuBody { text: "" }
                                og_clue := LiyuMuted { text: "" }
                                og_open := LiyuBtnPrimary { width: Fit text: "打开礼物" }
                                og_answer := LiyuInput { empty_text: "输入答案" }
                                og_answer_btn := LiyuBtn { width: Fit text: "提交答案" }
                                og_agree := LiyuChip { text: "我同意附加契约" }
                                og_ship_name := LiyuInput { empty_text: "收件人" }
                                og_ship_phone := LiyuInput { empty_text: "收件手机号" }
                                og_ship_address := LiyuInput { empty_text: "收件地址" }
                                og_accept := LiyuBtnPrimary { width: Fit text: "确认收下" }
                            }
                            og_logistics := LiyuCard { visible: false width: Fill height: Fit flow: Down padding: 14.0 spacing: 10.0
                                og_logistics_head := LiyuGroupHead { text: "物流信息" }
                                og_logistics_text := LiyuBody { text: "" }
                                og_confirm := LiyuBtn { width: Fit text: "确认收货" }
                            }
                            og_err := LiyuBad { visible: false text: "" }
                        }

                        // ================= 契约 =================
                        page_pact := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            pt_seg := LiyuSegTrack {
                                ps_mine := LiyuSeg { text: "我答应的" }
                                ps_theirs := LiyuSeg { text: "答应我的" }
                            }
                            pt_list := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 6.0
                                spacing: 2.0
                                p0 := LiyuPactRow { }
                                p1 := LiyuPactRow { }
                                p2 := LiyuPactRow { }
                                p3 := LiyuPactRow { }
                                p4 := LiyuPactRow { }
                                p5 := LiyuPactRow { }
                                p6 := LiyuPactRow { }
                                p7 := LiyuPactRow { }
                                pt_empty := LiyuEmpty {
                                    visible: false
                                    em_icon := LiyuIcon {
                                        icon_walk: Walk{ width: 28.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-pact.svg") color: liyu.ink_ghost }
                                    }
                                }
                            }
                            pt_note := LiyuMuted {
                                text: "契约是收礼时答应的一件小事。逾期无惩罚，兑现了记得标记。"
                            }
                        }

                        // ================= 熟人 =================
                        page_contacts := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            ct_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 12.0
                                ct_head := LiyuH2 { text: "我的熟人" }
                                ct_add := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    ca_input := LiyuInput { empty_text: "称呼，比如「老陈」" }
                                    ca_btn := LiyuBtn { width: Fit text: "添加" }
                                }
                                ct_phone := LiyuInput { empty_text: "手机号，多个用分号分隔；国际号码带国家码" }
                                ct_email := LiyuInput { empty_text: "邮箱，多个用分号分隔" }
                                ct_cancel_edit := LiyuBtnSm { visible: false width: Fit text: "取消修改" }
                                ct_paging := View { width: Fill height: Fit flow: Right spacing: 8.0 ct_prev := LiyuBtnSm { width: Fit text: "上一页" } ct_next := LiyuBtnSm { width: Fit text: "下一页" } }
                                ct_hint := LiyuMuted { text: "联系人只属于你的地址簿，导入不会创建平台好友关系。" }
                                ct_preview := View { visible: false width: Fill height: Fit flow: Down spacing: 8.0
                                    ct_preview_text := LiyuMuted { width: Fill text: "" }
                                    ct_import_confirm := LiyuBtnPrimary { width: Fit text: "确认导入（相同联系方式合并）" }
                                    ct_import_cancel := LiyuBtnSm { width: Fit text: "取消导入" }
                                }
                                ca_err := LiyuBad { visible: false text: "" }
                                ct_rows := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 2.0
                                    c0 := LiyuContactRow { }
                                    c1 := LiyuContactRow { }
                                    c2 := LiyuContactRow { }
                                    c3 := LiyuContactRow { }
                                    c4 := LiyuContactRow { }
                                    c5 := LiyuContactRow { }
                                    c6 := LiyuContactRow { }
                                    c7 := LiyuContactRow { }
                                    c8 := LiyuContactRow { }
                                    c9 := LiyuContactRow { }
                                }
                                ct_empty := LiyuEmpty {
                                    visible: false
                                    em_icon := LiyuIcon {
                                        icon_walk: Walk{ width: 28.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-contacts.svg") color: liyu.ink_ghost }
                                    }
                                }
                                ct_more := LiyuMuted { visible: false text: "" }
                            }
                        }

                        // ================= 我 =================
                        page_me := View {
 // Keep account edits inside their own draw list, separate from app navigation.
 new_batch: true
 visible: false width: Fill height: Fill flow: Right spacing: 16.0

 account_detail := View { width: Fill height: Fill flow: Down spacing: 12.0
 account_header := View { width: Fill height: Fit flow: Right spacing: 8.0
 acc_back := LiyuBtnSm { width: Fit text: "‹ 返回" }
 acc_title := LiyuH2 { width: Fill text: "我" }
 }
 account_landing := LiyuScrollY { width: Fill height: Fill flow: Down spacing: 14.0
 me_identity := LiyuCard { width: Fill height: Fit flow: Down spacing: 8.0
 me_identity_name := LiyuH2 { text: "" }
 me_identity_note := LiyuMuted { text: "账号与设置" }
 }
                            me_bal := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 6.0
                                me_bal_l := LiyuMuted { text: "礼遇余额" }
                                me_bal_v := LiyuH1 { text: "¥0" }
                                me_bal_row := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    me_bal_n := LiyuMuted { text: "只在礼遇内使用 · 折现、换购退差都进这里" }
                                    me_wallet := LiyuBtnSm { width: Fit text: "钱包与流水" }
                                }
                            }
                            account_draft_empty := LiyuMuted { visible: false text: "暂无待确认的送礼草稿" }
                            me_draft := LiyuCard {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                padding: 14.0
                                spacing: 8.0
                                draw_bg +: { color: liyu.card border_color: liyu.line_notice border_size: 1.0 }
                                me_draft_l := LiyuMuted { text: "AI 开好的送礼草稿，待本人确认" }
                                me_draft_t := LiyuWarmText { text: "" }
                                me_draft_note := LiyuMuted { text: "" }
                                me_draft_row := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    me_draft_confirm := LiyuBtnPrimary { width: Fit text: "确认草稿（未发送）" }
                                    me_draft_cancel := LiyuBtnSm { width: Fit text: "取消草稿" }
                                }
                            }
                            me_stats := View {
                                width: Fill height: Fit
                                flow: Right
                                spacing: 10.0
                                ms_sent := LiyuCard2 {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 2.0
                                    ms_sent_v := LiyuH2 { text: "0" }
                                    ms_sent_l := LiyuMuted { text: "送出" }
                                }
                                ms_recv := LiyuCard2 {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 2.0
                                    ms_recv_v := LiyuH2 { text: "0" }
                                    ms_recv_l := LiyuMuted { text: "收到" }
                                }
                                ms_pact := LiyuCard2 {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 2.0
                                    ms_pact_v := LiyuH2 { text: "0" }
                                    ms_pact_l := LiyuMuted { text: "已兑现契约" }
                                }
                            }
                            me_rows := LiyuCard {
                                visible: true
                                width: Fill height: Fit
                                flow: Down
                                padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 6.0}
                                spacing: 0.0
                                row_profile := LiyuSetRow { }
                                row_wish := LiyuSetRow { }
                                row_wallet := LiyuSetRow { }
                                row_settings := LiyuSetRow { }
                                row_about := LiyuSetRow { }
                            }
 }
                        page_profile := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            pf_status := LiyuBadgeBlue { text: "" }
                            pf_signout_row := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                pf_signout_note := LiyuMuted { width: Fill text: "登出会清除本机凭据并回到登录界面" }
                                pf_signout := LiyuBtn { width: Fit text: "登出" }
                            }
                            pf_name_head := LiyuGroupHead { text: "基本资料" }
                            pf_name_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 14.0
                                spacing: 10.0
                                pf_name_note := LiyuMuted { text: "显示名" }
                                pf_name_summary := View { width: Fill height: Fit flow: Right spacing: 8.0
 pf_name_value := LiyuMuted { width: Fill text: "" } pf_name_edit := LiyuBtnSm { width: Fit text: "修改" } }
 pf_name_editor := View { visible: false width: Fill height: Fit pf_name := LiyuInput { empty_text: "你的名字"  } }
                                pf_avatar_note := LiyuMuted { text: "头像（本地图片，JPEG / PNG / WebP）" }
                                pf_avatar_row := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    pf_avatar_thumb := LiyuThumb { width: 64 height: 64 }
                                    pf_avatar_col := View {
                                        width: Fill height: Fit
                                        flow: Down
                                        spacing: 6.0
                                        pf_avatar_status := LiyuMuted { text: "还没有头像，显示默认占位图" }
                                        pf_avatar_pick_row := View {
                                            width: Fill height: Fit
                                            flow: Right
                                            spacing: 8.0
                                            pf_avatar_load := LiyuBtn { width: Fit text: "选择图片…" }
                                        }
                                    }
                                }
                                pf_avatar_edit := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    pf_avatar_edit_note := LiyuMuted { text: "已载入原图：选裁切位置（九宫格循环）、旋转，确认后上传。" }
                                    pf_avatar_edit_row := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true}
                                        wrap_spacing: 8.0
                                        spacing: 8.0
                                        pf_avatar_anchor := LiyuBtnSm { width: Fit text: "裁切位置：居中" }
                                        pf_avatar_rotate := LiyuBtnSm { width: Fit text: "旋转 90°" }
                                        pf_avatar_confirm := LiyuBtnPrimary { width: Fit text: "确认上传" }
                                        pf_avatar_cancel := LiyuBtnSm { width: Fit text: "取消" }
                                    }
                                }
                                pf_avatar_delete := LiyuBtnSm { visible: false width: Fit text: "删除头像，恢复默认" }
                                pf_save := LiyuBtnPrimary { visible: false width: Fit text: "保存" }
                            }
                            pf_contact_head := LiyuGroupHead { text: "联系方式" }
                            pf_contact_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 14.0
                                spacing: 10.0
                                pf_contact_note := LiyuMuted { text: "验证绑定手机号或邮箱后，该联系方式收到的待领取礼物会自动归入你的礼盒。" }
                                pf_phone_summary := View { width: Fill height: Fit flow: Right spacing: 8.0 pf_phone_value := LiyuMuted { width: Fill text: "" } pf_phone_edit := LiyuBtnSm { width: Fit text: "修改手机号" } }
 pf_phone_editor := View { visible: false width: Fill height: Fit pf_phone := LiyuInput { empty_text: "手机号（国际号码带国家码）"  } }
                                pf_phone_code_editor := View { visible: false width: Fill height: Fit flow: Down spacing: 8.0 pf_phone_request := LiyuBtnSm { width: Fit text: "获取验证码" } pf_phone_code := LiyuInput { empty_text: "验证码" } }
                                pf_phone_save := LiyuBtnPrimary { visible: false width: Fit text: "确认绑定" }
                                pf_email_summary := View { width: Fill height: Fit flow: Right spacing: 8.0 pf_email_value := LiyuMuted { width: Fill text: "" } pf_email_edit := LiyuBtnSm { width: Fit text: "修改邮箱" } }
 pf_email_editor := View { visible: false width: Fill height: Fit pf_email := LiyuInput { empty_text: "邮箱"  } }
                                pf_email_code_editor := View { visible: false width: Fill height: Fit flow: Down spacing: 8.0 pf_email_request := LiyuBtnSm { width: Fit text: "获取验证码" } pf_email_code := LiyuInput { empty_text: "验证码" } }
                                pf_email_save := LiyuBtnPrimary { visible: false width: Fit text: "确认绑定" }
                            }
                            pf_address_head := LiyuGroupHead { text: "收货地址" }
                            pf_address_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 14.0
                                spacing: 10.0
                                pf_address_list := LiyuMuted { text: "还没有保存地址" }
                                pf_a0 := LiyuCard2 { visible: false flow: Down spacing: 6.0 pf_a0_text := LiyuMuted { text: "" } pf_a0_actions := View { width: Fill height: Fit flow: Right spacing: 8.0 pf_a0_edit := LiyuBtnSm { width: Fit text: "编辑" } pf_a0_del := LiyuBtnSm { width: Fit text: "删除" } } }
                                pf_a1 := LiyuCard2 { visible: false flow: Down spacing: 6.0 pf_a1_text := LiyuMuted { text: "" } pf_a1_actions := View { width: Fill height: Fit flow: Right spacing: 8.0 pf_a1_edit := LiyuBtnSm { width: Fit text: "编辑" } pf_a1_del := LiyuBtnSm { width: Fit text: "删除" } } }
                                pf_a2 := LiyuCard2 { visible: false flow: Down spacing: 6.0 pf_a2_text := LiyuMuted { text: "" } pf_a2_actions := View { width: Fill height: Fit flow: Right spacing: 8.0 pf_a2_edit := LiyuBtnSm { width: Fit text: "编辑" } pf_a2_del := LiyuBtnSm { width: Fit text: "删除" } } }
                                pf_a3 := LiyuCard2 { visible: false flow: Down spacing: 6.0 pf_a3_text := LiyuMuted { text: "" } pf_a3_actions := View { width: Fill height: Fit flow: Right spacing: 8.0 pf_a3_edit := LiyuBtnSm { width: Fit text: "编辑" } pf_a3_del := LiyuBtnSm { width: Fit text: "删除" } } }
                                pf_a4 := LiyuCard2 { visible: false flow: Down spacing: 6.0 pf_a4_text := LiyuMuted { text: "" } pf_a4_actions := View { width: Fill height: Fit flow: Right spacing: 8.0 pf_a4_edit := LiyuBtnSm { width: Fit text: "编辑" } pf_a4_del := LiyuBtnSm { width: Fit text: "删除" } } }
                                pf_a5 := LiyuCard2 { visible: false flow: Down spacing: 6.0 pf_a5_text := LiyuMuted { text: "" } pf_a5_actions := View { width: Fill height: Fit flow: Right spacing: 8.0 pf_a5_edit := LiyuBtnSm { width: Fit text: "编辑" } pf_a5_del := LiyuBtnSm { width: Fit text: "删除" } } }
                                pf_addr_name_editor := View { visible: false width: Fill height: Fit pf_addr_name := LiyuInput { empty_text: "收件人"  } }
                                pf_addr_phone_editor := View { visible: false width: Fill height: Fit pf_addr_phone := LiyuInput { empty_text: "收件手机号"  } }
                                pf_addr_text_editor := View { visible: false width: Fill height: Fit pf_addr_text := LiyuInput { empty_text: "详细地址"  } }
                                pf_addr_new := LiyuBtnSm { width: Fit text: "添加地址" }
 pf_addr_add := LiyuBtnPrimary { visible: false width: Fit text: "保存" }
                                pf_addr_cancel := LiyuBtnSm { visible: false width: Fit text: "取消" }
                            }
                            pf_err := LiyuBad { visible: false text: "" }
                        }
                        page_settings := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0

                            ap_head := LiyuGroupHead { text: "外观" }
                            ap_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 14.0
                                spacing: 10.0
                                ap_seg := LiyuSegTrack {
                                    ap_dark := LiyuSeg { text: "深色" }
                                    ap_light := LiyuSeg { text: "浅色" }
                                }
                                ap_note := LiyuMuted { text: "只改这台设备上的礼遇，不动系统设置。" }
                            }

                            nk_head := LiyuGroupHead { text: "我的称呼" }
                            nk_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 14.0
                                spacing: 10.0
                                nk_note := LiyuMuted { text: "「猜我是谁」时 TA 要猜的名字。可以用「/」写多个，比如「阿岚/岚岚」，猜中任何一个都算对。" }
                                nk_row := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    nk_value := LiyuMuted { width: Fill text: "" }
 nk_edit := LiyuBtnSm { width: Fit text: "修改" }
 nk_input_editor := View { visible: false width: Fill height: Fit nk_input := LiyuInput { empty_text: "你的称呼"  } }
                                    nk_save := LiyuBtn { visible: false width: Fit text: "保存" }
                                }
                                nk_err := LiyuBad { visible: false text: "" }
                            }

                            ntf_head := LiyuGroupHead { text: "通知" }
                            ntf_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 8.0}
                                spacing: 0.0
                                row_ntf_gift := LiyuSwitchRow { }
                                row_ntf_pact := LiyuSwitchRow { }
                                row_ntf_wish := LiyuSwitchRow { }
                                ntf_note := LiyuMuted {
                                    margin: Inset{left: 12.0, right: 12.0, top: 6.0}
                                    text: "不做「TA 刚打开了你的礼卡」这类实时提醒 —— 悬念留给对方。"
                                }
                            }

                            data_head := LiyuGroupHead { text: "数据" }
                            data_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 8.0}
                                spacing: 0.0
                                row_export := LiyuSetRow { }
                                row_reset := LiyuSetRow { }
                                reset_confirm := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    margin: Inset{left: 12.0, right: 12.0, top: 4.0}
                                    spacing: 8.0
                                    rc_text := LiyuBad { text: "会清掉本机的礼物、契约、流水和熟人，换回演示数据；深浅、称呼以外的设置也会复原。" }
                                    rc_row := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true}
                                        wrap_spacing: 8.0
                                        spacing: 8.0
                                        rc_cancel := LiyuBtn { width: Fit text: "再想想" }
                                        rc_ok := LiyuBtnDanger { width: Fit text: "确认恢复" }
                                    }
                                }
                            }
                        }
 account_about := LiyuCard { visible: false width: Fill height: Fit flow: Down spacing: 12.0
 LiyuH2 { text: "礼遇 LiYu" }
 LiyuMuted { text: "猜得到的心意。" }
 LiyuMuted { text: "版本 0.1.0" }
 account_intro := LiyuBtnSm { width: Fit text: "使用介绍" }
 }
 account_error := LiyuBad { text: "" }
 account_edit_bar := View { visible: false width: Fill height: Fit flow: Right spacing: 8.0
 account_cancel := LiyuBtn { width: 80 height: 32 padding: Inset{left: 8.0, right: 8.0, top: 4.0, bottom: 4.0} draw_text +: { text_style +: { font_size: 13.0 } } text: "取消" }
 account_save := LiyuBtnPrimary { width: 80 height: 32 padding: Inset{left: 8.0, right: 8.0, top: 4.0, bottom: 4.0} draw_text +: { text_style +: { font_size: 13.0 } } text: "保存" }
 }
 }
 }


                        // ================= 送礼页（覆盖页）=================
                        //
                        // 单页向导：礼物 / 送给谁 / 1 解密游戏 / 2 契约 / 3 寄语，
                        // 付款条固定在底部，不跟着滚动 —— 否则得先滚一屏才能点「发送」。
                        page_send := View {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 10.0
                            sd_scroll := LiyuScrollY {
                                width: Fill height: Fill
                                flow: Down
                                spacing: 14.0
                                sd_banner := LiyuBadgeBlue { visible: false text: "" draw_text +: { text_style +: { font_size: 13.0 } } }
                                sd_item := LiyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 14.0
                                    spacing: 10.0
                                    // 「换一件」放在标题行右边，手机上商品名才有整行宽。
                                    sd_item_top := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 8.0
                                        sd_item_head := LiyuGroupHead { margin: 0.0 text: "已选礼物" }
                                        sd_change := LiyuBtnSm { width: Fit text: "换一件" }
                                    }
                                    sd_item_row := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 12.0
                                        sd_img := LiyuThumb { width: 56 height: 56 }
                                        sd_col := View {
                                            width: Fill height: Fit
                                            flow: Down
                                            spacing: 2.0
                                            sd_name := Label {
                                                flow: Right{wrap: true}
                                                width: Fill
                                                max_lines: 1
                                                text_overflow: Ellipsis
                                                padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                                                text: ""
                                                draw_text +: { color: liyu.ink text_style +: { font_size: 15.0 } }
                                            }
                                            sd_sub := Label {
                                                flow: Right{wrap: true}
                                                width: Fill
                                                max_lines: 1
                                                text_overflow: Ellipsis
                                                padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                                                text: ""
                                                draw_text +: { color: liyu.ink_2 text_style +: { font_size: 12.0 } }
                                            }
                                            sd_price := Label {
                                                flow: Right{wrap: true}
                                                width: Fit
                                                padding: Inset{left: 3.0, right: 3.0, top: 2.0, bottom: 0.0}
                                                text: ""
                                                draw_text +: { color: liyu.warm text_style +: { font_size: 15.0 } }
                                            }
                                        }
                                    }
                                    // 「换一件」就地展开整份目录，点一行即换、收起。
                                    sd_pick := View {
                                        visible: false
                                        width: Fill height: Fit
                                        flow: Down
                                        spacing: 2.0
                                        q0 := LiyuGiftRow { }
                                        q1 := LiyuGiftRow { }
                                        q2 := LiyuGiftRow { }
                                        q3 := LiyuGiftRow { }
                                        q4 := LiyuGiftRow { }
                                        q5 := LiyuGiftRow { }
                                        q6 := LiyuGiftRow { }
                                        q7 := LiyuGiftRow { }
                                        q8 := LiyuGiftRow { }
                                        q9 := LiyuGiftRow { }
                                        q10 := LiyuGiftRow { }
                                        q11 := LiyuGiftRow { }
                                    }
                                }

                                sd_to_head := LiyuH3 { text: "送给谁（仅自己可见）" }
                                sd_peers := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    sp0 := LiyuChip { text: "" }
                                    sp1 := LiyuChip { text: "" }
                                    sp2 := LiyuChip { text: "" }
                                    sp3 := LiyuChip { text: "" }
                                    sp4 := LiyuChip { text: "" }
                                    sp5 := LiyuChip { text: "先不指定" }
                                }
                                // 认领心愿单时收礼人是定的：芯片收起，换成这一行。
                                sd_peer_fixed := LiyuBody { visible: false text: "" }
                                sd_dv := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    sd_dv_h := LiyuH3 { margin: Inset{top: 6.0} text: "什么时候送到" }
                                    sd_dv_seg := LiyuSegTrack {
                                        dv0 := LiyuSeg { text: "" }
                                        dv1 := LiyuSeg { text: "现在就送" }
                                    }
                                    sd_dv_n := LiyuMuted { text: "" }
                                }

                                sd_s1 := LiyuH3 { margin: Inset{top: 6.0} text: "1 · 设置解密游戏" }
                                sd_unlock := LiyuSegTrack {
                                    un0 := LiyuSeg { text: "猜我是谁" }
                                    un1 := LiyuSeg { text: "私密问答" }
                                    un2 := LiyuSeg { text: "专属暗号" }
                                    un3 := LiyuSeg { text: "直接领取" }
                                }
                                sd_game := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    // TextInput 没有 visible：显隐落在外面这层 View 上。
                                    sd_clue_box := View {
                                        width: Fill height: Fit flow: Down spacing: 8.0
                                        sd_clue_l := LiyuMuted { text: "线索" }
                                        sd_clue := LiyuInput { empty_text: "" }
                                    }
                                    sd_ans_box := View {
                                        visible: false
                                        width: Fill height: Fit flow: Down spacing: 8.0
                                        sd_ans_l := LiyuMuted { text: "答案" }
                                        sd_ans := LiyuInput { empty_text: "" }
                                    }
                                    sd_nick := LiyuMuted { text: "" }
                                    sd_free := LiyuMuted { visible: false text: "TA 打开礼卡直接领取，不用答题。惊喜少一点，但一定拆得开。" }
                                }

                                // 开关行自带 12 的左右内边距（给悬停底色留位），这里往外挪 12，文字和各段标题对齐。
                                sd_pact_row := LiyuSwitchRow { margin: Inset{left: -12.0, right: -12.0, top: 6.0} }
                                sd_pact := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    sd_presets := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true}
                                        wrap_spacing: 8.0
                                        spacing: 8.0
                                        pp0 := LiyuChip { text: "回请咖啡" }
                                        pp1 := LiyuChip { text: "晒一晒" }
                                        pp2 := LiyuChip { text: "陪看电影" }
                                        pp3 := LiyuChip { text: "见面拥抱" }
                                    }
                                    sd_pact_in := LiyuInput { empty_text: "自定义契约，最多 24 字" }
                                    sd_pact_note := LiyuMuted { text: "契约只写轻约定，不涉及钱。收下即答应，折现或换购作废。" }
                                }

                                sd_s3 := LiyuH3 { margin: Inset{top: 6.0} text: "3 · 寄语" }
                                sd_msg := LiyuInput { empty_text: "给 TA 的一句话，最多 40 字（揭晓后才看得到）" }

                            }
                            // 固定底栏只留一句小结和「去结算」，别和表单抢高度；怎么付在结算页里选。
                            sd_bar := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 6.0
                                padding: Inset{top: 8.0}
                                sd_split := LiyuMuted { text: "" }
                                sd_err := LiyuBad { visible: false text: "" }
                                sd_go := LiyuBtnPrimary {
                                    width: Fill
                                    text: "去结算"
                                    draw_icon +: { svg: crate_resource("self:resources/icons/wallet.svg") }
                                }
                            }
                        }

                        // ================= 礼卡页（覆盖页）=================
                        page_card := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            cd_row := View {
                                width: Fill height: Fit
                                flow: Right
                                spacing: 18.0
                                cd_prev := View {
                                    width: Fit height: Fit
                                    flow: Down
                                    cd_share := LiyuShareCard { }
                                }
                                cd_right := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 10.0
                                    cd_ok := LiyuWarmText { text: "礼卡已生成。发给 TA，等 TA 来拆。" }
                                    cd_link := LiyuMuted { text: "" }
                                    cd_style_l := LiyuGroupHead { text: "礼卡样式" }
                                    cd_style := LiyuSegTrack {
                                        cs_warm := LiyuSeg { text: "暖杏" }
                                        cs_night := LiyuSeg { text: "夜蓝" }
                                    }
                                    cd_btns := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true}
                                        wrap_spacing: 8.0
                                        spacing: 8.0
                                        cd_copy := LiyuBtn { width: Fit text: "复制链接" draw_icon +: { svg: crate_resource("self:resources/icons/share.svg") } }
                                        cd_save := LiyuBtn { width: Fit text: "保存礼卡图片" draw_icon +: { svg: crate_resource("self:resources/icons/download.svg") } }
                                        cd_peek := LiyuBtn { width: Fit text: "以 TA 的视角看看" draw_icon +: { svg: crate_resource("self:resources/icons/eye.svg") } }
                                    }
                                    cd_saved := LiyuMuted { visible: false text: "" }
                                    cd_note := LiyuMuted { text: "礼卡上只有玩法和线索，礼物名、价格和你的名字都不出现。" }
                                    cd_done := LiyuBtnPrimary { width: Fill text: "完成" }
                                }
                            }
                        }

                        // ================= 拆礼页（覆盖页）=================
                        //
                        // 一页四个阶段：解密 → 揭晓 → 收下 / 换购折现 → 完成。
                        // 每个阶段是一块 View，同时只露一块。
                        page_open := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0

                            op_preview := LiyuBadgeBlue { visible: false text: "TA 打开礼卡时看到的样子（预览，不能作答）" draw_text +: { text_style +: { font_size: 13.0 } } }

                            // ---- 解密 ----
                            op_decrypt := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 12.0
                                od_icon := LiyuIconWarm {
                                    icon_walk: Walk{ width: 36.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/nav-box.svg") }
                                }
                                od_head := LiyuH2 { text: "你收到一份神秘礼物" }
                                od_badge := LiyuBadgeBlue { text: "" draw_text +: { text_style +: { font_size: 12.5 } } }
                                od_card := LiyuCard2 {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 6.0
                                    od_ctitle := LiyuMuted { text: "" }
                                    od_clue := LiyuH3 { text: "" }
                                }
                                od_cands := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    align: Align{x: 0.0, y: 0.5}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    od_cl := Label {
                                        flow: Right{wrap: true}
                                        width: Fit
                                        text: "可能是："
                                        draw_text +: { color: liyu.ink_2 text_style +: { font_size: 13.0 } }
                                    }
                                    cd0 := LiyuBtnSm { width: Fit text: "" }
                                    cd1 := LiyuBtnSm { width: Fit text: "" }
                                    cd2 := LiyuBtnSm { width: Fit text: "" }
                                    cd3 := LiyuBtnSm { width: Fit text: "" }
                                    cd4 := LiyuBtnSm { width: Fit text: "" }
                                    cd5 := LiyuBtnSm { width: Fit text: "" }
                                }
                                od_in_box := View {
                                    width: Fill height: Fit
                                    od_input := LiyuInput { empty_text: "输入 TA 的名字" }
                                }
                                od_wrong := LiyuBad { visible: false text: "" }
                                od_bar := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    od_left := LiyuMuted { text: "" }
                                    od_submit := LiyuBtnPrimary { width: Fit text: "提交答案" }
                                }
                                od_note := LiyuMuted { text: "猜错了礼物也不会消失 —— 机会用完照样拆开，只是不说是谁。" }
                                od_tip := LiyuMuted { visible: false text: "" draw_text +: { color: liyu.ink_3 } }
                            }

                            // ---- 揭晓 ----
                            op_reveal := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                spacing: 12.0
                                or_head := LiyuH2 { text: "" }
                                or_gift := LiyuGiftCard { }
                                or_value := LiyuMuted { text: "" }
                                or_pact := LiyuCard2 {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 6.0
                                    or_pt := LiyuWarmText { text: "附加契约" }
                                    or_ptext := LiyuBody { text: "" }
                                }
                                or_acts := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 10.0
                                    or_accept := LiyuBtnPrimary {
                                        width: Fill
                                        text: "满意，开心收下"
                                        draw_icon +: { svg: crate_resource("self:resources/icons/check.svg") }
                                    }
                                    or_swap := LiyuBtn {
                                        width: Fill
                                        text: "不太喜欢？换购 / 折现"
                                        draw_icon +: { svg: crate_resource("self:resources/icons/refresh.svg") }
                                    }
                                }
                            }

                            // ---- 收下 ----
                            op_accept := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                spacing: 10.0
                                oa_title := LiyuH3 { text: "" }
                                // 同意契约用开关行：开/关一眼看得出，契约长也能换行（胶囊不换行，手机上放不下）。
                                oa_agree := LiyuSwitchRow { visible: false margin: Inset{left: -12.0, right: -12.0} }
                                oa_ship := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    oa_ship_l := LiyuMuted { text: "收件信息只给这一单用，也留在本机方便下次预填。" }
                                    oa_name := LiyuInput { empty_text: "收件人" }
                                    oa_phone := LiyuInput { empty_text: "手机号（国际号码带国家码）" }
                                    oa_addr := LiyuInput { empty_text: "收件地址" }
                                }
                                oa_ev := LiyuMuted { visible: false text: "这是电子券，收下后立即发放券码。" }
                                oa_err := LiyuBad { visible: false text: "" }
                                oa_bar := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    oa_back := LiyuBtn { width: Fit text: "再看看" }
                                    oa_gap := View { width: Fill height: Fit }
                                    oa_ok := LiyuBtnPrimary { width: Fit text: "确认收下" }
                                }
                            }

                            // ---- 换购 / 折现 ----
                            op_swap := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                spacing: 10.0
                                ow_seg := LiyuSegTrack {
                                    ow_cash := LiyuSeg { text: "折成余额" }
                                    ow_swap := LiyuSeg { text: "换一份" }
                                }
                                ow_cashbox := LiyuCard2 {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 6.0
                                    ow_l1 := LiyuBody { text: "" }
                                    ow_l2 := LiyuBody { text: "" }
                                    ow_l3 := LiyuH3 { text: "" }
                                }
                                ow_list := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 2.0
                                    ow_credit := LiyuMuted { text: "" }
                                    x0 := LiyuChoice { }
                                    x1 := LiyuChoice { }
                                    x2 := LiyuChoice { }
                                    x3 := LiyuChoice { }
                                    x4 := LiyuChoice { }
                                    x5 := LiyuChoice { }
                                    x6 := LiyuChoice { }
                                    x7 := LiyuChoice { }
                                    x8 := LiyuChoice { }
                                    x9 := LiyuChoice { }
                                    x10 := LiyuChoice { }
                                }
                                ow_ship := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    ow_ship_l := LiyuMuted { text: "换成了实物，填一下收件信息：" }
                                    ow_name := LiyuInput { empty_text: "收件人" }
                                    ow_phone := LiyuInput { empty_text: "手机号（国际号码带国家码）" }
                                    ow_addr := LiyuInput { empty_text: "收件地址" }
                                }
                                ow_calc := LiyuMuted { text: "" }
                                ow_void := LiyuMuted { visible: false text: "换购 / 折现后，这份礼物附带的契约作废。" }
                                ow_err := LiyuBad { visible: false text: "" }
                                ow_bar := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    ow_back := LiyuBtn { width: Fit text: "再看看" }
                                    ow_gap := View { width: Fill height: Fit }
                                    ow_ok := LiyuBtnPrimary { width: Fit text: "确认折现" }
                                }
                            }

                            // ---- 完成 ----
                            op_done := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                spacing: 10.0
                                odn_icon := LiyuIcon {
                                    icon_walk: Walk{ width: 36.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/check-circle.svg") color: liyu.good }
                                }
                                odn_title := LiyuH2 { text: "" }
                                odn_text := LiyuBody { text: "" }
                                odn_hint := LiyuCard2 {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 4.0
                                    odn_hint_t := LiyuWarmText { text: "" draw_text +: { text_style +: { font_size: 14.0 } } }
                                }
                                odn_bar := View {
                                    width: Fill height: Fit
                                    // 主按钮 44 高、次按钮 36 高，按行居中才不高低错开。
                                    flow: Right{wrap: true row_align: RowAlign.Center}
                                    wrap_spacing: 8.0
                                    spacing: 10.0
                                    odn_return := LiyuBtnPrimary {
                                        width: Fit
                                        text: "给 TA 回一份礼"
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-gift.svg") }
                                    }
                                    odn_home := LiyuBtn { width: Fit text: "回礼盒" }
                                }
                            }
                        }

                        // ================= 送出详情（覆盖页）=================
                        page_sent := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            ss_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                ss_head := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 12.0
                                    ss_img := LiyuThumb { width: 64 height: 64 }
                                    ss_title := LiyuH2 { text: "" }
                                    ss_price := Label {
                                        flow: Right{wrap: true}
                                        width: Fit
                                        text: ""
                                        draw_text +: { color: liyu.ink text_style +: { font_size: 16.0 } }
                                    }
                                }
                                ss_state := LiyuBadge { text: "" draw_text +: { text_style +: { font_size: 12.5 } } }
                                ss_acts := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    align: Align{x: 0.0, y: 0.5}
                                    wrap_spacing: 6.0
                                    spacing: 8.0
                                    ss_copy := LiyuBtnSm { width: Fit text: "复制链接" }
                                    ss_view := LiyuBtnSm { width: Fit text: "看礼卡" }
                                }
                            }
                            ss_tl := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 2.0
                                ss_tl_head := LiyuGroupHead { margin: Inset{bottom: 4.0} text: "进度" }
                                t0 := LiyuStep { }
                                t1 := LiyuStep { }
                                t2 := LiyuStep { }
                                t3 := LiyuStep { }
                                t4 := LiyuStep { }
                            }
                            ss_info := LiyuCard2 {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 6.0
                                ss_play := LiyuBody { text: "" }
                                ss_pact := LiyuBody { text: "" }
                                ss_wish := LiyuBody { visible: false text: "" }
                                ss_msg := LiyuMuted { text: "" }
                            }
                            ss_withdraw := LiyuBtnDanger { visible: false width: Fit text: "撤回并退款" draw_icon +: { svg: crate_resource("self:resources/icons/undo.svg") } }
                            ss_confirm := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                ss_ctext := LiyuBad { text: "" }
                                ss_crow := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    ss_cno := LiyuBtn { width: Fit text: "再想想" }
                                    ss_cyes := LiyuBtnDanger { width: Fit text: "确认撤回" }
                                }
                            }
                            ss_sim := LiyuCard2 {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                ss_sim_t := LiyuWarmText { text: "演示" }
                                ss_sim_n := LiyuMuted { text: "收礼人不在这台设备上。点一下替 TA 推进一步，看看你会收到什么。" }
                                ss_sim_go := LiyuBtn { width: Fit text: "模拟 TA 的下一步" draw_icon +: { svg: crate_resource("self:resources/icons/sparkle.svg") } }
                            }
                        }

                        // ================= 独立收礼 / 送礼列表 =================
                        page_account_received := LiyuScrollY {
                            visible: false width: Fill height: Fill flow: Down spacing: 14.0
                            received_gift_note := LiyuMuted { text: "收到的礼物" }
                            received_gift_retry := LiyuBtnSm { width: Fit text: "刷新" }
                            received_gift_list := LiyuCard { width: Fill height: Fit flow: Down padding: 6.0 spacing: 0.0
                                received_gift_0 := LiyuBoxRow { }
                                received_gift_1 := LiyuBoxRow { }
                                received_gift_2 := LiyuBoxRow { }
                                received_gift_3 := LiyuBoxRow { }
                                received_gift_4 := LiyuBoxRow { }
                                received_gift_5 := LiyuBoxRow { }
                                received_gift_6 := LiyuBoxRow { }
                                received_gift_7 := LiyuBoxRow { }
                                received_gift_8 := LiyuBoxRow { }
                                received_gift_9 := LiyuBoxRow { }
                                received_gift_10 := LiyuBoxRow { }
                                received_gift_11 := LiyuBoxRow { }
                                received_gift_empty := LiyuEmpty { }
                            }
                            received_gift_more := LiyuMuted { visible: false text: "" }
                        }
                        page_account_sent := LiyuScrollY {
                            visible: false width: Fill height: Fill flow: Down spacing: 14.0
                            sent_gift_note := LiyuMuted { text: "送出的礼物" }
                            sent_gift_retry := LiyuBtnSm { width: Fit text: "刷新" }
                            sent_gift_list := LiyuCard { width: Fill height: Fit flow: Down padding: 6.0 spacing: 0.0
                                sent_gift_0 := LiyuBoxRow { }
                                sent_gift_1 := LiyuBoxRow { }
                                sent_gift_2 := LiyuBoxRow { }
                                sent_gift_3 := LiyuBoxRow { }
                                sent_gift_4 := LiyuBoxRow { }
                                sent_gift_5 := LiyuBoxRow { }
                                sent_gift_6 := LiyuBoxRow { }
                                sent_gift_7 := LiyuBoxRow { }
                                sent_gift_8 := LiyuBoxRow { }
                                sent_gift_9 := LiyuBoxRow { }
                                sent_gift_10 := LiyuBoxRow { }
                                sent_gift_11 := LiyuBoxRow { }
                                sent_gift_empty := LiyuEmpty { }
                            }
                            sent_gift_more := LiyuMuted { visible: false text: "" }
                        }
                        // ================= 钱包与流水（覆盖页）=================
                        page_wallet := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            wl_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 6.0
                                wl_l := LiyuMuted { text: "礼遇余额" }
                                wl_v := LiyuH1 { text: "¥0" }
                                wl_top := LiyuBtn { width: Fit text: "演示充值 ¥50" draw_icon +: { svg: crate_resource("self:resources/icons/wallet.svg") } }
                                wl_note := LiyuMuted { text: "余额只在礼遇内使用，暂不支持提现。送礼时可以优先抵扣。" }
                            }
                            wl_head := LiyuGroupHead { text: "流水（最近 12 笔）" }
                            wl_list := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 6.0
                                spacing: 0.0
                                l0 := LiyuLedgerRow { }
                                l1 := LiyuLedgerRow { }
                                l2 := LiyuLedgerRow { }
                                l3 := LiyuLedgerRow { }
                                l4 := LiyuLedgerRow { }
                                l5 := LiyuLedgerRow { }
                                l6 := LiyuLedgerRow { }
                                l7 := LiyuLedgerRow { }
                                l8 := LiyuLedgerRow { }
                                l9 := LiyuLedgerRow { }
                                l10 := LiyuLedgerRow { }
                                l11 := LiyuLedgerRow { }
                                wl_empty := LiyuEmpty {
                                    visible: false
                                    em_icon := LiyuIcon {
                                        icon_walk: Walk{ width: 28.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/wallet.svg") color: liyu.ink_ghost }
                                    }
                                }
                            }
                        }

                        // ================= 设置（覆盖页）=================


                        // ================= 账户资料（覆盖页）=================


                        // ================= 单件礼物订单 =================
                        page_direct_order := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            ca_status := LiyuBadgeBlue { text: "" }
                            ca_pick := LiyuCard {
                                width: Fill height: Fit flow: Down padding: 14.0 spacing: 10.0
                                ca_pick_title := LiyuH3 { text: "确认礼物" }
                                ca_friends_note := LiyuMuted { text: "输入手机号或邮箱即可送礼，无需对方先注册或加好友。" }
                                ca_recipient_label := LiyuInput { empty_text: "收礼人称呼（可选）" }
                                ca_recipient_value := LiyuInput { empty_text: "收件手机号或邮箱（国际号码带国家码）" }
                                ca_friends := View {
                                    width: Fill height: Fit flow: Right{wrap: true} wrap_spacing: 8.0 spacing: 8.0
                                    ca_f0 := LiyuChip { text: "" }
                                    ca_f1 := LiyuChip { text: "" }
                                    ca_f2 := LiyuChip { text: "" }
                                    ca_f3 := LiyuChip { text: "" }
                                }
                                ca_choice_paging := View { width: Fill height: Fit flow: Right spacing: 8.0 ca_choice_prev := LiyuBtnSm { width: Fit text: "上一组联系人" } ca_choice_next := LiyuBtnSm { width: Fit text: "下一组联系人" } }
                                ca_checkout := LiyuBtnPrimary { width: Fit text: "生成测试订单" }
                            }
                            ca_order := LiyuCard {
                                visible: false width: Fill height: Fit flow: Down padding: 14.0 spacing: 10.0
                                ca_order_text := LiyuBody { text: "" }
                                ca_pay := LiyuBtnPrimary { width: Fit text: "测试支付" }
                            }
                            ca_error := LiyuBad { visible: false text: "" }
                        }

                        // ================= 商品详情（覆盖页）=================
                        //
                        // 大图 + 名字价格 + 两个动作；宽屏左图右字，手机上下排（apply_shaping 里切）。
                        // 从「帮 TA 挑」进来时多一张心愿卡，说这件为什么对得上 TA 的心愿。
                        page_product := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            pd_top := View {
                                width: Fill height: Fit
                                flow: Right
                                spacing: 24.0
                                pd_img := LiyuThumb { width: 300 height: 300 }
                                pd_info := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    pd_cat := LiyuBadgeBlue { text: "" draw_text +: { text_style +: { font_size: 12.0 } } }
                                    pd_name := LiyuH1 { text: "" }
                                    pd_brand := LiyuMuted { text: "" }
                                    pd_price := Label {
                                        flow: Right{wrap: true}
                                        width: Fit
                                        margin: Inset{top: 4.0}
                                        text: ""
                                        draw_text +: { color: liyu.warm text_style +: { font_size: 26.0 } }
                                    }
                                    pd_tags := LiyuMuted { text: "" draw_text +: { color: liyu.ink_3 } }
                                    pd_server_note := LiyuMuted { text: "" }
                                    pd_ctx := LiyuCard2 {
                                        visible: false
                                        width: Fill height: Fit
                                        flow: Down
                                        spacing: 4.0
                                        pd_ctx_t := LiyuWarmText { text: "" draw_text +: { text_style +: { font_size: 14.0 } } }
                                        pd_ctx_r := LiyuMuted { text: "" }
                                    }
                                    pd_btns := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true row_align: RowAlign.Center}
                                        margin: Inset{top: 6.0}
                                        wrap_spacing: 8.0
                                        spacing: 10.0
                                        pd_send := LiyuBtnPrimary {
                                            width: Fit
                                            text: "送给 TA"
                                            draw_icon +: { svg: crate_resource("self:resources/icons/nav-gift.svg") }
                                        }
                                        pd_direct := LiyuBtn { width: Fit text: "按联系方式送礼" }
                                        pd_addwish := LiyuBtn {
                                            width: Fit
                                            text: "加到我的心愿单"
                                            draw_icon +: { svg: crate_resource("self:resources/icons/plus.svg") }
                                        }
                                    }
                                }
                            }
                            pd_desc_card := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 6.0
                                pd_desc_h := LiyuH3 { text: "商品介绍" }
                                pd_desc := LiyuBody { text: "" }
                                pd_kvs := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    margin: Inset{top: 6.0}
                                    pd_kv_kind := LiyuKV { }
                                    pd_kv_spec := LiyuKV { }
                                    pd_kv_ship := LiyuKV { }
                                }
                            }
                            pd_notes := LiyuCard2 {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 6.0
                                pd_notes_t := LiyuH3 { text: "送礼须知" }
                                pd_notes_b := LiyuMuted { text: "· 礼卡上只有玩法和线索，不出现商品名、价格和你的名字\n· TA 不喜欢可以换一件，或者折成余额\n· 7 天没拆开，全额退回你的余额" }
                            }
                            pd_rel_h := LiyuGroupHead { text: "同类还有" }
                            pd_rel := View {
                                width: Fill height: Fit
                                flow: Right{wrap: true}
                                wrap_spacing: 12.0
                                spacing: 12.0
                                pr0 := LiyuProductCard { }
                                pr1 := LiyuProductCard { }
                                pr2 := LiyuProductCard { }
                                pr3 := LiyuProductCard { }
                            }
                        }

                        // ================= 结算（覆盖页）=================
                        //
                        // 送礼页只管「送什么、怎么玩」，钱在这一页算清楚：订单、余额抵扣、付款方式。
                        // 付完原地变成成功页，再去分享礼卡。
                        page_checkout := View {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 10.0
                            co_scroll := LiyuScrollY {
                                width: Fill height: Fill
                                flow: Down
                                spacing: 14.0
                                co_paid := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    co_ok_icon := LiyuIcon {
                                        icon_walk: Walk{ width: 40.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/check-circle.svg") color: liyu.good }
                                    }
                                    co_ok_t := LiyuH2 { text: "支付成功" }
                                    co_ok_b := LiyuBody { text: "" }
                                }
                                co_order := LiyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 12.0
                                    co_order_h := LiyuGroupHead { margin: 0.0 text: "订单" }
                                    co_item := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 14.0
                                        co_img := LiyuThumb { width: 72 height: 72 }
                                        co_col := View {
                                            width: Fill height: Fit
                                            flow: Down
                                            spacing: 3.0
                                            co_name := LiyuH3 { text: "" }
                                            co_sub := LiyuMuted { text: "" }
                                            co_price := Label {
                                                flow: Right{wrap: true}
                                                width: Fit
                                                padding: Inset{left: 3.0, right: 3.0, top: 2.0, bottom: 0.0}
                                                text: ""
                                                draw_text +: { color: liyu.warm text_style +: { font_size: 16.0 } }
                                            }
                                        }
                                    }
                                    co_kvs := View {
                                        width: Fill height: Fit
                                        flow: Down
                                        co_to := LiyuKV { }
                                        co_play := LiyuKV { }
                                        co_pact := LiyuKV { }
                                        co_when := LiyuKV { }
                                        co_wish := LiyuKV { }
                                    }
                                }
                                co_pay := LiyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    co_pay_h := LiyuGroupHead { margin: 0.0 text: "付款" }
                                    co_bal_row := LiyuSwitchRow { margin: Inset{left: -12.0, right: -12.0} }
                                    co_lines := View {
                                        width: Fill height: Fit
                                        flow: Down
                                        co_l1 := LiyuKV { }
                                        co_l2 := LiyuKV { }
                                        co_l3 := LiyuKV { }
                                    }
                                    co_pm := View {
                                        width: Fill height: Fit
                                        flow: Down
                                        spacing: 8.0
                                        co_pm_h := LiyuMuted { text: "付款方式" }
                                        co_pm_row := View {
                                            width: Fill height: Fit
                                            flow: Right{wrap: true}
                                            wrap_spacing: 8.0
                                            spacing: 8.0
                                            pm0 := LiyuChip { text: "微信支付" }
                                            pm1 := LiyuChip { text: "支付宝" }
                                            pm2 := LiyuChip { text: "银行卡" }
                                        }
                                    }
                                    co_demo := LiyuMuted { text: "演示环境：不会真的扣款。抵扣和付款都记进「钱包与流水」。" draw_text +: { color: liyu.ink_3 } }
                                }
                            }
                            co_bar := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 6.0
                                padding: Inset{top: 8.0}
                                co_err := LiyuBad { visible: false text: "" }
                                co_go := LiyuBtnPrimary {
                                    width: Fill
                                    text: "确认支付"
                                    draw_icon +: { svg: crate_resource("self:resources/icons/lock.svg") }
                                }
                                co_after := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Right{wrap: true row_align: RowAlign.Center}
                                    wrap_spacing: 8.0
                                    spacing: 10.0
                                    co_card := LiyuBtnPrimary {
                                        width: Fit
                                        text: "去分享礼卡"
                                        draw_icon +: { svg: crate_resource("self:resources/icons/share.svg") }
                                    }
                                    co_home := LiyuBtn { width: Fit text: "继续逛逛" }
                                }
                            }
                        }

                        // ================= 我的心愿单（覆盖页）=================
                        page_wishes := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 12.0
                            mw_new := LiyuBtnPrimary {
                                width: Fit
                                text: "发布新的心愿单"
                                draw_icon +: { svg: crate_resource("self:resources/icons/plus.svg") }
                            }
                            mw0 := LiyuWishCard { }
                            mw1 := LiyuWishCard { }
                            mw2 := LiyuWishCard { }
                            mw3 := LiyuWishCard { }
                            mw_empty := LiyuEmpty {
                                visible: false
                                em_icon := LiyuIcon {
                                    icon_walk: Walk{ width: 28.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/nav-gift.svg") color: liyu.ink_ghost }
                                }
                            }
                            mw_note := LiyuMuted {
                                text: "心愿单只给熟人看。谁认领了哪一件，揭晓前你都不知道 —— 只知道「已有人送」。"
                            }
                        }

                        // ================= 一张心愿单（覆盖页）=================
                        //
                        // 好友的：每件一个「送这件 / 帮 TA 挑」；有人送了的调暗、按钮收起。
                        // 自己的：看进度、编辑、分享、提前结束；另有一张演示卡替好友来认领。
                        page_wish := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            wd_head := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 10.0
                                wd_top := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 12.0
                                    wd_face := RoundedView {
                                        width: 44 height: 44
                                        flow: Down
                                        align: Align{x: 0.5, y: 0.5}
                                        draw_bg +: { color: liyu.face border_radius: r.card }
                                        wd_initial := Label {
                                            flow: Right{wrap: true}
                                            text: ""
                                            draw_text +: { color: liyu.warm text_style +: { font_size: 16.0 } }
                                        }
                                    }
                                    wd_col := View {
                                        width: Fill height: Fit
                                        flow: Down
                                        spacing: 2.0
                                        wd_title := LiyuH2 { text: "" }
                                        wd_when := LiyuMuted { text: "" }
                                    }
                                    wd_badge := LiyuBadgeWarm { text: "" draw_text +: { text_style +: { font_size: 12.0 } } }
                                }
                                wd_note := LiyuBody { text: "" }
                                wd_prog := LiyuMuted { text: "" }
                                wd_track := RoundedView {
                                    width: Fill height: 6
                                    flow: Right
                                    draw_bg +: { color: liyu.track_off border_radius: r.tick }
                                    wd_fill := RoundedView {
                                        width: 0 height: 6
                                        draw_bg +: { color: liyu.warm border_radius: r.tick }
                                    }
                                }
                                wd_meta := LiyuMuted { text: "" draw_text +: { color: liyu.ink_3 } }
                            }
                            wd_tip := LiyuCard2 {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 4.0
                                wd_tip_t := LiyuMuted { text: "" }
                            }
                            wd_list := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 6.0
                                spacing: 0.0
                                wr0 := LiyuWishItemRow { }
                                wr1 := LiyuWishItemRow { }
                                wr2 := LiyuWishItemRow { }
                                wr3 := LiyuWishItemRow { }
                                wr4 := LiyuWishItemRow { }
                                wr5 := LiyuWishItemRow { }
                                wr6 := LiyuWishItemRow { }
                                wr7 := LiyuWishItemRow { }
                            }
                            wd_owner := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                spacing: 10.0
                                wd_acts := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    wd_edit := LiyuBtn { width: Fit text: "编辑" draw_icon +: { svg: crate_resource("self:resources/icons/edit.svg") } }
                                    wd_copy := LiyuBtn { width: Fit text: "复制分享链接" draw_icon +: { svg: crate_resource("self:resources/icons/share.svg") } }
                                    wd_close := LiyuBtn { width: Fit text: "提前结束" draw_icon +: { svg: crate_resource("self:resources/icons/lock.svg") } }
                                    wd_del := LiyuBtnDanger { width: Fit text: "删除" draw_icon +: { svg: crate_resource("self:resources/icons/trash.svg") } }
                                }
                                wd_confirm := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 8.0
                                    wd_ctext := LiyuBad { text: "" }
                                    wd_crow := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true}
                                        wrap_spacing: 8.0
                                        spacing: 8.0
                                        wd_cno := LiyuBtn { width: Fit text: "再想想" }
                                        wd_cyes := LiyuBtnDanger { width: Fit text: "确认" }
                                    }
                                }
                            }
                        }

                        // ================= 发布 / 编辑心愿单（覆盖页）=================
                        //
                        // 四段：日子 / 标题 / 想要什么 / 给谁看。「想要什么」两种放法：挑一件具体的
                        // （去目录里点），或者说个大概（品类 + 预算 + 一句要求），就地展开。
                        page_wish_edit := View {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 10.0
                            we_scroll := LiyuScrollY {
                                width: Fill height: Fill
                                flow: Down
                                spacing: 12.0
                                we_s1 := LiyuH3 { text: "1 · 什么日子" }
                                we_occ := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                        oc0 := LiyuChip { text: "生日" }
                                        oc1 := LiyuChip { text: "结婚" }
                                        oc2 := LiyuChip { text: "乔迁" }
                                        oc3 := LiyuChip { text: "毕业" }
                                        oc4 := LiyuChip { text: "宝宝" }
                                        oc5 := LiyuChip { text: "其他" }
                                }
                                we_date := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true row_align: RowAlign.Center}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    we_date_l := Label {
                                        flow: Right{wrap: true}
                                        width: Fit
                                        margin: Inset{right: 6.0}
                                        text: ""
                                        draw_text +: { color: liyu.ink text_style +: { font_size: 15.0 } }
                                    }
                                    // 三个按钮一起折行，窄屏上不会一个挂在日期后面、两个掉到下一行。
                                    we_dbtns := View {
                                        width: Fit height: Fit
                                        flow: Right
                                        spacing: 8.0
                                        we_dm := LiyuBtnSm { width: Fit text: "早一天" }
                                        we_dp := LiyuBtnSm { width: Fit text: "晚一天" }
                                        we_dw := LiyuBtnSm { width: Fit text: "晚一周" }
                                    }
                                }
                                we_s2 := LiyuH3 { margin: Inset{top: 6.0} text: "2 · 标题和想说的话" }
                                we_title := LiyuInput { empty_text: "" }
                                we_note := LiyuInput { empty_text: "想对朋友们说的话（可不写），最多 40 字" }
                                we_s3 := LiyuH3 { margin: Inset{top: 6.0} text: "3 · 想要什么" }
                                we_items := LiyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 6.0
                                    spacing: 0.0
                                    ei0 := LiyuWishItemRow { }
                                    ei1 := LiyuWishItemRow { }
                                    ei2 := LiyuWishItemRow { }
                                    ei3 := LiyuWishItemRow { }
                                    ei4 := LiyuWishItemRow { }
                                    ei5 := LiyuWishItemRow { }
                                    ei6 := LiyuWishItemRow { }
                                    ei7 := LiyuWishItemRow { }
                                    we_items_empty := LiyuMuted {
                                        margin: Inset{left: 12.0, right: 12.0, top: 8.0, bottom: 8.0}
                                        text: "还没放东西。挑一件具体的，或只说个大概 —— 如「一台电视，55 寸以上」，送礼的人会看到符合要求的商品。"
                                    }
                                }
                                we_add := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    we_add_exact := LiyuBtn { width: Fit text: "挑一件具体的" draw_icon +: { svg: crate_resource("self:resources/icons/search.svg") } }
                                    we_add_vague := LiyuBtn { width: Fit text: "说个大概" draw_icon +: { svg: crate_resource("self:resources/icons/sparkle.svg") } }
                                }
                                we_vague := LiyuCard2 {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 10.0
                                    we_v_k := LiyuMuted { text: "想要个什么？" }
                                    we_kinds := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true}
                                        wrap_spacing: 8.0
                                        spacing: 8.0
                                            wk0 := LiyuChip { text: "电视" }
                                            wk1 := LiyuChip { text: "耳机" }
                                            wk2 := LiyuChip { text: "音箱" }
                                            wk3 := LiyuChip { text: "投影仪" }
                                            wk4 := LiyuChip { text: "空气炸锅" }
                                            wk5 := LiyuChip { text: "扫地机" }
                                            wk6 := LiyuChip { text: "床品" }
                                            wk7 := LiyuChip { text: "餐具" }
                                            wk8 := LiyuChip { text: "灯" }
                                            wk9 := LiyuChip { text: "绿植" }
                                            wk10 := LiyuChip { text: "鲜花" }
                                            wk11 := LiyuChip { text: "蛋糕" }
                                            wk12 := LiyuChip { text: "咖啡" }
                                            wk13 := LiyuChip { text: "盲盒" }
                                            wk14 := LiyuChip { text: "玩具" }
                                            wk15 := LiyuChip { text: "推车" }
                                    }
                                    we_v_b := LiyuMuted { text: "预算" }
                                    we_budgets := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true}
                                        wrap_spacing: 8.0
                                        spacing: 8.0
                                            bd0 := LiyuChip { text: "不限" }
                                            bd1 := LiyuChip { text: "¥100 以内" }
                                            bd2 := LiyuChip { text: "¥300 以内" }
                                            bd3 := LiyuChip { text: "¥500 以内" }
                                            bd4 := LiyuChip { text: "¥1000 以内" }
                                            bd5 := LiyuChip { text: "¥3000 以内" }
                                            bd6 := LiyuChip { text: "¥5000 以内" }
                                    }
                                    we_v_w := LiyuMuted { text: "有什么要求（可不写）" }
                                    we_wants := LiyuInput { empty_text: "比如：55 寸以上、4K" }
                                    we_v_prev := LiyuMuted { text: "" draw_text +: { color: liyu.blue } }
                                    we_v_strip := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        spacing: 8.0
                                        vp0 := LiyuThumb { width: 48 height: 48 }
                                        vp1 := LiyuThumb { width: 48 height: 48 }
                                        vp2 := LiyuThumb { width: 48 height: 48 }
                                        vp3 := LiyuThumb { width: 48 height: 48 }
                                        vp4 := LiyuThumb { width: 48 height: 48 }
                                    }
                                    we_v_row := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true}
                                        wrap_spacing: 8.0
                                        spacing: 8.0
                                        we_v_ok := LiyuBtnPrimarySm { width: Fit text: "放进心愿单" }
                                        we_v_no := LiyuBtnSm { width: Fit text: "算了" }
                                    }
                                }
                                we_s4 := LiyuH3 { margin: Inset{top: 6.0} text: "4 · 给谁看" }
                                we_aud := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    aa0 := LiyuChip { text: "所有熟人" }
                                    au0 := LiyuChip { text: "" }
                                    au1 := LiyuChip { text: "" }
                                    au2 := LiyuChip { text: "" }
                                    au3 := LiyuChip { text: "" }
                                    au4 := LiyuChip { text: "" }
                                    au5 := LiyuChip { text: "" }
                                    au6 := LiyuChip { text: "" }
                                    au7 := LiyuChip { text: "" }
                                    au8 := LiyuChip { text: "" }
                                    au9 := LiyuChip { text: "" }
                                }
                                we_aud_n := LiyuMuted { text: "" }
                            }
                            we_bar := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 6.0
                                padding: Inset{top: 8.0}
                                we_err := LiyuBad { visible: false text: "" }
                                we_go := LiyuBtnPrimary {
                                    width: Fill
                                    text: "发布心愿单"
                                    draw_icon +: { svg: crate_resource("self:resources/icons/sparkle.svg") }
                                }
                            }
                        }

                        // ================= 挑一件（覆盖页）=================
                        //
                        // 两种用法：替好友「说了个大概」的心愿挑一件（只列符合的，附理由），
                        // 或者给自己的心愿单挑一件具体的（整份目录，按品类筛）。
                        page_wish_pick := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            wp_head := LiyuCard2 {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 4.0
                                wp_t := LiyuH3 { text: "" }
                                wp_s := LiyuMuted { text: "" }
                            }
                            wp_chips := View {
                                width: Fill height: Fit
                                flow: Right{wrap: true}
                                wrap_spacing: 8.0
                                spacing: 8.0
                                wq0 := LiyuChip { text: "全部" }
                                wq1 := LiyuChip { text: "咖啡茶饮" }
                                wq2 := LiyuChip { text: "电影演出" }
                                wq3 := LiyuChip { text: "潮流小物" }
                                wq4 := LiyuChip { text: "盲盒" }
                                wq5 := LiyuChip { text: "甜点鲜花" }
                                wq6 := LiyuChip { text: "数码家电" }
                                wq7 := LiyuChip { text: "家居生活" }
                                wq8 := LiyuChip { text: "母婴亲子" }
                            }
                            wp_grid := View {
                                width: Fill height: Fit
                                flow: Right{wrap: true}
                                wrap_spacing: 12.0
                                spacing: 12.0
                                wp0 := LiyuProductCard { }
                                wp1 := LiyuProductCard { }
                                wp2 := LiyuProductCard { }
                                wp3 := LiyuProductCard { }
                                wp4 := LiyuProductCard { }
                                wp5 := LiyuProductCard { }
                                wp6 := LiyuProductCard { }
                                wp7 := LiyuProductCard { }
                                wp8 := LiyuProductCard { }
                                wp9 := LiyuProductCard { }
                                wp10 := LiyuProductCard { }
                                wp11 := LiyuProductCard { }
                                wp12 := LiyuProductCard { }
                                wp13 := LiyuProductCard { }
                                wp14 := LiyuProductCard { }
                                wp15 := LiyuProductCard { }
                                wp16 := LiyuProductCard { }
                                wp17 := LiyuProductCard { }
                                wp18 := LiyuProductCard { }
                                wp19 := LiyuProductCard { }
                                wp20 := LiyuProductCard { }
                                wp21 := LiyuProductCard { }
                                wp22 := LiyuProductCard { }
                                wp23 := LiyuProductCard { }
                                wp24 := LiyuProductCard { }
                                wp25 := LiyuProductCard { }
                                wp26 := LiyuProductCard { }
                                wp27 := LiyuProductCard { }
                                wp28 := LiyuProductCard { }
                                wp29 := LiyuProductCard { }
                                wp30 := LiyuProductCard { }
                                wp31 := LiyuProductCard { }
                                wp32 := LiyuProductCard { }
                            }
                            wp_note := LiyuMuted { text: "" }
                        }

                            }
                        }

                        // ================= 认证闸（首次启动 / 登出 / 401 后的独立界面）=================
                        //
                        // 未认证时盖住整个应用：不是覆盖页、不在 Tab 里，登录 / 注册互切，
                        // 登录成功后进入应用。过了这一关才可能看到开场三屏或主界面。
                        page_auth := LiyuScrollY {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            align: Align{x: 0.5, y: 0.5}
                            spacing: 14.0
                            au_card := LiyuCard {
                                width: 420 height: Fit
                                flow: Down
                                padding: 24.0
                                spacing: 12.0
                                au_title := LiyuH1 { text: "欢迎来到礼遇" }
                                au_sub := LiyuMuted { text: "猜得到的心意。登录或注册后继续。" }
                                au_server_err := LiyuBad { visible: false text: "" }
                                au_retry := LiyuBtn { visible: false width: Fit text: "重试连接" }
                                au_identifier := LiyuInput { empty_text: "邮箱或手机号作为账号标识" }
                                au_password := LiyuInput { empty_text: "密码" is_password: true }
                                au_code_wrap := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down spacing: 8.0
                                    au_request_code := LiyuBtnSm { width: Fit text: "获取验证码" }
                                    au_code := LiyuInput { empty_text: "注册验证码" }
                                }
                                au_err := LiyuBad { visible: false text: "" }
                                au_actions := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    au_submit := LiyuBtnPrimary { width: Fit text: "登录" }
                                    au_switch := LiyuLink { text: "没有账号？去注册" }
                                }
                                au_test_note := LiyuMuted { visible: false text: "当前为测试通道，获取验证码后会显示测试验证码。" }
                            }
                        }

                        // ================= 开场三屏 =================
                        page_intro := View {
                            visible: false
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            // 固定高度：最后一屏收起「跳过」后，进度条不该跟着往上跳。
                            in_top := View {
                                width: Fill height: 40
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 6.0
                                in_d0 := RoundedView {
                                    width: 22 height: 4
                                    draw_bg +: { color: liyu.blue border_radius: r.tick }
                                }
                                in_d1 := RoundedView {
                                    width: 22 height: 4
                                    draw_bg +: { color: liyu.line_soft border_radius: r.tick }
                                }
                                in_d2 := RoundedView {
                                    width: 22 height: 4
                                    draw_bg +: { color: liyu.line_soft border_radius: r.tick }
                                }
                                in_gap := View { width: Fill height: Fit }
                                in_skip := LiyuLink { text: "跳过" }
                            }
                            in_mid := LiyuScrollY {
                                width: Fill height: Fill
                                flow: Down
                                spacing: 14.0
                                // 三枚图标写死、按步显隐：script_apply_eval! 里没有 crate_resource。
                                in_icons := View {
                                    width: Fit height: Fit
                                    flow: Right
                                    in_ic0 := View {
                                        width: Fit height: Fit
                                        in_i := LiyuIcon {
                                            icon_walk: Walk{ width: 40.0 height: Fit }
                                            draw_icon +: { svg: crate_resource("self:resources/icons/nav-gift.svg") color: liyu.warm }
                                        }
                                    }
                                    in_ic1 := View {
                                        visible: false
                                        width: Fit height: Fit
                                        in_i := LiyuIcon {
                                            icon_walk: Walk{ width: 40.0 height: Fit }
                                            draw_icon +: { svg: crate_resource("self:resources/icons/nav-pact.svg") color: liyu.warm }
                                        }
                                    }
                                    in_ic2 := View {
                                        visible: false
                                        width: Fit height: Fit
                                        in_i := LiyuIcon {
                                            icon_walk: Walk{ width: 40.0 height: Fit }
                                            draw_icon +: { svg: crate_resource("self:resources/icons/wallet.svg") color: liyu.good }
                                        }
                                    }
                                }
                                in_title := Label {
                                    flow: Right{wrap: true}
                                    width: Fill
                                    text: ""
                                    draw_text +: { wrap: Words color: liyu.ink text_style +: { font_size: 24.0 line_spacing: 1.35 } }
                                }
                                in_body := Label {
                                    flow: Right{wrap: true}
                                    width: Fill
                                    text: ""
                                    draw_text +: { wrap: Words color: liyu.ink_2 text_style +: { font_size: 15.0 line_spacing: 1.35 } }
                                }
                                in_points := LiyuCard {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 16.0
                                    spacing: 10.0
                                    ip0 := LiyuGood { text: "" }
                                    ip1 := LiyuGood { text: "" }
                                    ip2 := LiyuGood { text: "" }
                                    ip3 := LiyuGood { text: "" }
                                }
                            }
                            in_bar := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 10.0
                                in_back := LiyuBtn { visible: false text: "上一步" }
                                in_gap2 := View { width: Fill height: Fit }
                                in_next := LiyuBtnPrimary { text: "下一步" }
                            }
                        }
                        }

                        toast_layer := View {
                            width: Fill height: Fill
                            flow: Down
                            align: Align{x: 0.5, y: 1.0}
                            padding: Inset{left: 16.0, right: 16.0, bottom: 2.0}
                            toast := LiyuToast { }
                        }
                    }

                    // ---- 右列辅助栏（桌面宽屏）----
                    aside := LiyuScrollY {
                        width: 300 height: Fill
                        flow: Down
                        spacing: 14.0

                        aside_gift := View {
                            width: Fill height: Fit
                            flow: Down
                            spacing: 14.0
                            ag1 := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 8.0
                                ag1_t := LiyuH3 { text: "怎么玩" }
                                ag1_b := LiyuMuted { text: "1 挑一件小礼物\n2 出一道只有 TA 答得上的题，可附一个小契约\n3 把礼卡发给 TA，等 TA 来拆" }
                            }
                            ag2 := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 6.0
                                ag2_t := LiyuMuted { text: "礼遇余额" }
                                ag2_v := LiyuH2 { text: "¥0" }
                                ag2_b := LiyuMuted { text: "结算时可优先抵扣；不够的部分模拟支付。" }
                            }
                            ag3 := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 8.0
                                ag3_t := LiyuH3 { text: "心愿单" }
                                ag3_b := LiyuMuted { text: "生日、结婚、搬新家…… 列出想要的，熟人就知道送什么。可写具体哪一件，也可只说大概，如「一台电视，55 寸以上」。" }
                                ag3_go := LiyuBtnSm { width: Fit text: "我的心愿单" }
                            }
                        }
                        aside_box := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 14.0
                            ab1 := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 8.0
                                ab1_t := LiyuH3 { text: "礼盒一览" }
                                ab1_b := LiyuMuted { text: "" }
                            }
                            ab2 := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 8.0
                                ab2_t := LiyuH3 { text: "7 天没拆会自动退回" }
                                ab2_b := LiyuMuted { text: "收到的礼物 7 天内不揭晓，全额退回送礼人；你送出的也一样，钱回到你的余额。" }
                            }
                        }
                        aside_pact := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 14.0
                            ap1 := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 8.0
                                ap1_t := LiyuH3 { text: "契约不是合同" }
                                ap1_b := LiyuMuted { text: "· 只写轻约定，不涉及钱\n· 期限 7 天，逾期没有惩罚\n· 答应你的那一方，你可以「免了吧」" }
                            }
                        }
                        aside_contacts := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 14.0
                            ac1 := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 8.0
                                ac1_t := LiyuH3 { text: "熟人只在本机" }
                                ac1_b := LiyuMuted { text: "熟人是送礼的快捷选项，也是「猜我是谁」的候选名字来源。删掉一位不影响已送出的礼物。" }
                            }
                        }
                        aside_me := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 14.0
                            am1 := LiyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 8.0
                                am1_t := LiyuH3 { text: "余额从哪来" }
                                am1_b := LiyuMuted { text: "折现（手续费 8%）、换购退差（手续费 5%）、撤回和过期退款都进余额。余额只在礼遇内使用。" }
                            }
                        }
                    }
                }

                // ---- 手机底部导航 ----
                tabbar := RoundedView {
                    visible: false
                    width: Fill height: 66
                    flow: Right
                    align: Align{x: 0.5, y: 0.5}
                    padding: Inset{left: 6.0, right: 6.0, top: 4.0, bottom: 4.0}
                    spacing: 4.0
                    draw_bg +: {
                        color: liyu.bg_chrome
                        border_color: liyu.line
                        border_size: 1.0
                        border_radius: 0.0
                    }
                    tab_gift := LiyuNavTab { text: "挑礼" draw_icon +: { svg: crate_resource("self:resources/icons/nav-gift.svg") } }
                    tab_box := LiyuNavTab { text: "礼盒" draw_icon +: { svg: crate_resource("self:resources/icons/nav-box.svg") } }
                    tab_pact := LiyuNavTab { text: "契约" draw_icon +: { svg: crate_resource("self:resources/icons/nav-pact.svg") } }
                    tab_contacts := LiyuNavTab { text: "熟人" draw_icon +: { svg: crate_resource("self:resources/icons/nav-contacts.svg") } }
                    tab_me := LiyuNavTab { text: "我" draw_icon +: { svg: crate_resource("self:resources/icons/nav-me.svg") } }
                }
            }

            // ---- 熟人页「+」的导入菜单浮层 ----
            ct_menu_layer := View {
                visible: false
                width: Fill height: Fill
                flow: Overlay
                // 透明全屏命中区：点菜单以外任何地方收起。
                ct_hit := mod.widgets.ButtonFlat {
                    width: Fill height: Fill
                    text: ""
                    margin: 0.0
                    padding: 0.0
                    draw_bg +: {
                        border_size: 0.0
                        border_radius: 0.0
                        color: #0000
                        color_hover: #0000
                        color_down: #0000
                        color_focus: #0000
                    }
                }
                ct_menu_pos := View {
                    width: Fill height: Fill
                    flow: Overlay
                    align: Align{x: 1.0, y: 0.0}
                    padding: Inset{top: 58.0, right: 12.0}
                    ct_menu := LiyuMenu {
                        im_local := LiyuMenuItem {
                            text: "从本机导入"
                            draw_icon +: { svg: crate_resource("self:resources/icons/nav-contacts.svg") }
                        }
                        im_file := LiyuMenuItem {
                            text: "从文件导入"
                            draw_icon +: { svg: crate_resource("self:resources/icons/download.svg") }
                        }
                    }
                }
            }
        }
    }
}


// ---------------------------------------------------------------------------
// 文案与控件 id 表
// ---------------------------------------------------------------------------

/// 开场三屏（liyu/docs/03-pages.md 10 节）。第一屏先说「这是送礼，不是转账」，
/// 第二屏说悬念，第三屏说「不喜欢也没关系」—— 顺序就是一份礼物的一生。
const INTRO: [(&str, &str); 3] = [
    (
        "送一份猜得到的心意",
        "挑一件小礼物，出一道只有 TA 答得上的题。礼卡上没有礼物名、没有价格，也没有你的名字。",
    ),
    (
        "拆开之前，全是悬念",
        "TA 可以猜你是谁、回答只有你们知道的问题，或者对上暗号。猜错也不怕，机会用完礼物照样拆开。",
    ),
    (
        "不合心意？换购或折现",
        "TA 可以开心收下、答应你附上的小契约；也可以换一件，或折成余额，再用余额给你回一份「反击礼物」。",
    ),
];

/// 第 3 屏底部的规则要点（04-rules 的缩写）。
const INTRO_POINTS: [&str; 4] = [
    "礼卡上只有玩法和线索",
    "没拆开之前，TA 不知道是你",
    "7 天没拆，全额退回",
    "余额只在礼遇内使用，不能提现",
];

const INTRO_DOTS: [LiveId; 3] = [live_id!(in_d0), live_id!(in_d1), live_id!(in_d2)];
const INTRO_ICONS: [LiveId; 3] = [live_id!(in_ic0), live_id!(in_ic1), live_id!(in_ic2)];
const INTRO_POINT_ROWS: [LiveId; 4] = [live_id!(ip0), live_id!(ip1), live_id!(ip2), live_id!(ip3)];

const TABS: [LiveId; 5] = [
    live_id!(tab_gift),
    live_id!(tab_box),
    live_id!(tab_pact),
    live_id!(tab_contacts),
    live_id!(tab_me),
];
const TAB_TITLES: [&str; 5] = ["挑礼", "礼盒", "契约", "熟人", "我"];
const PAGES: [LiveId; 5] = [
    live_id!(page_gift),
    live_id!(page_box),
    live_id!(page_pact),
    live_id!(page_contacts),
    live_id!(page_me),
];
const ASIDES: [LiveId; 5] = [
    live_id!(aside_gift),
    live_id!(aside_box),
    live_id!(aside_pact),
    live_id!(aside_contacts),
    live_id!(aside_me),
];

/// 0 = 全部，其余是 Category::ALL 的下标 + 1。
const CAT_CHIPS: [LiveId; 9] = [
    live_id!(k0), live_id!(k1), live_id!(k2), live_id!(k3), live_id!(k4), live_id!(k5),
    live_id!(k6), live_id!(k7), live_id!(k8),
];
const GIFT_SEGS: [LiveId; 2] = [live_id!(gs_shop), live_id!(gs_wish)];
/// 挑礼宫格：目录里一件一张卡。
const GRID_CARDS: [LiveId; 33] = [
    live_id!(pc0), live_id!(pc1), live_id!(pc2), live_id!(pc3), live_id!(pc4), live_id!(pc5),
    live_id!(pc6), live_id!(pc7), live_id!(pc8), live_id!(pc9), live_id!(pc10), live_id!(pc11),
    live_id!(pc12), live_id!(pc13), live_id!(pc14), live_id!(pc15), live_id!(pc16), live_id!(pc17),
    live_id!(pc18), live_id!(pc19), live_id!(pc20), live_id!(pc21), live_id!(pc22), live_id!(pc23),
    live_id!(pc24), live_id!(pc25), live_id!(pc26), live_id!(pc27), live_id!(pc28), live_id!(pc29),
    live_id!(pc30), live_id!(pc31), live_id!(pc32),
];
const PICK_ROWS: [LiveId; 12] = [
    live_id!(q0), live_id!(q1), live_id!(q2), live_id!(q3), live_id!(q4), live_id!(q5),
    live_id!(q6), live_id!(q7), live_id!(q8), live_id!(q9), live_id!(q10), live_id!(q11),
];
const BOX_ROWS: [LiveId; 12] = [
    live_id!(b0), live_id!(b1), live_id!(b2), live_id!(b3), live_id!(b4), live_id!(b5),
    live_id!(b6), live_id!(b7), live_id!(b8), live_id!(b9), live_id!(b10), live_id!(b11),
];
const BOX_SEGS: [LiveId; 2] = [live_id!(bs_recv), live_id!(bs_sent)];
const PACT_ROWS: [LiveId; 8] = [
    live_id!(p0), live_id!(p1), live_id!(p2), live_id!(p3),
    live_id!(p4), live_id!(p5), live_id!(p6), live_id!(p7),
];
const PACT_SEGS: [LiveId; 2] = [live_id!(ps_mine), live_id!(ps_theirs)];
const CONTACT_ROWS: [LiveId; 10] = [
    live_id!(c0), live_id!(c1), live_id!(c2), live_id!(c3), live_id!(c4),
    live_id!(c5), live_id!(c6), live_id!(c7), live_id!(c8), live_id!(c9),
];
const LEDGER_ROWS: [LiveId; 12] = [
    live_id!(l0), live_id!(l1), live_id!(l2), live_id!(l3), live_id!(l4), live_id!(l5),
    live_id!(l6), live_id!(l7), live_id!(l8), live_id!(l9), live_id!(l10), live_id!(l11),
];
const STEP_ROWS: [LiveId; 5] = [live_id!(t0), live_id!(t1), live_id!(t2), live_id!(t3), live_id!(t4)];
const SWAP_ROWS: [LiveId; 11] = [
    live_id!(x0), live_id!(x1), live_id!(x2), live_id!(x3), live_id!(x4), live_id!(x5),
    live_id!(x6), live_id!(x7), live_id!(x8), live_id!(x9), live_id!(x10),
];
const SWAP_SEGS: [LiveId; 2] = [live_id!(ow_cash), live_id!(ow_swap)];
const PEER_CHIPS: [LiveId; 6] = [
    live_id!(sp0), live_id!(sp1), live_id!(sp2), live_id!(sp3), live_id!(sp4), live_id!(sp5),
];
/// 送礼页前五枚熟人芯片，第六枚固定是「先不指定」。
const PEER_SLOTS: usize = 5;
const UNLOCK_SEGS: [LiveId; 4] = [live_id!(un0), live_id!(un1), live_id!(un2), live_id!(un3)];
const PRESET_CHIPS: [LiveId; 4] = [live_id!(pp0), live_id!(pp1), live_id!(pp2), live_id!(pp3)];
const CAND_BTNS: [LiveId; 6] = [
    live_id!(cd0), live_id!(cd1), live_id!(cd2), live_id!(cd3), live_id!(cd4), live_id!(cd5),
];
const STYLE_SEGS: [LiveId; 2] = [live_id!(cs_warm), live_id!(cs_night)];
const THEME_SEGS: [LiveId; 2] = [live_id!(ap_dark), live_id!(ap_light)];

/// 拆礼页的五个阶段块。
const STAGE_VIEWS: [LiveId; 5] = [
    live_id!(op_decrypt),
    live_id!(op_reveal),
    live_id!(op_accept),
    live_id!(op_swap),
    live_id!(op_done),
];

// ---- 商品详情 / 结算 / 心愿单 ----
/// 商品详情底部「同类还有」。
const REL_CARDS: [LiveId; 4] = [
    live_id!(pr0), live_id!(pr1), live_id!(pr2), live_id!(pr3),
];
/// 挑礼页「熟人的心愿单」。
const WISH_CARDS: [LiveId; 6] = [
    live_id!(fw0), live_id!(fw1), live_id!(fw2), live_id!(fw3), live_id!(fw4), live_id!(fw5),
];
/// 「我的心愿单」。
const MY_WISH_CARDS: [LiveId; 4] = [
    live_id!(mw0), live_id!(mw1), live_id!(mw2), live_id!(mw3),
];
/// 心愿单详情的一件一行。
const WISH_ROWS: [LiveId; 8] = [
    live_id!(wr0), live_id!(wr1), live_id!(wr2), live_id!(wr3), live_id!(wr4), live_id!(wr5),
    live_id!(wr6), live_id!(wr7),
];
/// 发布 / 编辑页里已经放进去的几件。
const EDIT_ROWS: [LiveId; 8] = [
    live_id!(ei0), live_id!(ei1), live_id!(ei2), live_id!(ei3), live_id!(ei4), live_id!(ei5),
    live_id!(ei6), live_id!(ei7),
];
/// 心愿单卡片上的一排小图（LiyuWishCard 里）。
const WISH_THUMBS: [LiveId; 5] = [
    live_id!(wt0), live_id!(wt1), live_id!(wt2), live_id!(wt3), live_id!(wt4),
];
const OCC_CHIPS: [LiveId; 6] = [
    live_id!(oc0), live_id!(oc1), live_id!(oc2), live_id!(oc3), live_id!(oc4), live_id!(oc5),
];
const KIND_CHIPS: [LiveId; 16] = [
    live_id!(wk0), live_id!(wk1), live_id!(wk2), live_id!(wk3), live_id!(wk4), live_id!(wk5),
    live_id!(wk6), live_id!(wk7), live_id!(wk8), live_id!(wk9), live_id!(wk10), live_id!(wk11),
    live_id!(wk12), live_id!(wk13), live_id!(wk14), live_id!(wk15),
];
const BUDGET_CHIPS: [LiveId; 7] = [
    live_id!(bd0), live_id!(bd1), live_id!(bd2), live_id!(bd3), live_id!(bd4), live_id!(bd5),
    live_id!(bd6),
];
/// 「给谁看」的熟人芯片（前面还有一枚「所有熟人」aa0）。
const AUD_CHIPS: [LiveId; 10] = [
    live_id!(au0), live_id!(au1), live_id!(au2), live_id!(au3), live_id!(au4), live_id!(au5),
    live_id!(au6), live_id!(au7), live_id!(au8), live_id!(au9),
];
/// 「说个大概」时预览的几件候选。
const PREVIEW_THUMBS: [LiveId; 5] = [
    live_id!(vp0), live_id!(vp1), live_id!(vp2), live_id!(vp3), live_id!(vp4),
];
/// 「挑一件」页的宫格。
const PICK_CARDS: [LiveId; 33] = [
    live_id!(wp0), live_id!(wp1), live_id!(wp2), live_id!(wp3), live_id!(wp4), live_id!(wp5),
    live_id!(wp6), live_id!(wp7), live_id!(wp8), live_id!(wp9), live_id!(wp10), live_id!(wp11),
    live_id!(wp12), live_id!(wp13), live_id!(wp14), live_id!(wp15), live_id!(wp16), live_id!(wp17),
    live_id!(wp18), live_id!(wp19), live_id!(wp20), live_id!(wp21), live_id!(wp22), live_id!(wp23),
    live_id!(wp24), live_id!(wp25), live_id!(wp26), live_id!(wp27), live_id!(wp28), live_id!(wp29),
    live_id!(wp30), live_id!(wp31), live_id!(wp32),
];
const PICK_CHIPS: [LiveId; 9] = [
    live_id!(wq0), live_id!(wq1), live_id!(wq2), live_id!(wq3), live_id!(wq4), live_id!(wq5),
    live_id!(wq6), live_id!(wq7), live_id!(wq8),
];
const DELIVER_SEGS: [LiveId; 2] = [live_id!(dv0), live_id!(dv1)];
const PAY_CHIPS: [LiveId; 3] = [
    live_id!(pm0), live_id!(pm1), live_id!(pm2),
];

/// 商品图，下标和 `CATALOG` 一一对应（400×400，圆角已经烤进 alpha）。
const PRODUCT_PNGS: [&[u8]; 33] = [
    include_bytes!("../resources/products/p00.png"),
    include_bytes!("../resources/products/p01.png"),
    include_bytes!("../resources/products/p02.png"),
    include_bytes!("../resources/products/p03.png"),
    include_bytes!("../resources/products/p04.png"),
    include_bytes!("../resources/products/p05.png"),
    include_bytes!("../resources/products/p06.png"),
    include_bytes!("../resources/products/p07.png"),
    include_bytes!("../resources/products/p08.png"),
    include_bytes!("../resources/products/p09.png"),
    include_bytes!("../resources/products/p10.png"),
    include_bytes!("../resources/products/p11.png"),
    include_bytes!("../resources/products/p12.png"),
    include_bytes!("../resources/products/p13.png"),
    include_bytes!("../resources/products/p14.png"),
    include_bytes!("../resources/products/p15.png"),
    include_bytes!("../resources/products/p16.png"),
    include_bytes!("../resources/products/p17.png"),
    include_bytes!("../resources/products/p18.png"),
    include_bytes!("../resources/products/p19.png"),
    include_bytes!("../resources/products/p20.png"),
    include_bytes!("../resources/products/p21.png"),
    include_bytes!("../resources/products/p22.png"),
    include_bytes!("../resources/products/p23.png"),
    include_bytes!("../resources/products/p24.png"),
    include_bytes!("../resources/products/p25.png"),
    include_bytes!("../resources/products/p26.png"),
    include_bytes!("../resources/products/p27.png"),
    include_bytes!("../resources/products/p28.png"),
    include_bytes!("../resources/products/p29.png"),
    include_bytes!("../resources/products/p30.png"),
    include_bytes!("../resources/products/p31.png"),
    include_bytes!("../resources/products/p32.png"),
];
/// 还没揭晓的礼物：礼盒里只给一个问号。
const MYSTERY_PNG: &[u8] = include_bytes!("../resources/products/mystery.png");

/// 商品卡的最小宽度和间距：能排几列就排几列（2..=5），卡宽按剩余宽度均分。
const TILE_MIN: f64 = 150.0;
const TILE_GAP: f64 = 12.0;
/// 侧栏宽度（DSL 里 sidebar 的 width）。
const SIDEBAR_W: f64 = 208.0;

// ---------------------------------------------------------------------------
// 响应式布局
// ---------------------------------------------------------------------------

/// 解析后的布局形态：手机（底部导航）/ 平板（侧栏，无右列）/ 桌面（侧栏 + 右列）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Shape {
    Phone,
    Tablet,
    #[default]
    Desktop,
}

/// 一次布局解析的结果；结果不变就不重复套用。
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shaping {
    shape: Shape,
    /// 右列辅助栏是否还有位置。
    aside: bool,
    /// 横屏手机这类矮 surface：开场三屏收起大图标。
    short: bool,
}

/// 开场三屏的正文列宽。
const INTRO_COL: f64 = 620.0;
/// 右列辅助栏的栏内容宽度（栏本身还要加一截贴边的内边距）。
const ASIDE_COL: f64 = 300.0;
/// 手机形态的上限宽度：低于此值走底部导航单列。
const PHONE_MAX: f64 = 720.0;
/// 桌面形态（侧栏 + 右列）的下限宽度。
const DESKTOP_MIN: f64 = 1060.0;
/// 右列辅助栏至少需要的宽度。
const ASIDE_MIN: f64 = 900.0;

/// 自己吃左右留白的那些容器：正文区不留左右内边距，滚动条才贴得住窗口边，
/// 所以这一份留白落到每个可滚动页面（以及送礼页那条固定底栏）身上。
const SIDE_PAD_VIEWS: [LiveId; 25] = [
    live_id!(page_gift),
    live_id!(page_box),
    live_id!(page_online_gift),
    live_id!(page_pact),
    live_id!(page_contacts),
    live_id!(page_me),
    live_id!(sd_scroll),
    live_id!(sd_bar),
    live_id!(page_card),
    live_id!(page_open),
    live_id!(page_sent),
    live_id!(page_wallet),
    live_id!(page_account_received),
    live_id!(page_account_sent),
    live_id!(page_settings),
    live_id!(page_profile),
    live_id!(page_direct_order),
    live_id!(page_product),
    live_id!(co_scroll),
    live_id!(co_bar),
    live_id!(page_wishes),
    live_id!(page_wish),
    live_id!(we_scroll),
    live_id!(we_bar),
    live_id!(page_wish_pick),
];

const CART_FRIEND_CHIPS: [LiveId; 4] = [live_id!(ca_f0), live_id!(ca_f1), live_id!(ca_f2), live_id!(ca_f3)];
const PROFILE_ADDRESS_ROWS: [LiveId; 6] = [
    live_id!(pf_a0), live_id!(pf_a1), live_id!(pf_a2),
    live_id!(pf_a3), live_id!(pf_a4), live_id!(pf_a5),
];
const PROFILE_ADDRESS_TEXT: [LiveId; 6] = [
    live_id!(pf_a0_text), live_id!(pf_a1_text), live_id!(pf_a2_text),
    live_id!(pf_a3_text), live_id!(pf_a4_text), live_id!(pf_a5_text),
];
const PROFILE_ADDRESS_ACTIONS: [LiveId; 6] = [
    live_id!(pf_a0_actions), live_id!(pf_a1_actions), live_id!(pf_a2_actions),
    live_id!(pf_a3_actions), live_id!(pf_a4_actions), live_id!(pf_a5_actions),
];
const PROFILE_ADDRESS_EDIT: [LiveId; 6] = [
    live_id!(pf_a0_edit), live_id!(pf_a1_edit), live_id!(pf_a2_edit),
    live_id!(pf_a3_edit), live_id!(pf_a4_edit), live_id!(pf_a5_edit),
];
const PROFILE_ADDRESS_DELETE: [LiveId; 6] = [
    live_id!(pf_a0_del), live_id!(pf_a1_del), live_id!(pf_a2_del),
    live_id!(pf_a3_del), live_id!(pf_a4_del), live_id!(pf_a5_del),
];

/// 可用 surface 尺寸 → 布局形态。宿主把应用放进多大的 tile，这里就按多大
/// 排版：窗口尺寸不代表 tile 尺寸，所以只看自己拿到的 turtle。
fn shaping_for(size: Vec2d) -> Shaping {
    let (w, h) = (size.x, size.y);
    let shape = if w < PHONE_MAX {
        Shape::Phone
    } else if w < DESKTOP_MIN {
        Shape::Tablet
    } else {
        Shape::Desktop
    };
    Shaping {
        shape,
        aside: shape == Shape::Desktop && w >= ASIDE_MIN,
        short: h < 560.0,
    }
}

// ---------------------------------------------------------------------------
// 视图状态
// ---------------------------------------------------------------------------

/// 盖在 Tab 页上的覆盖页。它们不是第六个 Tab：从哪来、回哪去（见 `go_back`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Overlay {
    /// 发起神秘送礼（单页向导）。
    Send,
    /// 礼卡已生成 / 看礼卡。
    Card,
    /// 拆礼（收到的礼物，或「以 TA 的视角看看」预览）。
    Open,
    /// 送出详情。
    Sent,
    Wallet,
    ReceivedGifts,
    SentGifts,
    Settings,
    Profile,
    DirectOrder,
    OnlineGift,
    /// 商品详情（可以带着「这是谁心愿单上的哪一件」的上下文）。
    Product,
    /// 结算：订单 + 余额抵扣 + 付款方式，付完原地变成功页。
    Checkout,
    /// 我的心愿单（列表）。
    Wishes,
    /// 一张心愿单（自己的或好友的）。
    Wish,
    /// 发布 / 编辑心愿单。
    WishEdit,
    /// 挑一件：替好友的含糊心愿挑，或者给自己的心愿单挑。
    WishPick,
}

/// A data destination in the account workspace, independent of the gift-box tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccountSection {
    Wishes,
    Received,
    Sent,
    Drafts,
    Wallet,
}
impl AccountSection {
    const ALL: [Self; 5] = [
        Self::Wishes,
        Self::Received,
        Self::Sent,
        Self::Drafts,
        Self::Wallet,
    ];
    fn id(self) -> LiveId {
        match self {
            Self::Wishes => live_id!(account_wishes),
            Self::Received => live_id!(account_received),
            Self::Sent => live_id!(account_sent),
            Self::Drafts => live_id!(account_drafts),
            Self::Wallet => live_id!(account_wallet),
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Wishes => "心愿单",
            Self::Received => "收到的礼物",
            Self::Sent => "送出的礼物",
            Self::Drafts => "送礼草稿",
            Self::Wallet => "钱包与流水",
        }
    }
}
const RECEIVED_GIFT_ROWS: [LiveId; 12] = [
    live_id!(received_gift_0),
    live_id!(received_gift_1),
    live_id!(received_gift_2),
    live_id!(received_gift_3),
    live_id!(received_gift_4),
    live_id!(received_gift_5),
    live_id!(received_gift_6),
    live_id!(received_gift_7),
    live_id!(received_gift_8),
    live_id!(received_gift_9),
    live_id!(received_gift_10),
    live_id!(received_gift_11),
];
const SENT_GIFT_ROWS: [LiveId; 12] = [
    live_id!(sent_gift_0),
    live_id!(sent_gift_1),
    live_id!(sent_gift_2),
    live_id!(sent_gift_3),
    live_id!(sent_gift_4),
    live_id!(sent_gift_5),
    live_id!(sent_gift_6),
    live_id!(sent_gift_7),
    live_id!(sent_gift_8),
    live_id!(sent_gift_9),
    live_id!(sent_gift_10),
    live_id!(sent_gift_11),
];

/// 心愿单上的一件：（心愿单 id, 第几件）。
type WishAt = (u64, usize);

/// 拆礼页的阶段。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Decrypt,
    Reveal,
    Accept,
    Swap,
    Done,
}

impl Stage {
    fn index(self) -> usize {
        match self {
            Stage::Decrypt => 0,
            Stage::Reveal => 1,
            Stage::Accept => 2,
            Stage::Swap => 3,
            Stage::Done => 4,
        }
    }
}

/// 校验失败时的一句话（全是 data.rs 里的静态文案）。
/// 写成别名是因为 Script 派生宏不认字段类型里的生命周期。
type Msg = &'static str;

#[derive(Script, ScriptHook, Widget)]
pub struct LiyuView {
    #[deref]
    view: View,
    #[rust]
    state: LiyuState,
    #[rust]
    profile: Profile,
    #[rust]
    profile_err: Option<Msg>,
    #[rust]
    profile_edit_address: Option<i64>,
    #[rust]
    commerce: Commerce,
    #[rust]
    gift_client: GiftClient,
    #[rust]
    online_gift_id: Option<i64>,
    #[rust]
    online_gift_sent: bool,
    #[rust]
    online_gift_err: Option<Msg>,
    #[rust]
    direct_product: Option<u16>,
    #[rust]
    direct_contact_idx: usize,
    #[rust]
    pending_contacts: Option<contacts::Import>,
    #[rust]
    editing_contact: Option<usize>,
    #[rust]
    contact_page: usize,
    #[rust]
    direct_contact_page: usize,
    #[rust]
    contact_import_receiver: Option<std::sync::mpsc::Receiver<Result<contacts::Import, Msg>>>,
    #[rust]
    contact_import_poll: Timer,
    #[rust]
    online_notice: Option<commerce_client::Notification>,
    #[rust]
    direct_error: Option<Msg>,
    #[rust]
    pal: Pal,
    #[rust]
    initialized: bool,
    #[rust]
    tab: usize,
    #[rust]
    overlay: Option<Overlay>,

    // ---- 挑礼 ----
    /// 0 = 全部，1..=8 = Category::ALL 的下标 + 1。
    #[rust]
    cat_idx: usize,
    #[rust]
    gift_rows: Vec<u16>,
    /// 挑礼页的分段：false = 挑礼物，true = 熟人的心愿单。
    #[rust]
    gift_wish_seg: bool,
    #[rust]
    friend_rows: Vec<u64>,

    /// 覆盖页的来路：从一张覆盖页进另一张时把前一张压进来，返回时弹出。
    /// 从 Tab 上直接进来的覆盖页栈是空的，返回回到 `send_from` 那个 Tab。
    #[rust]
    back_stack: Vec<Overlay>,

    // ---- 商品详情 ----
    #[rust]
    pd_item: u16,
    /// 从好友心愿单进来的：这件对应哪条心愿。
    #[rust]
    pd_wish: Option<WishAt>,
    #[rust]
    pd_remote: Option<ProductDetail>,
    #[rust]
    rel_rows: Vec<u16>,

    // ---- 结算 ----
    /// 付完了：生成的礼物 id（页面变成功页）。
    #[rust]
    co_paid: Option<u64>,
    #[rust]
    co_err: Option<Msg>,

    // ---- 心愿单 ----
    #[rust]
    my_rows: Vec<u64>,
    #[rust]
    wish_id: Option<u64>,
    /// 心愿单详情的二次确认：0 = 没有，1 = 提前结束，2 = 删除。
    #[rust]
    wish_armed: u8,
    #[rust]
    wish_draft: WishDraft,
    #[rust]
    wish_err: Option<Msg>,
    /// 「说个大概」的小表单是否展开，以及选中的品类 / 预算档。
    #[rust]
    vague_open: bool,
    #[rust]
    vague_kind: usize,
    #[rust]
    vague_budget: usize,
    #[rust]
    aud_names: Vec<String>,
    /// 「挑一件」页：`Some` = 替好友的这条心愿挑；`None` = 给自己的心愿单挑。
    #[rust]
    wp_wish: Option<WishAt>,
    #[rust]
    wp_cat: usize,
    #[rust]
    wp_rows: Vec<u16>,
    /// 熟人列表每一行对应的进行中心愿单。
    #[rust]
    contact_wish: Vec<Option<u64>>,

    // ---- 送礼页 ----
    /// 送礼页从哪个 Tab 进来的，返回就回哪。
    #[rust]
    send_from: usize,
    #[rust]
    draft: SendDraft,
    #[rust]
    send_peers: Vec<String>,
    #[rust]
    send_error: Option<Msg>,
    #[rust]
    contract_on: bool,
    /// 回礼时顶上那条提示（「回礼 · 刚变现的 ¥x 可用」）。
    #[rust]
    return_banner: Option<String>,
    /// 「换一件」的就地目录是否展开。
    #[rust]
    pick_open: bool,
    /// 就地目录里列的是哪几件（同类，或者含糊心愿的候选）。
    #[rust]
    pick_rows: Vec<u16>,

    // ---- 礼盒 ----
    #[rust]
    box_sent: bool,
    #[rust]
    box_rows: Vec<u64>,
    #[rust]
    account_section: Option<AccountSection>,
    #[rust]
    received_gift_rows: Vec<i64>,
    #[rust]
    sent_gift_rows: Vec<i64>,

    // ---- 礼卡页 ----
    #[rust]
    card_gift: Option<u64>,
    #[rust]
    card_style: ShareStyle,

    // ---- 拆礼页 ----
    #[rust]
    open_gift: Option<u64>,
    /// 「以 TA 的视角看看」：只读预览自己送出的礼卡。
    #[rust]
    open_preview: bool,
    #[rust]
    stage: Stage,
    #[rust]
    open_wrong: Option<String>,
    #[rust]
    open_err: Option<Msg>,
    #[rust]
    accept_agree: bool,
    /// 换购 / 折现页：false = 折成余额，true = 换一份。
    #[rust]
    swap_exchange: bool,
    #[rust]
    swap_pick: Option<u16>,
    #[rust]
    swap_rows: Vec<u16>,
    #[rust]
    cand_names: Vec<String>,

    // ---- 送出详情 ----
    #[rust]
    sent_gift: Option<u64>,
    #[rust]
    withdraw_armed: bool,

    // ---- 契约 ----
    #[rust]
    pact_theirs: bool,
    #[rust]
    pact_rows: Vec<u64>,

    // ---- 熟人 ----
    #[rust]
    contact_rows: Vec<usize>,
    #[rust]
    add_error: Option<AddContactError>,
    #[rust]
    import_menu: bool,

    // ---- 设置 ----
    #[rust]
    reset_armed: bool,
    #[rust]
    export_note: Option<String>,
    #[rust]
    nick_err: Option<Msg>,
    /// 设置页选的深浅：这一拍的 action 走完再换（换主题会重建整棵树）。
    #[rust]
    pending_theme: Option<ThemeMode>,

    // ---- 开场三屏 / 通知 / toast ----
    #[rust]
    intro: Option<usize>,
    /// 认证闸:未登录时 true,整应用被 page_auth 盖住。
    #[rust]
    auth_gate: bool,
    #[rust]
    server_error: Option<Msg>,
    /// 认证界面处于注册模式(验证码字段可见,按钮文案互换)。
    #[rust]
    auth_register: bool,
    #[rust]
    notice: Option<Notice>,
    /// 这次运行里已经发过的通知种类（只活在内存里，重启重新算一次没有坏处）。
    #[rust]
    notice_sent: Vec<NoticeKind>,
    #[rust]
    notice_poll: Timer,
    #[rust]
    toast_timer: Timer,
    #[rust]
    undo: Option<UndoSnapshot>,

    // ---- 布局 / 动画 ----
    #[rust]
    shaping: Option<Shaping>,
    #[rust]
    last_size: Vec2d,
    /// 页面正文能用的宽度（去掉侧栏、右列和左右留白）。商品宫格按它排列数。
    #[rust]
    content_w: f64,
    /// #7 账号/设置分层导航状态机(L1/L2/L3 层级、编辑草稿、desktop/mobile 布局)。
    #[rust]
    account_nav: Option<account_nav::AccountNavModel>,
    #[rust]
    account_edit: Option<u8>,
    /// 心愿单详情的进度条：已认领的比例。
    #[rust]
    wish_frac: f64,
    /// 商品图纹理，按目录下标懒加载；最后一格是问号图。
    #[rust]
    tex: Vec<Option<Texture>>,
    // ---- 头像工坊（资料页）----
    /// 正在编辑的头像会话（选图 → 裁切/旋转 → 确认上传或取消）。
    #[rust]
    avatar_session: Option<avatar::AvatarEditSession>,
    /// 头像预览纹理（当前头像或编辑中快照）；None = 默认占位图。
    #[rust]
    avatar_tex: Option<Texture>,
    /// 头像上传/删除进行中：禁用按钮并显示状态，防止重复提交。
    #[rust]
    avatar_busy: bool,
    /// 待上传头像字节（断网时 profile_client 已落盘，这里留一份用于预览）。
    #[rust]
    avatar_pending_bytes: Option<Vec<u8>>,
    /// 裁切对齐位置(0-8,3×3 网格:0=左上,4=居中,8=右下)。点「裁切」按此位置取最大正方形。
    #[rust]
    avatar_crop_anchor: u8,
    /// AI 送礼草稿闸(prepare_gift_draft):只开本机可审阅草稿,不发送/不扣款/不改服务端。
    /// Option 包装以符合 derive 宏字段形式(与 avatar_session 同模式)。
    #[rust]
    draft_gate: Option<ai_draft::DraftGate>,
    /// 当前待本人确认的草稿(AI 开好、界面呈现、本人确认或取消后才算数)。
    #[rust]
    draft_pending: Option<ai_draft::GiftDraft>,
    /// 已上传头像的服务端字节缓存:(avatar_url, 解码前字节)。只在 URL 变化时重新 GET,
    /// 避免 refresh_avatar 反复同步请求卡界面。
    #[rust]
    avatar_server_cache: Option<(String, Vec<u8>)>,
    /// Reuse the decoded GPU texture until the actual preview bytes change.
    #[rust]
    avatar_texture_bytes: Option<Vec<u8>>,
}

impl LiyuView {
    // ---- 小工具 ----

    fn set_text(&mut self, cx: &mut Cx, path: &[LiveId], text: &str) {
        let widget = self.view.widget(cx, path);
        if widget.text() != text {
            widget.set_text(cx, text);
        }
    }

    fn show(&mut self, cx: &mut Cx, path: &[LiveId], on: bool) {
        let widget = self.view.widget(cx, path);
        if widget.visible() != on {
            widget.set_visible(cx, on);
        }
    }

    fn input_text(&mut self, cx: &mut Cx, path: &[LiveId]) -> String {
        self.view.widget(cx, path).text()
    }

    fn tint_text(&mut self, cx: &mut Cx, path: &[LiveId], c: Vec4f) {
        let mut w = self.view.widget(cx, path);
        script_apply_eval!(cx, w, { draw_text +: { color: #(c) } });
    }

    fn tint_bg(&mut self, cx: &mut Cx, path: &[LiveId], c: Vec4f) {
        let mut w = self.view.widget(cx, path);
        script_apply_eval!(cx, w, { draw_bg +: { color: #(c) } });
    }

    /// Keyboard navigation belongs to the app, so it works in both hosting modes.
    fn handle_keyboard_navigation(&mut self, cx: &mut Cx, event: &Event) -> bool {
        let Event::KeyDown(key) = event else { return false };
        if key.key_code == KeyCode::Escape && self.tab == 4 {
            self.go_back(cx); return true;
        }
        if key.modifiers.control || key.modifiers.logo || key.modifiers.alt {
            return false;
        }
        fn collect(widget: WidgetRef, parents: Vec<Area>, out: &mut Vec<(WidgetRef, Vec<Area>)>, cx: &Cx) {
            if !widget.visible() { return; }
            let area = widget.area();
            let focusable = widget.borrow::<TextInput>().is_some()
                || widget.borrow::<Button>().is_some_and(|button| button.enabled())
                || widget.borrow::<CheckBox>().is_some();
            if focusable && !area.is_empty() && area.is_valid(cx) {
                out.push((widget.clone(), parents.clone()));
            }
            let mut next = parents;
            if !area.is_empty() { next.push(area); }
            let mut children = Vec::new();
            widget.children(&mut |_, child| children.push(child));
            for child in children { collect(child, next.clone(), out, cx); }
        }
        let mut stops = Vec::new();
        if self.auth_gate || self.intro.is_some() {
            let path = if self.auth_gate { ids!(page_auth) } else { ids!(page_intro) };
            collect(self.view.widget(cx, path), Vec::new(), &mut stops, cx);
        } else {
            let mut children = Vec::new();
            self.view.children(&mut |_, child| children.push(child));
            for child in children { collect(child, Vec::new(), &mut stops, cx); }
        }
        let focused = stops.iter().position(|(widget, _)| widget.key_focus(cx));
        if key.key_code == KeyCode::Tab {
            if stops.is_empty() { return true; }
            let index = focus_index(focused, stops.len(), key.modifiers.shift);
            let (widget, parents) = &stops[index];
            if let Some(mut input) = widget.borrow_mut::<TextInput>() {
                input.take_key_focus(cx);
            } else {
                widget.set_key_focus(cx);
            }
            let mut from = widget.area();
            for parent in parents.iter().rev() {
                cx.send_trigger(*parent, Trigger { id: live_id!(scroll_focus_nav), from });
                from = *parent;
            }
            widget.redraw(cx);
            return true;
        }
        if self.auth_gate && key.is_repeat && matches!(key.key_code, KeyCode::ReturnKey | KeyCode::NumpadEnter) {
            return true;
        }
        if matches!(key.key_code, KeyCode::ReturnKey | KeyCode::NumpadEnter | KeyCode::Space) && !key.is_repeat {
            if let Some(index) = focused {
                let widget = &stops[index].0;
                let actions = cx.capture_actions(|cx| {
                    if widget.borrow::<Button>().is_some() {
                        cx.widget_action(widget.widget_uid(), ButtonAction::Clicked(key.modifiers));
                    } else if let Some(mut check) = widget.borrow_mut::<CheckBox>() {
                        let active = !check.active(cx);
                        check.set_active(cx, active, Animate::Yes);
                        cx.widget_action(widget.widget_uid(), CheckBoxAction::Change(active));
                    }
                });
                if !actions.is_empty() {
                    self.handle_actions(cx, &actions);
                    return true;
                }
            }
        }
        false
    }

    fn clicked(&mut self, cx: &mut Cx, path: &[LiveId], actions: &Actions) -> bool {
        self.view.button(cx, path).clicked(actions)
    }

    fn toggled(&mut self, cx: &mut Cx, path: &[LiveId], actions: &Actions) -> bool {
        self.view.check_box(cx, path).changed(actions).is_some()
    }

    /// 让一组芯片 / 分段互斥选中（CheckBox 本身可以再点关掉，这里强制单选）。
    fn set_chip_group(&mut self, cx: &mut Cx, chips: &[LiveId], active: usize) {
        for (j, id) in chips.iter().enumerate() {
            let chip = self.view.check_box(cx, &[*id]);
            if chip.active(cx) != (j == active) {
                chip.set_active(cx, j == active, Animate::Yes);
            }
        }
    }

    /// 目录下标 → 纹理（`None` = 问号图）。第一次用到才解码，之后一直复用。
    fn texture(&mut self, cx: &mut Cx, i: Option<u16>) -> Option<Texture> {
        let k = i.map_or(CATALOG.len(), |i| (i as usize).min(CATALOG.len() - 1));
        if self.tex.len() <= CATALOG.len() {
            self.tex.resize(CATALOG.len() + 1, None);
        }
        if self.tex[k].is_none() {
            let bytes = if k < CATALOG.len() { PRODUCT_PNGS[k] } else { MYSTERY_PNG };
            self.tex[k] = ImageBuffer::from_png(bytes).ok().map(|b| b.into_new_mip_texture(cx));
        }
        self.tex[k].clone()
    }

    /// 给一个 LiyuThumb 换商品图，并恢复不透明（列表行是复用的，上一次可能被调暗过）。
    fn set_img(&mut self, cx: &mut Cx, path: &[LiveId], i: Option<u16>) {
        let t = self.texture(cx, i);
        let img = self.view.image(cx, path);
        img.set_texture(cx, t);
        if let Some(mut im) = img.borrow_mut() {
            im.draw_bg.opacity = 1.0;
        };
    }

    /// 调暗一张图（心愿单上已经有人送的那几件）。
    fn dim_img(&mut self, cx: &mut Cx, path: &[LiveId], opacity: f32) {
        if let Some(mut im) = self.view.image(cx, path).borrow_mut() {
            im.draw_bg.opacity = opacity;
        }
    }

    fn set_img_size(&mut self, cx: &mut Cx, path: &[LiveId], side: f64) {
        if let Some(mut im) = self.view.image(cx, path).borrow_mut() {
            im.walk.width = Size::Fixed(side);
            im.walk.height = Size::Fixed(side);
        }
    }

    /// 写一条 LiyuKV：左边键，中间值，右边（可空）金额。
    fn set_kv(&mut self, cx: &mut Cx, row: &[LiveId], k: &str, v: &str, r: &str) {
        self.set_text(cx, &join(row, live_id!(kv_k)), k);
        self.set_text(cx, &join(row, live_id!(kv_v)), v);
        self.set_text(cx, &join(row, live_id!(kv_r)), r);
    }

    fn tone_color(&self, t: Tone) -> Vec4f {
        match t {
            Tone::Pending => self.pal.blue,
            Tone::Done => self.pal.good,
            Tone::Returned => self.pal.ink_3,
        }
    }

    // ---- 导航 ----

    fn set_tab(&mut self, cx: &mut Cx, i: usize) {
        if self.account_has_unsaved_changes(cx) {
            for root in [live_id!(sidebar),live_id!(tabbar)] {
                for (j,id) in TABS.iter().enumerate() {self.view.check_box(cx,&[root,*id]).set_active(cx,j==self.tab,Animate::Yes);}
            }
            self.toast(cx, "有未保存的修改，请保存或取消"); return;
        }
        if self.account_edit.is_some() { self.cancel_account_edit(cx); }
        // 切 Tab 即离开所有覆盖页；送礼草稿随之丢掉（它本来就不落盘）。
        self.overlay = None;
        self.back_stack.clear();
        self.wish_armed = 0;
        self.open_preview = false;
        self.withdraw_armed = false;
        self.reset_armed = false;
        self.import_menu = false;
        self.add_error = None;
        self.tab = i;
        self.account_section = None;
        for (j, id) in TABS.iter().enumerate() {
            for root in [live_id!(sidebar), live_id!(tabbar)] {
                self.view
                    .check_box(cx, &[root, *id])
                    .set_active(cx, j == i, Animate::Yes);
                // DrawSvg 没有 active 通道，图标的选中色在这里逐个 apply。
                let c = if j == i { self.pal.warm } else { self.pal.ink_2 };
                let mut tab = self.view.widget(cx, &[root, *id]);
                script_apply_eval!(cx, tab, { draw_icon +: { color: #(c) } });
            }
        }
        for (j, id) in ASIDES.iter().enumerate() {
            self.show(cx, &[*id], j == i);
        }
        match i {
            0 => self.refresh_gift(cx),
            1 => self.refresh_box(cx),
            2 => self.refresh_pacts(cx),
            3 => self.refresh_contacts(cx),
            _ => {
                self.refresh_me(cx);
                match self.account_nav.as_ref().and_then(|n| if n.depth()>1 {n.selected()} else {None}) {
                    Some(account_nav::AccountCategory::Profile | account_nav::AccountCategory::AccountContact | account_nav::AccountCategory::ShippingAddress) => self.open_profile(cx),
                    Some(account_nav::AccountCategory::General | account_nav::AccountCategory::Notifications | account_nav::AccountCategory::DataManagement) => self.open_settings(cx),
                    _ => {}
                }
            },
        }
        self.update_page_visibility(cx);
    }

    fn open_overlay(&mut self, cx: &mut Cx, o: Overlay) {
        if self.account_has_unsaved_changes(cx) {
            self.toast(cx,"有未保存的修改，请保存或取消"); return;
        }
        if self.account_edit.is_some() { self.cancel_account_edit(cx); }
        self.overlay = Some(o);
        self.import_menu = false;
        // 覆盖页每次都从顶上看起：上一次滚到哪儿和这一次无关。
        self.scroll_top(cx, o);
        self.update_page_visibility(cx);
    }

    /// 进一张覆盖页并记下来路：从 Tab 上进来的清空来路栈、记住 Tab；
    /// 从另一张覆盖页进来的把那一张压栈，返回时回到它。
    fn nav_to(&mut self, cx: &mut Cx, o: Overlay) {
        match self.overlay {
            None => {
                self.send_from = self.tab;
                self.back_stack.clear();
            }
            Some(cur) if cur != o => self.back_stack.push(cur),
            _ => {}
        }
        self.open_overlay(cx, o);
    }

    /// 回到上一张覆盖页（按当前数据重铺，但不回顶：刚才看到哪儿还在哪儿）。
    /// 栈空了就回进来时的那个 Tab。
    fn pop_back(&mut self, cx: &mut Cx) {
        match self.back_stack.pop() {
            Some(o) => {
                self.overlay = Some(o);
                self.refresh_overlay(cx, o);
                self.update_page_visibility(cx);
            }
            None => self.set_tab(cx, self.send_from),
        }
    }

    fn refresh_overlay(&mut self, cx: &mut Cx, o: Overlay) {
        match o {
            Overlay::Send => self.refresh_send(cx),
            Overlay::Card => self.refresh_card(cx),
            Overlay::Open => self.refresh_open(cx),
            Overlay::Sent => self.refresh_sent(cx),
            Overlay::Wallet => self.refresh_wallet(cx),
            Overlay::ReceivedGifts => self.refresh_account_gifts(cx, false),
            Overlay::SentGifts => self.refresh_account_gifts(cx, true),
            Overlay::Settings => self.refresh_settings(cx),
            Overlay::Profile => self.refresh_profile(cx),
            Overlay::DirectOrder => self.refresh_direct_order(cx),
            Overlay::OnlineGift => self.refresh_online_gift(cx),
            Overlay::Product => self.refresh_product(cx),
            Overlay::Checkout => self.refresh_checkout(cx),
            Overlay::Wishes => self.refresh_wishes(cx),
            Overlay::Wish => self.refresh_wish(cx),
            Overlay::WishEdit => self.refresh_wish_edit(cx),
            Overlay::WishPick => self.refresh_wish_pick(cx),
        }
    }

    fn scroll_top(&mut self, cx: &mut Cx, o: Overlay) {
        let id = match o {
            Overlay::Send => live_id!(sd_scroll),
            Overlay::Card => live_id!(page_card),
            Overlay::Open => live_id!(page_open),
            Overlay::Sent => live_id!(page_sent),
            Overlay::Wallet => live_id!(page_wallet),
            Overlay::ReceivedGifts => live_id!(page_account_received),
            Overlay::SentGifts => live_id!(page_account_sent),
            Overlay::Settings => live_id!(page_settings),
            Overlay::Profile => live_id!(page_profile),
            Overlay::DirectOrder => live_id!(page_direct_order),
            Overlay::OnlineGift => live_id!(page_online_gift),
            Overlay::Product => live_id!(page_product),
            Overlay::Checkout => live_id!(co_scroll),
            Overlay::Wishes => live_id!(page_wishes),
            Overlay::Wish => live_id!(page_wish),
            Overlay::WishEdit => live_id!(we_scroll),
            Overlay::WishPick => live_id!(page_wish_pick),
        };
        self.view.view(cx, &[id]).set_scroll_pos(cx, Vec2d::default());
    }

    /// 顶栏「返回」：礼卡、拆礼、送出详情、钱包、设置有固定的来处；
    /// 送礼、商品、结算和心愿单这几张按来路栈一层层退回去。
    fn go_back(&mut self, cx: &mut Cx) {
        if self.tab == 4 && self.account_has_unsaved_changes(cx) {
            self.toast(cx, "有未保存的修改，请保存或点击取消"); return;
        }
        if self.tab == 4 && self.account_edit.is_some() {
            self.cancel_account_edit(cx); return;
        }
        if self.tab == 4 && self.account_nav.as_ref().and_then(|n| n.selected()).is_some() && matches!(self.overlay,None|Some(Overlay::Profile|Overlay::Settings)) {
            self.account_nav.as_mut().unwrap().pop_discarding();
            self.overlay = None;
            self.update_page_visibility(cx); return;
        }
        if self.tab == 4 && self.account_section.is_some() {
            if let Some(o) = self.back_stack.pop() {
                self.overlay = Some(o);
                if matches!(o, Overlay::ReceivedGifts | Overlay::SentGifts) { let _ = self.gift_client.refresh(); }
                self.refresh_overlay(cx, o);
            } else {
                self.account_section = None;
                self.overlay = None;
                self.refresh_me(cx);
            }
            self.update_page_visibility(cx);
            return;
        }
        match self.overlay {
            Some(
                Overlay::Send
                | Overlay::Product
                | Overlay::Checkout
                | Overlay::Wishes
                | Overlay::Wish
                | Overlay::WishEdit
                | Overlay::WishPick
                | Overlay::DirectOrder
                | Overlay::OnlineGift
                | Overlay::ReceivedGifts
                | Overlay::SentGifts,
            ) => self.pop_back(cx),
            Some(Overlay::Card) | Some(Overlay::Sent) => {
                self.box_sent = true;
                self.set_tab(cx, 1);
            }
            Some(Overlay::Open) => {
                if self.open_preview {
                    self.open_preview = false;
                    self.refresh_card(cx);
                    self.open_overlay(cx, Overlay::Card);
                } else if matches!(self.stage, Stage::Accept | Stage::Swap) {
                    self.enter_stage(cx, Stage::Reveal);
                } else {
                    self.box_sent = false;
                    self.set_tab(cx, 1);
                }
            }
            Some(Overlay::Wallet) | Some(Overlay::Settings) | Some(Overlay::Profile) => self.set_tab(cx, 4),
            None => {}
        }
    }

    fn update_page_visibility(&mut self, cx: &mut Cx) {
        let gate = self.auth_gate;
        let intro = !gate && self.intro.is_some();
        self.show(cx, ids!(page_auth), gate);
        self.show(cx, ids!(page_intro), intro);
        // 认证闸优先:gate 时隐藏所有主页面与 overlay,只留 page_auth 全屏,
        // 避免认证卡片盖在挑礼首页上被底部裁掉(默认小窗口登录/注册按钮不可见)。
        for (j, id) in PAGES.iter().enumerate() {
            self.show(cx, &[*id], !gate && !intro && j == self.tab && (self.overlay.is_none() || (j == 4 && matches!(self.overlay, Some(Overlay::Profile | Overlay::Settings)))));
        }
        self.show(cx, ids!(account_workspace), !intro && !gate);
        let ov = if intro || gate { None } else { self.overlay };
        self.show(cx, ids!(page_send), ov == Some(Overlay::Send));
        self.show(cx, ids!(page_card), ov == Some(Overlay::Card));
        self.show(cx, ids!(page_open), ov == Some(Overlay::Open));
        self.show(cx, ids!(page_sent), ov == Some(Overlay::Sent));
        self.show(cx, ids!(page_wallet), ov == Some(Overlay::Wallet));
        self.show(cx, ids!(page_account_received), ov == Some(Overlay::ReceivedGifts));
        self.show(cx, ids!(page_account_sent), ov == Some(Overlay::SentGifts));
        self.show(cx, ids!(page_settings), ov == Some(Overlay::Settings));
        self.show(cx, ids!(page_profile), ov == Some(Overlay::Profile));
        self.refresh_account_nav(cx);
        self.show(cx, ids!(page_direct_order), ov == Some(Overlay::DirectOrder));
        self.show(cx, ids!(page_online_gift), ov == Some(Overlay::OnlineGift));
        self.show(cx, ids!(page_product), ov == Some(Overlay::Product));
        self.show(cx, ids!(page_checkout), ov == Some(Overlay::Checkout));
        self.show(cx, ids!(page_wishes), ov == Some(Overlay::Wishes));
        self.show(cx, ids!(page_wish), ov == Some(Overlay::Wish));
        self.show(cx, ids!(page_wish_edit), ov == Some(Overlay::WishEdit));
        self.show(cx, ids!(page_wish_pick), ov == Some(Overlay::WishPick));
        let menu = self.import_menu && !gate && !intro && self.overlay.is_none() && self.tab == 3;
        self.show(cx, ids!(ct_menu_layer), menu);
        self.refresh_topbar(cx);
        // Authentication and onboarding hide the chrome without resizing the
        // surface. Restore navigation when those states change at the same size.
        self.reshape(cx);
        self.redraw(cx);
    }

    fn refresh_topbar(&mut self, cx: &mut Cx) {
        let title: String = match self.overlay {
            None => self.account_section.filter(|_| self.tab == 4).map_or(TAB_TITLES[self.tab], AccountSection::label).into(),
            Some(Overlay::Send) => {
                if self.draft.wish.is_some() {
                    "认领心愿".into()
                } else if self.return_banner.is_some() {
                    "回一份礼".into()
                } else {
                    "发起神秘送礼".into()
                }
            }
            Some(Overlay::Card) => "神秘礼卡".into(),
            Some(Overlay::Open) if self.open_preview => "TA 看到的礼卡".into(),
            Some(Overlay::Open) => match self.stage {
                Stage::Decrypt => "神秘礼物".into(),
                Stage::Reveal => "礼物揭晓".into(),
                Stage::Accept => "收下礼物".into(),
                Stage::Swap => if self.swap_exchange { "换一份".into() } else { "折成余额".into() },
                Stage::Done => "完成".into(),
            },
            Some(Overlay::Sent) => "送出详情".into(),
            Some(Overlay::Wallet) => "钱包与流水".into(),
            Some(Overlay::ReceivedGifts) => "收到的礼物".into(),
            Some(Overlay::SentGifts) => "送出的礼物".into(),
            Some(Overlay::Settings) => "设置".into(),
            Some(Overlay::Profile) => "账户资料".into(),
            Some(Overlay::DirectOrder) => "直接送礼".into(),
            Some(Overlay::OnlineGift) => "服务器礼物".into(),
            Some(Overlay::Product) => "商品详情".into(),
            Some(Overlay::Checkout) => {
                if self.co_paid.is_some() { "支付成功".into() } else { "确认订单".into() }
            }
            Some(Overlay::Wishes) => "我的心愿单".into(),
            Some(Overlay::Wish) => match self.wish_id.and_then(|id| self.state.wishlist(id)) {
                Some(w) if w.is_mine() => "我的心愿单".into(),
                Some(w) => format!("{}的心愿单", w.owner),
                None => "心愿单".into(),
            },
            Some(Overlay::WishEdit) => {
                if self.wish_draft.id.is_some() { "编辑心愿单".into() } else { "发布心愿单".into() }
            }
            Some(Overlay::WishPick) => {
                if self.wp_wish.is_some() { "帮 TA 挑一件".into() } else { "挑一件放进心愿单".into() }
            }
        };
        self.set_text(cx, ids!(tb_title), &title);
        let overlay = self.overlay.is_some();
        self.show(cx, ids!(tb_back), overlay && !matches!(self.overlay,Some(Overlay::Profile|Overlay::Settings)));
        self.show(cx, ids!(tb_action), !overlay && self.tab <= 1);
        let wish_seg = self.tab == 0 && self.gift_wish_seg;
        self.set_text(cx, ids!(tb_action), if wish_seg { "发布心愿单" } else { "发起神秘送礼" });
        self.show(cx, ids!(tb_add), !overlay && self.tab == 3);
    }

    fn refresh_all(&mut self, cx: &mut Cx) {
        if self.auth_gate {
            self.refresh_auth(cx);
            self.update_page_visibility(cx);
            return;
        }
        self.set_tab_keep_overlay(cx);
        self.refresh_gift(cx);
        self.refresh_box(cx);
        self.refresh_pacts(cx);
        self.refresh_contacts(cx);
        self.refresh_me(cx);
        self.refresh_wallet(cx);
        self.refresh_settings(cx);
        self.refresh_profile(cx);
        self.refresh_direct_order(cx);
        if let Some(o) = self.overlay {
            self.refresh_overlay(cx, o);
        }
        self.refresh_intro(cx);
        self.update_page_visibility(cx);
    }

    /// 只重画导航的选中态（换主题之后用：不能像 set_tab 那样顺手关掉覆盖页）。
    fn set_tab_keep_overlay(&mut self, cx: &mut Cx) {
        let i = self.tab;
        for (j, id) in TABS.iter().enumerate() {
            for root in [live_id!(sidebar), live_id!(tabbar)] {
                self.view
                    .check_box(cx, &[root, *id])
                    .set_active(cx, j == i, Animate::No);
                let c = if j == i { self.pal.warm } else { self.pal.ink_2 };
                let mut tab = self.view.widget(cx, &[root, *id]);
                script_apply_eval!(cx, tab, { draw_icon +: { color: #(c) } });
            }
        }
        for (j, id) in ASIDES.iter().enumerate() {
            self.show(cx, &[*id], j == i);
        }
    }

    /// Hidden lists refresh when entered by set_tab; only update the visible tab now.
    fn after_data_change(&mut self, cx: &mut Cx) {
        if self.overlay.is_none()
            || matches!(self.overlay, Some(Overlay::Profile | Overlay::Settings))
        {
            match self.tab {
                0 => self.refresh_gift(cx),
                1 => self.refresh_box(cx),
                2 => self.refresh_pacts(cx),
                3 => self.refresh_contacts(cx),
                _ => self.refresh_me(cx),
            }
        }
        match self.overlay {
            Some(Overlay::Wallet) => self.refresh_wallet(cx),
            Some(Overlay::ReceivedGifts) => self.refresh_account_gifts(cx, false),
            Some(Overlay::SentGifts) => self.refresh_account_gifts(cx, true),
            _ => {},
        }
    }

    // ---- 主题 ----

    /// 换一套深浅：重跑「装色板 → 重建预设」，再拿新的类型默认值把整棵树
    /// ScriptReapply 一遍，之后把跟数据走的颜色重新写一次。
    fn apply_theme(&mut self, cx: &mut Cx, mode: ThemeMode) {
        if theme::mode() == mode {
            return;
        }
        theme::set_mode(mode);
        cx.with_vm(|vm| {
            vm.with_reload(|vm| {
                theme::install(vm);
                canvas::script_mod(vm);
                account_ui::script_mod(vm);
                script_mod(vm);
            });
            let source = script_eval!(vm, { mod.widgets.LiyuView });
            self.script_apply(vm, &Apply::ScriptReapply, &mut Scope::empty(), source);
        });
        self.after_restyle(cx);
    }

    /// reapply 会把 DSL 里的文案、显隐、布局都刷回去，所以整张界面按状态重铺一遍。
    fn after_restyle(&mut self, cx: &mut Cx) {
        self.pal = Pal::read(cx);
        self.shaping = None;
        // Reapply may reset image widgets, so restore their cached content too.
        self.avatar_texture_bytes = None;
        self.refresh_all(cx);
        self.redraw(cx);
    }

    // ---- 开场三屏 ----

    fn open_intro(&mut self, cx: &mut Cx, step: usize) {
        self.intro = Some(step.min(INTRO.len() - 1));
        self.refresh_intro(cx);
        self.update_page_visibility(cx);
        self.reshape(cx);
    }

    /// 看完和跳过都记 onboarded；重看的入口在「我 → 关于礼遇」。
    fn close_intro(&mut self, cx: &mut Cx) {
        self.intro = None;
        self.state.settings.onboarded = true;
        self.state.save();
        self.update_page_visibility(cx);
        self.reshape(cx);
        // 引导开着时通知是压住的；关掉马上查一次，不用等下一轮 60 秒轮询。
        self.poll_notices(cx);
    }

    fn refresh_intro(&mut self, cx: &mut Cx) {
        let Some(step) = self.intro else { return };
        let (title, body) = INTRO[step];
        for (i, id) in INTRO_ICONS.iter().enumerate() {
            self.show(cx, &[*id], i == step);
        }
        self.set_text(cx, ids!(in_title), title);
        self.set_text(cx, ids!(in_body), body);
        for (i, id) in INTRO_DOTS.iter().enumerate() {
            let c = if i == step { self.pal.blue } else { self.pal.line_soft };
            self.tint_bg(cx, &[*id], c);
        }
        let last = step + 1 == INTRO.len();
        self.show(cx, ids!(in_points), last);
        for (i, id) in INTRO_POINT_ROWS.iter().enumerate() {
            self.set_text(cx, &[*id], &format!("· {}", INTRO_POINTS[i]));
        }
        self.show(cx, ids!(in_back), step > 0);
        self.show(cx, ids!(in_skip), !last);
        self.set_text(cx, ids!(in_next), if last { "知道了，开始" } else { "下一步" });
    }

    // ---- 通知 / toast ----

    /// 每分钟一次：先把到期的礼物退回，再看有没有该发的通知。
    fn poll_notices(&mut self, cx: &mut Cx) {
        let n = self.state.sweep(today_days());
        if n > 0 {
            self.after_data_change(cx);
            if self.overlay == Some(Overlay::Open) {
                self.refresh_open(cx);
            }
            if self.overlay == Some(Overlay::Sent) {
                self.refresh_sent(cx);
            }
        }
        // 认证闸压着时不弹业务通知(有礼物等你拆等);登录/选演示后按账号再查。
        if self.auth_gate || self.notice.is_some() || self.intro.is_some() {
            return;
        }
        if self.online_notice.is_some() {
            return;
        }
        if let Ok(rows) = commerce_client::unread_notifications() {
            if let Some(n) = rows.into_iter().next() {
                self.set_text(cx, ids!(nt_title), &n.title);
                self.set_text(cx, ids!(nt_text), &n.body);
                self.show(cx, ids!(notice_layer), true);
                self.online_notice = Some(n);
                self.redraw(cx);
            }
        }
    }

    fn close_notice(&mut self, cx: &mut Cx) {
        self.notice = None;
        self.online_notice = None;
        self.show(cx, ids!(notice_layer), false);
        self.redraw(cx);
    }

    /// 一条没有撤销按钮的提示（撤销那条走 `offer_undo`）。
    fn toast(&mut self, cx: &mut Cx, text: &str) {
        self.set_text(cx, ids!(to_text), text);
        self.show(cx, ids!(to_undo), false);
        self.show(cx, ids!(toast), true);
        self.undo = None;
        if !self.toast_timer.is_empty() {
            cx.stop_timer(self.toast_timer);
        }
        self.toast_timer = cx.start_timeout(2.4);
        self.redraw(cx);
    }

    /// 删完之后给 5 秒后悔时间。
    fn offer_undo(&mut self, cx: &mut Cx, snap: UndoSnapshot) {
        if !self.toast_timer.is_empty() {
            cx.stop_timer(self.toast_timer);
        }
        self.set_text(cx, ids!(to_text), &snap.label);
        self.show(cx, ids!(to_undo), true);
        self.show(cx, ids!(toast), true);
        self.undo = Some(snap);
        self.toast_timer = cx.start_timeout(5.0);
        self.redraw(cx);
    }

    fn close_toast(&mut self, cx: &mut Cx, restore: bool) {
        if !self.toast_timer.is_empty() {
            cx.stop_timer(self.toast_timer);
            self.toast_timer = Timer::empty();
        }
        let snap = self.undo.take();
        self.show(cx, ids!(toast), false);
        if restore {
            if let Some(u) = snap {
                self.state.restore(u);
                self.refresh_contacts(cx);
            }
        }
        self.redraw(cx);
    }

    // ---- 响应式 ----

    fn update_responsive(&mut self, cx: &mut Cx, size: Vec2d) {
        if size.x < 2.0 || size.y < 2.0 {
            return;
        }
        let want = shaping_for(size);
        // 开场三屏的正文列按宽度居中，所以尺寸变了也要重排；尺寸和形态都没变就什么都不做。
        if self.shaping == Some(want) && self.last_size == size {
            return;
        }
        self.last_size = size;
        self.shaping = Some(want);
        self.apply_shaping(cx, want);
    }

    fn reshape(&mut self, cx: &mut Cx) {
        if let Some(s) = self.shaping {
            self.apply_shaping(cx, s);
        }
    }

    fn apply_shaping(&mut self, cx: &mut Cx, s: Shaping) {
        let phone = s.shape == Shape::Phone;
        // 开场三屏里连导航都不给：三句话不该能被一脚跨过去。认证闸同样全屏。
        let intro = self.intro.is_some() || self.auth_gate;
        self.show(cx, ids!(sidebar), !phone && !intro);
        self.show(cx, ids!(topbar), !intro);
        self.show(cx, ids!(tabbar), phone && !intro);
        self.show(cx, ids!(aside), s.aside && !intro && self.tab != 4);
        if let Some(mut top) = self.view.view(cx, ids!(tb_bar)).borrow_mut() {
            top.layout.padding = if phone {
                Inset { left: 18.0, right: 12.0, top: 0.0, bottom: 0.0 }
            } else {
                Inset { left: 20.0, right: 20.0, top: 0.0, bottom: 0.0 }
            };
        }
        let side = if phone { 16.0 } else { 20.0 };
        if let Some(mut v) = self.view.view(cx, ids!(page_intro)).borrow_mut() {
            let col = ((self.last_size.x - INTRO_COL) * 0.5).max(side);
            v.layout.padding = Inset { left: col, right: col, top: 0.0, bottom: 0.0 };
        }
        self.show(cx, ids!(in_icons), !s.short);
        if let Some(mut main) = self.view.view(cx, ids!(main)).borrow_mut() {
            main.layout.padding = if phone {
                Inset { left: 0.0, right: 0.0, top: 6.0, bottom: 10.0 }
            } else {
                Inset { left: 0.0, right: 0.0, top: 8.0, bottom: 16.0 }
            };
        }
        for id in SIDE_PAD_VIEWS {
            if let Some(mut v) = self.view.view(cx, &[id]).borrow_mut() {
                v.layout.padding.left = side;
                v.layout.padding.right = side;
            }
        }
        if let Some(mut n) = self.view.view(cx, ids!(notice_layer)).borrow_mut() {
            n.layout.padding.left = side;
            n.layout.padding.right = side;
        }
        if let Some(mut aside) = self.view.view(cx, ids!(aside)).borrow_mut() {
            aside.layout.padding.left = 0.0;
            aside.layout.padding.right = side;
            aside.walk.width = Size::Fixed(ASIDE_COL + side);
        }
        // 正文宽度：窗口减去侧栏、右列和页面自己的左右留白；再留 6px 给滚动条和取整，
        // 宁可每张卡窄一点，也别被挤掉一列。
        let nav_w = if phone || intro { 0.0 } else { SIDEBAR_W };
        let aside_w = if s.aside && !intro && self.tab != 4 { ASIDE_COL + side } else { 0.0 };
        let account_menu_w = if self.tab == 4 && self.last_size.x >= 820.0 && !intro { 200.0 + side } else { 0.0 };
        self.content_w = (self.last_size.x - nav_w - aside_w - account_menu_w - 2.0 * side - 6.0).max(240.0);
        let compact_content = phone || self.content_w < 640.0;
        // 礼卡页：宽屏左卡右操作，手机竖排。
        if let Some(mut row) = self.view.view(cx, ids!(cd_row)).borrow_mut() {
            row.layout.flow = if compact_content {
                Flow::Down
            } else {
                Flow::Right { row_align: RowAlign::Top, wrap: false }
            };
        }
        // 竖排时卡片占整行并居中，不然贴左边、右侧空一块。
        if let Some(mut prev) = self.view.view(cx, ids!(cd_prev)).borrow_mut() {
            prev.walk.width = if compact_content { Size::fill() } else { Size::fit() };
            prev.layout.align.x = if compact_content { 0.5 } else { 0.0 };
        }
        if let Some(mut menu) = self.view.view(cx, ids!(me_list)).borrow_mut() {
            menu.walk.margin.left = side;
            menu.walk.margin.right = if self.last_size.x < 820.0 { side } else { 0.0 };
        }
        self.layout_tiles(cx);
        self.refresh_account_nav(cx);
        // 商品详情：宽屏左图右信息，手机图在上、占满一行（最多 360）。
        if let Some(mut top) = self.view.view(cx, ids!(pd_top)).borrow_mut() {
            top.layout.flow = if compact_content {
                Flow::Down
            } else {
                Flow::Right { row_align: RowAlign::Top, wrap: false }
            };
        }
        let pd = if compact_content { (self.content_w - 36.0).clamp(0.0, 360.0).floor() } else { 300.0 };
        self.set_img_size(cx, ids!(pd_img), pd);
        self.redraw(cx);
    }

    /// 三处商品宫格（挑礼、挑一件、同类还有）按正文宽度一起排：能放几列放几列，
    /// 卡宽均分剩下的宽度，图边长 = 卡宽 − 两侧内边距。
    fn layout_tiles(&mut self, cx: &mut Cx) {
        let w = self.content_w;
        let cols = (((w + TILE_GAP) / (TILE_MIN + TILE_GAP)).floor() as usize).clamp(2, 5);
        let tile = ((w - TILE_GAP * (cols - 1) as f64) / cols as f64).floor();
        for id in GRID_CARDS.iter().chain(PICK_CARDS.iter()).chain(REL_CARDS.iter()) {
            if let Some(mut v) = self.view.view(cx, &[*id]).borrow_mut() {
                v.walk.width = Size::Fixed(tile);
            }
            self.set_img_size(cx, &[*id, live_id!(pk_img)], tile - 16.0);
        }
        self.layout_wish_fill(cx);
    }

    /// 心愿单详情的进度条（轨道是卡片内容宽度，LiyuCard 左右各 18 内边距）。
    fn layout_wish_fill(&mut self, cx: &mut Cx) {
        let full = (self.content_w - 36.0).max(0.0);
        if let Some(mut v) = self.view.view(cx, ids!(wd_fill)).borrow_mut() {
            v.walk.width = Size::Fixed((full * self.wish_frac.clamp(0.0, 1.0)).floor());
        }
    }
}

/// 从 prepare_gift_draft 结果 JSON 文本提取 draft_id(`"draft_id":"liyu-draft-NNNNNN"`)。
/// 字符串扫描即可,草稿 JSON 由本机 draft_json 生成、格式稳定,无需引入 serde 依赖问题。
fn extract_draft_id(text: &str) -> Option<String> {
    let key = "\"draft_id\":\"";
    let start = text.find(key)? + key.len();
    let rest = &text[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// 合并只读投影(ai.rs)与草稿闸(ai_draft.rs)两份 ServiceManifest:
/// id/label/brief 取只读投影的(主清单),tools/topics 取两清单并集。
/// 同一 tool 名只保留只读投影版本(写工具 prepare_gift_draft 名唯一,不会撞)。
pub fn merge_manifests(mut base: ServiceManifest, extra: ServiceManifest) -> ServiceManifest {

    for tool in extra.tools {
        if !base.tools.iter().any(|t| t.name == tool.name) {
            base.tools.push(tool);
        }
    }
    for topic in extra.topics {
        if !base.topics.iter().any(|t| t.name == topic.name) {
            base.topics.push(topic);
        }
    }
    base
}

impl LiyuView {
    /// AI 工具应答：只读工具给礼盒的匿名汇总（ai.rs）；`prepare_gift_draft` 走本机草稿闸
    /// （只开草稿、待本人在界面确认，不发送/不扣款/不改服务端）。
    pub fn ai_answer(&mut self, cx: &mut Cx, call: &ServiceCall) -> ToolResult {
        if call.tool.as_str() == "prepare_gift_draft" {
            // 写工具必须先有真实身份:首次未选择登录/注册(AuthMode::None)时拒绝,
            // 不允许匿名调用方在本机开送礼草稿。守卫放在调用侧(身份上下文所在),
            // 不侵入 ai_draft.rs 的纯状态机逻辑。
            if profile_client::mode() == profile_client::AuthMode::None {
                return ToolResult::refused(
                    &call.call_id,
                    "还没有选择身份：请先登录、注册，再来准备送礼草稿",
                );
            }
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let account = profile_client::active_identifier();
            let gate = self.draft_gate.get_or_insert_with(ai_draft::DraftGate::new);
            let result = gate.answer(&account, call, now_ms);
            // 开成功的草稿存为「待本人确认」：从结果 JSON 取 draft_id 再取回草稿。
            if result.outcome == ToolOutcome::Ok {
                if let Some(id) = extract_draft_id(&result.text) {
                    self.draft_pending = self.draft_gate.as_ref().and_then(|g| g.draft(&id));
                    // 建好草稿立即刷新「我」页(me_draft 卡片所在),用户即使已停在该页也能看到待确认。
                    self.refresh_me(cx);
                    self.redraw(cx);
                }
            }
            return result;
        }
        ai::answer(&ai::BoxSummary::from_state(&self.state), call)
    }

    // ---- 列表行 ----

    /// 目录里的一件礼物铺进一行 LiyuGiftRow（挑礼页和送礼页的「换一件」共用）。
    fn fill_gift_row(&mut self, cx: &mut Cx, row: LiveId, i: u16) {
        let it = item(i);
        self.set_img(cx, &[row, live_id!(gr_img)], Some(i));
        self.set_text(cx, &[row, live_id!(gr_name)], it.name);
        self.set_text(cx, &[row, live_id!(gr_sub)], &item_sub(it));
        self.set_text(cx, &[row, live_id!(gr_price)], &yuan(it.price));
    }

    fn set_row(&mut self, cx: &mut Cx, row: &[LiveId], name: &str, sub: &str, value: &str) {
        self.set_text(cx, &join(row, live_id!(st_name)), name);
        self.show(cx, &join(row, live_id!(st_sub)), !sub.is_empty());
        self.set_text(cx, &join(row, live_id!(st_sub)), sub);
        self.set_text(cx, &join(row, live_id!(st_val)), value);
    }

    /// 写一条开关行。开关的样子只跟着传进来的 `on` 走。
    fn set_switch(&mut self, cx: &mut Cx, row: &[LiveId], name: &str, sub: &str, on: bool) {
        self.set_text(cx, &join(row, live_id!(sw_name)), name);
        self.show(cx, &join(row, live_id!(sw_sub)), !sub.is_empty());
        self.set_text(cx, &join(row, live_id!(sw_sub)), sub);
        let c = if on { self.pal.blue } else { self.pal.track_off };
        self.tint_bg(cx, &join(row, live_id!(sw_track)), c);
        self.show(cx, &join(row, live_id!(sw_off)), !on);
        self.show(cx, &join(row, live_id!(sw_on)), on);
    }

    /// 空态：列表没有内容时露出 LiyuEmpty，并写上文案与（可选的）动作。
    fn apply_list_state(&mut self, cx: &mut Cx, slot: &[LiveId], text: &str, action: &str, n: usize) {
        self.show(cx, slot, n == 0);
        if n > 0 {
            return;
        }
        self.set_text(cx, &join(slot, live_id!(em_text)), text);
        self.show(cx, &join(slot, live_id!(em_sub)), false);
        self.show(cx, &join(slot, live_id!(em_action)), !action.is_empty());
        self.set_text(cx, &join(slot, live_id!(em_action)), action);
    }

    // ---- 挑礼 ----

    fn refresh_gift(&mut self, cx: &mut Cx) {
        let wish = self.gift_wish_seg;
        self.set_chip_group(cx, &GIFT_SEGS, wish as usize);
        self.show(cx, ids!(gf_shop), !wish);
        self.show(cx, ids!(gf_wish), wish);
        self.set_chip_group(cx, &CAT_CHIPS, self.cat_idx);
        let cat = self.cat_idx.checked_sub(1).map(|i| Category::ALL[i]);
        self.gift_rows = catalog_in(cat);
        let rows = self.gift_rows.clone();
        self.fill_cards(cx, &GRID_CARDS, &rows, &[]);
        self.refresh_friend_wishes(cx);
        let bal = yuan(self.state.balance());
        self.set_text(cx, ids!(ag2_v), &bal);
    }

    /// 一张商品卡：图 / 名字 / 品牌 / 价格 + 形态；`note` 是「帮 TA 挑」时的理由。
    fn fill_product_card(&mut self, cx: &mut Cx, card: LiveId, i: u16, note: &str) {
        let it = item(i);
        self.set_img(cx, &[card, live_id!(pk_img)], Some(i));
        self.set_text(cx, &[card, live_id!(pk_name)], it.name);
        self.set_text(cx, &[card, live_id!(pk_brand)], it.brand);
        self.set_text(cx, &[card, live_id!(pk_price)], &yuan(it.price));
        self.set_text(cx, &[card, live_id!(pk_tag)], kind_text(it));
        self.show(cx, &[card, live_id!(pk_note)], !note.is_empty());
        self.set_text(cx, &[card, live_id!(pk_note)], note);
    }

    /// 把一串商品铺进一组卡片，多出来的卡收起。`notes` 可以比 `items` 短。
    fn fill_cards(&mut self, cx: &mut Cx, cards: &[LiveId], items: &[u16], notes: &[String]) {
        for (j, card) in cards.iter().enumerate() {
            let hit = items.get(j).copied();
            self.show(cx, &[*card], hit.is_some());
            if let Some(i) = hit {
                let note = notes.get(j).cloned().unwrap_or_default();
                self.fill_product_card(cx, *card, i, &note);
            }
        }
    }

    // ---- 送礼页 ----

    /// 进送礼页。`peer` 为空 = 先不指定；`banner` 只在回礼时有。
    fn open_send(&mut self, cx: &mut Cx, item_idx: u16, peer: String, banner: Option<String>) {
        self.draft = SendDraft {
            item: item_idx,
            peer,
            unlock: Unlock::GuessWho,
            use_balance: self.state.balance() > 0,
            pay: Some(0),
            ..SendDraft::default()
        };
        self.return_banner = banner;
        self.contract_on = false;
        self.pick_open = false;
        self.send_error = None;
        for id in [live_id!(sd_clue), live_id!(sd_ans), live_id!(sd_pact_in), live_id!(sd_msg)] {
            self.set_text(cx, &[id], "");
        }
        self.refresh_send(cx);
        self.nav_to(cx, Overlay::Send);
    }

    fn refresh_send(&mut self, cx: &mut Cx) {
        let d = self.draft.clone();
        let today = today_days();
        // 从心愿单来的：那张单子和那一件。
        let wish = d.wish.and_then(|(w, k)| {
            let l = self.state.wishlist(w)?.clone();
            let wi = l.items.get(k)?.clone();
            Some((l, wi))
        });
        // 顶上一行：认领心愿 / 回礼
        let banner = match &wish {
            Some((l, wi)) => Some(format!("认领{}的心愿：{}", spaced(&l.owner), wi.title())),
            None => self.return_banner.clone(),
        };
        self.show(cx, ids!(sd_banner), banner.is_some());
        if let Some(b) = banner {
            self.set_text(cx, ids!(sd_banner), &b);
        }
        // 已选礼物
        let it = item(d.item);
        self.set_img(cx, ids!(sd_img), Some(d.item));
        self.set_text(cx, ids!(sd_name), it.name);
        self.set_text(cx, ids!(sd_sub), &item_sub(it));
        self.set_text(cx, ids!(sd_price), &yuan(it.price));
        // 「换一件」：具体的心愿不能换；说了个大概的只在符合的里面换；平时在同类里换。
        let exact = wish.as_ref().is_some_and(|(_, wi)| wi.is_exact());
        self.pick_rows = match &wish {
            Some((_, wi)) => wish_candidates(wi).into_iter().map(|m| m.item).collect(),
            None => catalog_in(Some(it.cat)),
        };
        self.pick_rows.truncate(PICK_ROWS.len());
        let can_pick = !exact && self.pick_rows.len() > 1;
        if !can_pick {
            self.pick_open = false;
        }
        self.show(cx, ids!(sd_change), can_pick);
        let label = if self.pick_open {
            "收起"
        } else if wish.is_some() {
            "换一件符合的"
        } else {
            "同类换一件"
        };
        self.set_text(cx, ids!(sd_change), label);
        self.show(cx, ids!(sd_pick), self.pick_open);
        if self.pick_open {
            let rows = self.pick_rows.clone();
            for (j, row) in PICK_ROWS.iter().enumerate() {
                let hit = rows.get(j).copied();
                self.show(cx, &[*row], hit.is_some());
                if let Some(i) = hit {
                    self.fill_gift_row(cx, *row, i);
                }
            }
        }

        // 认领心愿时收礼人是定的：芯片收起，只写一行。
        let fixed = wish.is_some();
        self.show(cx, ids!(sd_peers), !fixed);
        self.show(cx, ids!(sd_peer_fixed), fixed);
        if let Some((l, _)) = &wish {
            self.set_text(cx, ids!(sd_peer_fixed), &format!("{}（心愿单的主人）", l.owner));
        }
        // 什么时候送到：只有心愿单、而且日子还没到，才能约在那一天。
        let ahead = wish.as_ref().map(|(l, _)| l.event_on).filter(|&e| e > today);
        self.show(cx, ids!(sd_dv), ahead.is_some());
        if let Some(e) = ahead {
            self.set_text(cx, ids!(dv0), &format!("{} 当天送到", md_cn(e)));
            self.set_chip_group(cx, &DELIVER_SEGS, if d.deliver_on.is_some() { 0 } else { 1 });
            let n = if d.deliver_on.is_some() {
                format!("礼卡 {} 才出现在 TA 的礼盒里。在那之前，TA 的心愿单上只显示这件「已被认领」。", md_cn(e))
            } else {
                "礼卡现在就发出，TA 马上就能拆。".to_string()
            };
            self.set_text(cx, ids!(sd_dv_n), &n);
        }

        // 送给谁：备注只自己看得见，礼卡上不出现。
        let mut peers: Vec<String> = Vec::new();
        if !d.peer.is_empty() {
            peers.push(d.peer.clone());
        }
        for c in &self.state.contacts {
            if peers.len() >= PEER_SLOTS {
                break;
            }
            if !peers.contains(&c.label) {
                peers.push(c.label.clone());
            }
        }
        for (j, id) in PEER_CHIPS.iter().take(PEER_SLOTS).enumerate() {
            self.show(cx, &[*id], j < peers.len());
            if let Some(p) = peers.get(j) {
                self.set_text(cx, &[*id], p);
            }
        }
        let active = if d.peer.is_empty() {
            PEER_SLOTS
        } else {
            peers.iter().position(|p| *p == d.peer).unwrap_or(PEER_SLOTS)
        };
        self.send_peers = peers;
        self.set_chip_group(cx, &PEER_CHIPS, active);

        // 1 · 解密游戏
        let ui = Unlock::ALL.iter().position(|u| *u == d.unlock).unwrap_or(0);
        self.set_chip_group(cx, &UNLOCK_SEGS, ui);
        let free = d.unlock == Unlock::Free;
        let (clue_l, clue_ph, ans_l, ans_ph) = match d.unlock {
            Unlock::GuessWho => ("线索（会印在礼卡上）", "比如：上周一起喝咖啡的那个人", "", ""),
            Unlock::Question => (
                "问题（会印在礼卡上）",
                "比如：我们第一次见面在哪个城市？",
                "答案（只有你知道，不上礼卡）",
                "答案，最多 20 字",
            ),
            Unlock::Passphrase => (
                "暗号提示（可不写）",
                "比如：我们的口头禅",
                "暗号（不上礼卡）",
                "TA 要输入的暗号",
            ),
            Unlock::Free => ("", "", "", ""),
        };
        self.show(cx, ids!(sd_clue_box), !free);
        self.set_text(cx, ids!(sd_clue_l), clue_l);
        self.view.text_input(cx, ids!(sd_clue)).set_empty_text(cx, clue_ph.to_string());
        let has_ans = !ans_l.is_empty();
        self.show(cx, ids!(sd_ans_box), has_ans);
        self.set_text(cx, ids!(sd_ans_l), ans_l);
        self.view.text_input(cx, ids!(sd_ans)).set_empty_text(cx, ans_ph.to_string());
        let guess = d.unlock == Unlock::GuessWho;
        self.show(cx, ids!(sd_nick), guess);
        if guess {
            let nick = self.state.settings.nickname.clone();
            self.set_text(
                cx,
                ids!(sd_nick),
                &format!("TA 要猜的是你的称呼「{nick}」，{MAX_ATTEMPTS} 次机会；猜不中礼物照样拆开，只是不揭晓你。"),
            );
        }
        self.show(cx, ids!(sd_free), free);

        // 2 · 契约
        self.set_switch(
            cx,
            ids!(sd_pact_row),
            "2 · 附加契约（可选）",
            "TA 收下即答应，比如「下周回请一杯咖啡」",
            self.contract_on,
        );
        self.show(cx, ids!(sd_pact), self.contract_on);
        self.refresh_presets(cx);

        // 底栏：只说合计和余额能抵多少，怎么付到结算页再选。
        let (a, _) = self.state.pay_split(it.price, true);
        let split = if a > 0 {
            format!("合计 {} · 余额最多可抵 {}", yuan(it.price), yuan(a))
        } else {
            format!("合计 {} · 下一步选付款方式", yuan(it.price))
        };
        self.set_text(cx, ids!(sd_split), &split);
        self.show(cx, ids!(sd_err), self.send_error.is_some());
        if let Some(e) = self.send_error {
            self.set_text(cx, ids!(sd_err), e);
        }
        self.set_text(cx, ids!(sd_go), &format!("去结算 {}", yuan(it.price)));
        self.redraw(cx);
    }

    /// 预设契约芯片：输入框里正好是哪一条，哪一枚就亮。
    fn refresh_presets(&mut self, cx: &mut Cx) {
        let text = self.input_text(cx, ids!(sd_pact_in));
        let hit = PACT_PRESETS.iter().position(|(_, full)| text.trim() == *full);
        for (j, id) in PRESET_CHIPS.iter().enumerate() {
            self.view.check_box(cx, &[*id]).set_active(cx, Some(j) == hit, Animate::Yes);
        }
    }

    fn submit_send(&mut self, cx: &mut Cx) {
        self.draft.clue = self.input_text(cx, ids!(sd_clue));
        self.draft.answer = self.input_text(cx, ids!(sd_ans));
        self.draft.message = self.input_text(cx, ids!(sd_msg));
        self.draft.contract = if self.contract_on {
            Some(self.input_text(cx, ids!(sd_pact_in)))
        } else {
            None
        };
        if self.draft.unlock == Unlock::Free || self.draft.unlock == Unlock::GuessWho {
            self.draft.answer.clear();
        }
        if self.draft.unlock == Unlock::Free {
            self.draft.clue.clear();
        }
        // 先按送出时的规则校验一遍（不动数据），过了才进结算页，付钱那一步只会因为钱出错。
        match self.state.check_send(&self.draft, today_days()) {
            Ok(()) => {
                self.send_error = None;
                self.open_checkout(cx);
            }
            Err(e) => {
                self.send_error = Some(e);
                self.refresh_send(cx);
            }
        }
    }

    // ---- 礼卡页 ----

    fn card_scene(&self) -> Option<share::ShareCardScene> {
        let g = self.state.gift(self.card_gift?)?;
        Some(share::scene_for(g, self.card_style))
    }

    fn refresh_card(&mut self, cx: &mut Cx) {
        let Some(g) = self.card_gift.and_then(|id| self.state.gift(id)).cloned() else {
            return;
        };
        if let Some(scene) = self.card_scene() {
            if let Some(mut card) = self.view.widget(cx, ids!(cd_share)).borrow_mut::<LiyuShareCard>() {
                card.set_scene(scene);
            }
        }
        self.view.widget(cx, ids!(cd_share)).redraw(cx);
        let fresh = g.state() == GiftState::Sealed;
        let ok = if fresh {
            format!("礼卡已生成。发给{}，等 TA 来拆。", spaced(&g.shown_recipient()))
        } else {
            format!("这张礼卡的状态：{}", g.status_text(today_days()))
        };
        self.set_text(cx, ids!(cd_ok), &ok);
        self.set_text(cx, ids!(cd_link), &gift_link(g.id));
        let si = if self.card_style == ShareStyle::Warm { 0 } else { 1 };
        self.set_chip_group(cx, &STYLE_SEGS, si);
        self.show(cx, ids!(cd_saved), false);
    }

    fn copy_link(&mut self, cx: &mut Cx, id: u64) {
        let Some(g) = self.state.gift(id) else { return };
        let text = format!("有一份神秘礼物等你来拆：{}", gift_link(g.id));
        cx.copy_to_clipboard(&text);
        self.toast(cx, "礼卡链接已复制");
    }

    // ---- 礼盒 ----

    fn refresh_box(&mut self, cx: &mut Cx) {
        let _ = self.gift_client.refresh();
        self.set_text(cx, ids!(bx_mode), if self.gift_client.online {
            "服务器礼盒 · 按收礼人/送礼人权限显示"
        } else {
            "礼盒加载失败，请检查服务器连接"
        });
        if self.gift_client.online {
            self.set_chip_group(cx, &BOX_SEGS, self.box_sent as usize);
            let rows = if self.box_sent {
                self.gift_client.outbox.iter().map(|g| {
                    let title = CATALOG.get(g.product_id as usize).map(|p| p.name).unwrap_or("商品");
                    (g.id, Some(g.product_id), format!("{} · 送给 {}", title, g.recipient_name),
                     "服务器送出礼物".to_string(), format!("{} · {} · 物流签收 {} · TA 确认 {}", g.state,g.delivery_text(),
                        if g.carrier_delivered { "是" } else { "否" }, if g.recipient_confirmed { "是" } else { "否" }))
                }).collect::<Vec<_>>()
            } else {
                self.gift_client.inbox.iter().map(|g| {
                    let revealed = g.product_id.is_some();
                    let title = g.product_id.and_then(|i| CATALOG.get(i as usize)).map(|p| p.name)
                        .unwrap_or("一份神秘礼物");
                    (g.id, g.product_id, title.to_string(),
                     if revealed { "已揭晓的礼物".into() } else { "拆开前不展示商品和送礼人".into() },
                     g.state.clone())
                }).collect::<Vec<_>>()
            };
            self.box_rows = rows.iter().map(|x| x.0 as u64).collect();
            for (j, row) in BOX_ROWS.iter().enumerate() {
                let Some((_, pic, title, sub, state)) = rows.get(j) else { self.show(cx, &[*row], false); continue };
                self.show(cx, &[*row], true);
                self.set_img(cx, &[*row, live_id!(bx_img)], *pic);
                self.set_text(cx, &[*row, live_id!(bx_title)], title);
                self.set_text(cx, &[*row, live_id!(bx_sub)], sub);
                self.set_text(cx, &[*row, live_id!(bx_state)], state);
            }
            self.apply_list_state(cx, ids!(bx_empty), if self.box_sent { "暂无服务器送出礼物" } else { "暂无服务器收到礼物" }, "", rows.len());
            let more = rows.len().saturating_sub(BOX_ROWS.len());
            self.show(cx, ids!(bx_more), more > 0);
            self.set_text(cx, ids!(bx_more), &format!("还有 {more} 份较早的没有显示"));
            self.set_text(cx, ids!(ab1_b), &format!("服务器收到 {} 份\n服务器送出 {} 份", self.gift_client.inbox.len(), self.gift_client.outbox.len()));
            return;
        }
        self.box_rows.clear();
        for row in BOX_ROWS { self.show(cx, &[row], false); }
        self.show(cx, ids!(bx_more), false);
        self.apply_list_state(cx, ids!(bx_empty), "礼盒加载失败", "请检查服务器连接后重试", 0);
    }

    fn open_online_gift(&mut self, cx: &mut Cx, id: i64, sent_role: bool) {
        self.online_gift_err = self.gift_client.detail(id, sent_role).err();
        if let Some(err) = self.online_gift_err {
            self.toast(cx, err);
            if !self.gift_client.online {
                match self.account_section {
                    Some(AccountSection::Received) => self.refresh_account_gifts(cx, false),
                    Some(AccountSection::Sent) => self.refresh_account_gifts(cx, true),
                    _ => self.refresh_box(cx),
                }
            }
            return;
        }
        self.online_gift_id = Some(id);
        self.online_gift_sent = sent_role;
        self.view.check_box(cx, ids!(og_agree)).set_active(cx, false, Animate::No);
        if let Some(address) = self.profile.addresses.first().cloned() {
            self.set_text(cx, ids!(og_ship_name), &address.recipient_name);
            self.set_text(cx, ids!(og_ship_phone), &address.phone);
            self.set_text(cx, ids!(og_ship_address), &address.address);
        }
        self.refresh_online_gift(cx);
        self.nav_to(cx, Overlay::OnlineGift);
    }

    fn refresh_online_gift(&mut self, cx: &mut Cx) {
        self.show(cx, ids!(og_err), self.online_gift_err.is_some());
        if let Some(err) = self.online_gift_err { self.set_text(cx, ids!(og_err), err); }
        let sent = self.online_gift_sent;
        self.set_text(cx, ids!(og_mode), if sent { "服务器送礼方视图 · 仅显示签收与确认结果" }
            else { "服务器收礼方视图 · 物流仅你可见" });
        self.show(cx, ids!(og_open), false);
        self.show(cx, ids!(og_answer), false);
        self.show(cx, ids!(og_answer_btn), false);
        self.show(cx, ids!(og_agree), false);
        self.show(cx, ids!(og_ship_name), false);
        self.show(cx, ids!(og_ship_phone), false);
        self.show(cx, ids!(og_ship_address), false);
        self.show(cx, ids!(og_accept), false);
        self.show(cx, ids!(og_logistics), false);
        self.show(cx, ids!(og_clue), false);
        if !self.gift_client.online {
            self.set_text(cx, ids!(og_mode), "连接中断 · 在线礼物已暂停");
            self.set_text(cx, ids!(og_title), "服务器暂时连不上");
            self.set_text(cx, ids!(og_summary), "请返回礼盒查看本地演示数据。在线礼物的操作没有写入本地模拟状态。");
            return;
        }
        if sent {
            let Some(g) = self.gift_client.sent_detail.clone() else { return };
            let product = CATALOG.get(g.product_id as usize).map(|p| p.name).unwrap_or("商品");
            self.set_text(cx, ids!(og_title), &format!("{} · 送给 {}", product, g.recipient_name));
            self.set_text(cx, ids!(og_summary), &format!("原订单金额 {} · 礼物状态 {} · {}。收礼人的换购、折现和地址不会显示在这里。", yuan(g.price_cents), g.state, g.delivery_text()));
            self.show(cx, ids!(og_logistics), true);
            self.set_text(cx, ids!(og_logistics_head), "送达状态");
            self.set_text(cx, ids!(og_logistics_text), &format!("物流是否签收：{}\n收礼人是否确认收货：{}",
                if g.carrier_delivered { "是" } else { "否" }, if g.recipient_confirmed { "是" } else { "否" }));
            self.show(cx, ids!(og_confirm), false);
            return;
        }
        let Some(g) = self.gift_client.received_detail.clone() else { return };
        let revealed = g.product_id.is_some();
        let title = g.product_id.and_then(|i| CATALOG.get(i as usize)).map(|p| p.name)
            .unwrap_or("一份神秘礼物");
        self.set_text(cx, ids!(og_title), title);
        let summary = if revealed {
            format!("状态：{} · {} · 来自 {}{}", g.state,
                g.price_cents.map(yuan).unwrap_or_default(), g.sender_name.as_deref().unwrap_or("神秘的朋友"),
                if g.message.is_empty() { String::new() } else { format!("\n寄语：{}", g.message) })
        } else { format!("状态：{} · 解谜方式：{} · 剩余 {} 次机会", g.state, g.unlock_kind, g.attempts_left) };
        self.set_text(cx, ids!(og_summary), &summary);
        self.show(cx, ids!(og_clue), !g.clue.is_empty() || !g.contract_text.is_empty());
        self.set_text(cx, ids!(og_clue), &format!("{}{}", if g.clue.is_empty() { String::new() } else { format!("线索：{}", g.clue) },
            if g.contract_text.is_empty() { String::new() } else { format!("\n附加契约：{}", g.contract_text) }));
        self.show(cx, ids!(og_open), g.state == "sealed");
        self.show(cx, ids!(og_answer), g.state == "opened");
        self.show(cx, ids!(og_answer_btn), g.state == "opened");
        let accept = g.state == "revealed";
        self.show(cx, ids!(og_agree), accept && !g.contract_text.is_empty());
        self.show(cx, ids!(og_ship_name), accept && g.physical);
        self.show(cx, ids!(og_ship_phone), accept && g.physical);
        self.show(cx, ids!(og_ship_address), accept && g.physical);
        self.show(cx, ids!(og_accept), accept);
        if let Some(s) = self.gift_client.shipment.clone() {
            self.show(cx, ids!(og_logistics), true);
            self.set_text(cx, ids!(og_logistics_head), "我的物流详情");
            self.set_text(cx, ids!(og_logistics_text), &format!("{} · 单号 {}\n收件人：{} · {}\n地址：{}\n物流签收：{} · 我已确认：{}\n{}",
                s.carrier, s.tracking_number, s.recipient_name, s.recipient_phone, s.recipient_address,
                if s.carrier_delivered { "是" } else { "否" }, if s.recipient_confirmed { "是" } else { "否" }, s.events.join("\n")));
            self.show(cx, ids!(og_confirm), s.carrier_delivered && !s.recipient_confirmed);
        } else if let Some(code) = g.voucher_code {
            self.show(cx, ids!(og_logistics), true);
            self.set_text(cx, ids!(og_logistics_head), "电子券");
            self.set_text(cx, ids!(og_logistics_text), &format!("券码：{code}"));
            self.show(cx, ids!(og_confirm), false);
        }
    }

    // ---- 拆礼页 ----

    /// 打开一份收到的礼物：第一次打开时「待拆」→「解谜中」（直接领取则直接揭晓）。
    fn open_received(&mut self, cx: &mut Cx, id: u64) {
        let Some(st) = self.state.open(id, today_days()) else { return };
        self.open_gift = Some(id);
        self.open_preview = false;
        self.open_wrong = None;
        self.set_text(cx, ids!(od_input), "");
        let stage = match st {
            GiftState::Sealed | GiftState::Opened => Stage::Decrypt,
            GiftState::Revealed => Stage::Reveal,
            _ => Stage::Done,
        };
        self.after_data_change(cx);
        self.overlay = Some(Overlay::Open);
        self.enter_stage(cx, stage);
        self.open_overlay(cx, Overlay::Open);
    }

    /// 「以 TA 的视角看看」：同一张拆礼页，只读。
    fn open_preview(&mut self, cx: &mut Cx) {
        let Some(id) = self.card_gift else { return };
        self.open_gift = Some(id);
        self.open_preview = true;
        self.open_wrong = None;
        self.set_text(cx, ids!(od_input), "");
        self.overlay = Some(Overlay::Open);
        self.enter_stage(cx, Stage::Decrypt);
        self.open_overlay(cx, Overlay::Open);
    }

    fn enter_stage(&mut self, cx: &mut Cx, stage: Stage) {
        self.stage = stage;
        self.scroll_top(cx, Overlay::Open);
        self.open_err = None;
        let s = &self.state.settings;
        let ship = (s.ship_name.clone(), s.ship_phone.clone(), s.ship_addr.clone());
        match stage {
            Stage::Accept => {
                self.accept_agree = false;
                self.set_text(cx, ids!(oa_name), &ship.0);
                self.set_text(cx, ids!(oa_phone), &ship.1);
                self.set_text(cx, ids!(oa_addr), &ship.2);
            }
            Stage::Swap => {
                self.swap_exchange = false;
                self.swap_pick = None;
                self.set_text(cx, ids!(ow_name), &ship.0);
                self.set_text(cx, ids!(ow_phone), &ship.1);
                self.set_text(cx, ids!(ow_addr), &ship.2);
            }
            _ => {}
        }
        self.refresh_open(cx);
        self.refresh_topbar(cx);
        self.redraw(cx);
    }

    fn refresh_open(&mut self, cx: &mut Cx) {
        let Some(g) = self.open_gift.and_then(|id| self.state.gift(id)).cloned() else {
            return;
        };
        // 状态被别处推进过（过期扫描、模拟器），阶段跟着状态走。
        if !self.open_preview {
            let st = g.state();
            let want = match st {
                GiftState::Sealed | GiftState::Opened => Some(Stage::Decrypt),
                GiftState::Revealed if self.stage == Stage::Decrypt || self.stage == Stage::Done => {
                    Some(Stage::Reveal)
                }
                s if s.is_terminal() => Some(Stage::Done),
                _ => None,
            };
            if let Some(w) = want {
                self.stage = w;
            }
        }
        for (j, id) in STAGE_VIEWS.iter().enumerate() {
            self.show(cx, &[*id], j == self.stage.index());
        }
        self.show(cx, ids!(op_preview), self.open_preview);
        match self.stage {
            Stage::Decrypt => self.refresh_decrypt(cx, &g),
            Stage::Reveal => self.refresh_reveal(cx, &g),
            Stage::Accept => self.refresh_accept(cx, &g),
            Stage::Swap => self.refresh_swap(cx, &g),
            Stage::Done => self.refresh_done(cx, &g),
        }
        self.redraw(cx);
    }

    fn refresh_decrypt(&mut self, cx: &mut Cx, g: &Gift) {
        let u = g.unlock();
        let preview = self.open_preview;
        self.set_text(
            cx,
            ids!(od_head),
            if preview { "TA 打开礼卡会看到这些" } else { "你收到一份神秘礼物" },
        );
        let badge = if u == Unlock::Free {
            u.label().to_string()
        } else {
            format!("{} · {} 次机会", u.label(), MAX_ATTEMPTS)
        };
        self.set_text(cx, ids!(od_badge), &badge);
        let has_clue = !g.clue.is_empty();
        self.show(cx, ids!(od_card), has_clue);
        self.set_text(cx, ids!(od_ctitle), u.clue_title());
        self.set_text(cx, ids!(od_clue), &g.clue);

        let guess = u == Unlock::GuessWho;
        self.show(cx, ids!(od_cands), guess);
        self.cand_names = if guess { self.state.guess_candidates(g) } else { Vec::new() };
        for (j, id) in CAND_BTNS.iter().enumerate() {
            let name = self.cand_names.get(j).cloned();
            self.show(cx, &[*id], name.is_some());
            if let Some(n) = name {
                self.set_text(cx, &[*id], &n);
            }
        }
        let free = u == Unlock::Free;
        self.show(cx, ids!(od_in_box), !free);
        let ph = match u {
            Unlock::GuessWho => "输入 TA 的称呼",
            Unlock::Question => "输入答案",
            _ => "输入暗号",
        };
        self.view.text_input(cx, ids!(od_input)).set_empty_text(cx, ph.to_string());
        self.view.text_input(cx, ids!(od_input)).set_is_read_only(cx, preview);
        self.show(cx, ids!(od_wrong), self.open_wrong.is_some());
        if let Some(w) = self.open_wrong.clone() {
            self.set_text(cx, ids!(od_wrong), &w);
        }
        let left = if free {
            "打开即可领取".to_string()
        } else if preview {
            format!("共 {MAX_ATTEMPTS} 次机会")
        } else {
            format!("还有 {} 次机会", g.attempts_left())
        };
        self.set_text(cx, ids!(od_left), &left);
        self.show(cx, ids!(od_submit), !preview);
        self.show(cx, ids!(od_note), !free);
        let tip = !preview && !g.demo_tip.is_empty();
        self.show(cx, ids!(od_tip), tip);
        if tip {
            self.set_text(cx, ids!(od_tip), &format!("演示提示：{}", g.demo_tip));
        }
    }

    fn refresh_reveal(&mut self, cx: &mut Cx, g: &Gift) {
        let sender = g.shown_sender();
        let head = if g.unlock() == Unlock::Free {
            format!("来自{sender}的礼物")
        } else if g.solved {
            format!("答对了，是{sender}！")
        } else {
            "礼物拆开啦（TA 是谁，先保密）".to_string()
        };
        self.set_text(cx, ids!(or_head), &head);
        self.fill_gift_card(cx, g);
        self.set_text(
            cx,
            ids!(or_value),
            &format!("礼物价值 {} · 不合心意可以换一份，或折成余额", yuan(g.price)),
        );
        self.show(cx, ids!(or_pact), g.has_contract());
        self.set_text(cx, ids!(or_ptext), &format!("收下即答应：{}", g.contract));
    }

    fn fill_gift_card(&mut self, cx: &mut Cx, g: &Gift) {
        let it = g.final_item();
        self.set_img(cx, ids!(gc_img), Some(final_index(g)));
        self.set_text(cx, ids!(gc_kind), &format!("{} · {}", kind_text(it), it.cat.label()));
        self.set_text(cx, ids!(gc_name), it.name);
        self.set_text(cx, ids!(gc_spec), it.spec);
        let from = if g.message.is_empty() {
            format!("来自{}", spaced(&g.shown_sender()))
        } else {
            format!("来自{}：「{}」", spaced(&g.shown_sender()), g.message)
        };
        self.set_text(cx, ids!(gc_from), &from);
    }

    fn refresh_accept(&mut self, cx: &mut Cx, g: &Gift) {
        let it = g.catalog();
        self.set_text(cx, ids!(oa_title), &format!("收下「{}」", it.name));
        self.show(cx, ids!(oa_agree), g.has_contract());
        let agree = self.accept_agree;
        self.set_switch(
            cx,
            ids!(oa_agree),
            &format!("我同意：{}", g.contract),
            "收下就要答应；不想答应可以换购或折现",
            agree,
        );
        self.show(cx, ids!(oa_ship), it.physical);
        self.show(cx, ids!(oa_ev), !it.physical);
        self.show(cx, ids!(oa_err), self.open_err.is_some());
        if let Some(e) = self.open_err {
            self.set_text(cx, ids!(oa_err), e);
        }
    }

    fn refresh_swap(&mut self, cx: &mut Cx, g: &Gift) {
        self.set_chip_group(cx, &SWAP_SEGS, self.swap_exchange as usize);
        let ex = self.swap_exchange;
        self.show(cx, ids!(ow_cashbox), !ex);
        self.show(cx, ids!(ow_list), ex);
        if !ex {
            let (f, refund) = cashout_quote(g.price);
            self.set_text(cx, ids!(ow_l1), &format!("礼物价值 {}", yuan(g.price)));
            self.set_text(
                cx,
                ids!(ow_l2),
                &format!("手续费 {}（{}%，至少 ¥1）", yuan(f), CASHOUT_FEE_PCT),
            );
            self.set_text(cx, ids!(ow_l3), &format!("折成余额 {}", yuan(refund)));
            self.set_text(cx, ids!(ow_calc), "余额只在礼遇内使用，可以拿来回一份礼。");
            self.show(cx, ids!(ow_ship), false);
            self.set_text(cx, ids!(ow_ok), &format!("确认折现（到账 {}）", yuan(refund)));
        } else {
            let (f, credit) = exchange_credit(g.price);
            self.set_text(
                cx,
                ids!(ow_credit),
                &format!(
                    "可抵扣 {}（价值 {} − 手续费 {}，{}%）。多退少补，补差先用余额。",
                    yuan(credit),
                    yuan(g.price),
                    yuan(f),
                    EXCHANGE_FEE_PCT
                ),
            );
            let opts = self.state.exchange_options(g);
            self.swap_rows = opts.iter().map(|(i, _)| *i).collect();
            for (j, row) in SWAP_ROWS.iter().enumerate() {
                let Some((i, diff)) = opts.get(j).copied() else {
                    self.show(cx, &[*row], false);
                    continue;
                };
                self.show(cx, &[*row], true);
                let it = item(i);
                self.set_img(cx, &[*row, live_id!(ch_img)], Some(i));
                self.set_text(cx, &[*row, live_id!(ch_name)], it.name);
                self.set_text(cx, &[*row, live_id!(ch_sub)], &item_sub(it));
                let (dt, dc) = if diff >= 0 {
                    (format!("退 {}", yuan(diff)), self.pal.good)
                } else {
                    (format!("补 {}", yuan(-diff)), self.pal.ink_2)
                };
                self.set_text(cx, &[*row, live_id!(ch_diff)], &dt);
                self.tint_text(cx, &[*row, live_id!(ch_diff)], dc);
                self.show(cx, &[*row, live_id!(ch_tick)], self.swap_pick == Some(i));
            }
            let pick = self.swap_pick;
            self.show(cx, ids!(ow_ship), pick.map_or(false, |i| item(i).physical));
            let calc = match pick.and_then(|p| opts.iter().find(|(i, _)| *i == p).copied()) {
                Some((i, d)) if d >= 0 => format!("换成「{}」，差价 {} 退回余额", item(i).name, yuan(d)),
                Some((i, d)) => format!("换成「{}」，还需补 {}（余额优先）", item(i).name, yuan(-d)),
                None => "选一件想要的".to_string(),
            };
            self.set_text(cx, ids!(ow_calc), &calc);
            self.set_text(cx, ids!(ow_ok), "确认换购");
        }
        self.show(cx, ids!(ow_void), g.has_contract());
        self.show(cx, ids!(ow_err), self.open_err.is_some());
        if let Some(e) = self.open_err {
            self.set_text(cx, ids!(ow_err), e);
        }
    }

    fn refresh_done(&mut self, cx: &mut Cx, g: &Gift) {
        let it = g.final_item();
        let deliver = if it.physical {
            format!("「{}」会寄到：{}", it.name, g.ship_addr)
        } else {
            format!("「{}」的券码：{}", it.name, g.voucher)
        };
        let (title, text, hint, back) = match g.state() {
            GiftState::Accepted => {
                let mut t = deliver;
                if g.has_contract() {
                    t.push_str(&format!("\n契约已生效：{}。记得兑现哦。", g.contract));
                }
                ("收下啦".to_string(), t, "礼尚往来：也给 TA 回一份小惊喜？", true)
            }
            GiftState::Exchanged => {
                let mut t = deliver;
                if g.refund > 0 {
                    t.push_str(&format!("\n差价 {} 已退回余额。", yuan(g.refund)));
                }
                (format!("换成了「{}」", it.name), t, "礼尚往来：也给 TA 回一份小惊喜？", true)
            }
            GiftState::CashedOut => (
                format!("已折成 {} 余额", yuan(g.refund)),
                "余额只在礼遇内使用，送礼时可以直接抵扣。".to_string(),
                "用刚才变现的余额，给 TA 回送一份「反击礼物」吧！",
                true,
            ),
            _ => (
                "这份礼物已经退回了".to_string(),
                g.status_text(today_days()),
                "",
                false,
            ),
        };
        self.set_text(cx, ids!(odn_title), &title);
        self.set_text(cx, ids!(odn_text), &text);
        self.show(cx, ids!(odn_hint), !hint.is_empty());
        self.set_text(cx, ids!(odn_hint_t), hint);
        self.show(cx, ids!(odn_return), back);
        let label = if g.state() == GiftState::CashedOut { "用余额回礼" } else { "给 TA 回一份礼" };
        self.set_text(cx, ids!(odn_return), label);
    }

    fn submit_answer(&mut self, cx: &mut Cx) {
        let Some(id) = self.open_gift else { return };
        let guess = self.input_text(cx, ids!(od_input));
        match self.state.submit_answer(id, &guess, today_days()) {
            AnswerOutcome::Empty => self.open_wrong = Some("先写下你的答案".into()),
            AnswerOutcome::Wrong { left } => {
                self.open_wrong = Some(if left == 1 { "不对哦，最后一次机会了".into() } else { "不对哦，再想想".into() });
                self.set_text(cx, ids!(od_input), "");
            }
            AnswerOutcome::Right => {
                self.open_wrong = None;
                self.after_data_change(cx);
                self.enter_stage(cx, Stage::Reveal);
                self.toast(cx, "答对了！");
                return;
            }
            AnswerOutcome::Exhausted => {
                self.open_wrong = None;
                self.after_data_change(cx);
                self.enter_stage(cx, Stage::Reveal);
                self.toast(cx, "机会用完了，礼物照样拆开");
                return;
            }
            AnswerOutcome::NotOpen => {}
        }
        self.after_data_change(cx);
        self.refresh_open(cx);
    }

    fn submit_accept(&mut self, cx: &mut Cx) {
        let Some(id) = self.open_gift else { return };
        let f = AcceptForm {
            agree: self.accept_agree,
            name: self.input_text(cx, ids!(oa_name)),
            phone: self.input_text(cx, ids!(oa_phone)),
            addr: self.input_text(cx, ids!(oa_addr)),
        };
        match self.state.accept(id, &f, today_days()) {
            Ok(()) => {
                self.after_data_change(cx);
                self.enter_stage(cx, Stage::Done);
            }
            Err(e) => {
                self.open_err = Some(e);
                self.refresh_open(cx);
            }
        }
    }

    fn submit_swap(&mut self, cx: &mut Cx) {
        let Some(id) = self.open_gift else { return };
        let today = today_days();
        let res = if !self.swap_exchange {
            self.state.cash_out(id, today).map(|_| ())
        } else if let Some(pick) = self.swap_pick {
            let f = AcceptForm {
                agree: false,
                name: self.input_text(cx, ids!(ow_name)),
                phone: self.input_text(cx, ids!(ow_phone)),
                addr: self.input_text(cx, ids!(ow_addr)),
            };
            self.state.exchange(id, pick, &f, today).map(|_| ())
        } else {
            Err("先选一件要换的礼物")
        };
        match res {
            Ok(()) => {
                self.after_data_change(cx);
                self.refresh_gift(cx);
                self.enter_stage(cx, Stage::Done);
            }
            Err(e) => {
                self.open_err = Some(e);
                self.refresh_open(cx);
            }
        }
    }

    /// 完成页「回一份礼」：知道是谁就预填 TA；折现的钱够买哪件就先选哪件。
    fn start_return(&mut self, cx: &mut Cx) {
        let Some(g) = self.open_gift.and_then(|id| self.state.gift(id)).cloned() else {
            return;
        };
        let peer = if g.identity_known { g.peer_name() } else { String::new() };
        let cashed = g.state() == GiftState::CashedOut;
        let budget = if cashed { g.refund } else { self.state.balance() };
        let pick = best_item_within(budget).unwrap_or_else(cheapest_item);
        let banner = if cashed {
            format!("回礼 · 刚变现的 {} 可用", yuan(g.refund))
        } else {
            "回礼 · 礼尚往来".to_string()
        };
        self.send_from = 1;
        self.box_sent = false;
        self.open_send(cx, pick, peer, Some(banner));
        self.draft.use_balance = true;
        self.refresh_send(cx);
    }

    // ---- 送出详情 ----

    fn open_sent(&mut self, cx: &mut Cx, id: u64) {
        self.sent_gift = Some(id);
        self.withdraw_armed = false;
        self.refresh_sent(cx);
        self.open_overlay(cx, Overlay::Sent);
    }

    fn refresh_sent(&mut self, cx: &mut Cx) {
        let Some(g) = self.sent_gift.and_then(|id| self.state.gift(id)).cloned() else {
            return;
        };
        let today = today_days();
        self.set_img(cx, ids!(ss_img), Some(g.item));
        self.set_text(cx, ids!(ss_title), &format!("{} · 送给{}", g.catalog().name, spaced(&g.shown_recipient())));
        self.set_text(cx, ids!(ss_price), &yuan(g.price));
        self.set_text(cx, ids!(ss_state), &g.status_text(today));
        let tc = self.tone_color(g.tone());
        self.tint_text(cx, ids!(ss_state), tc);

        // 时间线：已发生的实心，接下来的空心（不写日期）。
        let mut steps: Vec<(Option<i64>, String)> =
            g.timeline(today).into_iter().map(|(d, t)| (Some(d), t)).collect();
        let mut future: Vec<String> = Vec::new();
        if g.is_scheduled(today) {
            future.push(format!("{} 礼卡送到 TA 手里", md_cn(g.sent_on)));
        }
        let rest: &[&str] = match g.state() {
            GiftState::Sealed => &["等 TA 打开礼卡", "揭晓", "TA 决定收下 / 换购 / 折现"],
            GiftState::Opened => &["揭晓", "TA 决定收下 / 换购 / 折现"],
            GiftState::Revealed => &["TA 决定收下 / 换购 / 折现"],
            _ => &[],
        };
        future.extend(rest.iter().map(|f| f.to_string()));
        for f in future {
            steps.push((None, f));
        }
        for (j, row) in STEP_ROWS.iter().enumerate() {
            let Some((day, text)) = steps.get(j).cloned() else {
                self.show(cx, &[*row], false);
                continue;
            };
            self.show(cx, &[*row], true);
            self.show(cx, &[*row, live_id!(tl_on)], day.is_some());
            self.show(cx, &[*row, live_id!(tl_off)], day.is_none());
            self.set_text(cx, &[*row, live_id!(tl_date)], &day.map(fmt_md).unwrap_or_default());
            self.set_text(cx, &[*row, live_id!(tl_text)], &text);
            let c = if day.is_some() { self.pal.ink } else { self.pal.ink_3 };
            self.tint_text(cx, &[*row, live_id!(tl_text)], c);
        }

        let play = if g.clue.is_empty() {
            format!("玩法：{}", g.unlock().label())
        } else {
            format!("玩法：{} · 「{}」", g.unlock().label(), g.clue)
        };
        self.set_text(cx, ids!(ss_play), &play);
        let pact = if g.has_contract() {
            format!("契约：{}", g.contract)
        } else {
            "没有附加契约".to_string()
        };
        self.set_text(cx, ids!(ss_pact), &pact);
        self.show(cx, ids!(ss_msg), !g.message.is_empty());
        self.set_text(cx, ids!(ss_msg), &format!("寄语：{}", g.message));
        // 认领的心愿：哪张单子上的哪一件。
        let wish = g.wish_id.and_then(|w| self.state.wishlist(w)).map(|l| {
            let what = l.items.iter().find(|x| x.gift_id == g.id).map(|x| x.title()).unwrap_or_default();
            if what.is_empty() {
                format!("心愿单：{}", l.title)
            } else {
                format!("心愿单：{} · 认领的是「{}」", l.title, what)
            }
        });
        self.show(cx, ids!(ss_wish), wish.is_some());
        self.set_text(cx, ids!(ss_wish), &wish.unwrap_or_default());

        let sealed = g.state() == GiftState::Sealed;
        self.show(cx, ids!(ss_withdraw), sealed && !self.withdraw_armed);
        self.show(cx, ids!(ss_confirm), sealed && self.withdraw_armed);
        self.set_text(
            cx,
            ids!(ss_ctext),
            &format!("撤回后礼卡失效，{} 全额退回余额。", yuan(g.price)),
        );
        self.show(cx, ids!(ss_sim), !g.state().is_terminal());
        self.show(cx, ids!(ss_view), !g.state().is_terminal());
        self.show(cx, ids!(ss_copy), !g.state().is_terminal());
        self.redraw(cx);
    }

    // ---- 契约 ----

    fn refresh_pacts(&mut self, cx: &mut Cx) {
        let today = today_days();
        self.set_chip_group(cx, &PACT_SEGS, self.pact_theirs as usize);
        let mine = !self.pact_theirs;
        let pacts: Vec<Pact> = self.state.pacts_of(mine).into_iter().cloned().collect();
        self.pact_rows = pacts.iter().map(|p| p.id).collect();
        for (j, row) in PACT_ROWS.iter().enumerate() {
            let Some(p) = pacts.get(j) else {
                self.show(cx, &[*row], false);
                continue;
            };
            self.show(cx, &[*row], true);
            let who = if p.peer.is_empty() { "TA".to_string() } else { p.peer.clone() };
            let who = if mine { format!("你答应{who}") } else { format!("{who}答应你") };
            self.set_text(cx, &[*row, live_id!(pc_text)], &p.text);
            self.set_text(
                cx,
                &[*row, live_id!(pc_sub)],
                &format!("{who} · {}起 · {}", fmt_md(p.made_on), p.due_text(today)),
            );
            let pending = p.state() == PactState::Pending;
            self.show(cx, &[*row, live_id!(pc_acts)], pending);
            self.set_text(
                cx,
                &[*row, live_id!(pc_done)],
                if mine { "标记已兑现" } else { "TA 兑现了" },
            );
            self.show(cx, &[*row, live_id!(pc_nudge)], !mine);
            self.show(cx, &[*row, live_id!(pc_waive)], !mine);
            self.set_text(
                cx,
                &[*row, live_id!(pc_nudge)],
                if p.nudged_on == today { "今天提醒过了" } else { "提醒 TA" },
            );
        }
        let (text, action) = if mine {
            ("还没有答应过别人的契约", "")
        } else {
            ("还没有人答应你契约", "送礼时附一个")
        };
        self.apply_list_state(cx, ids!(pt_empty), text, action, pacts.len());
    }

    // ---- 熟人 ----

    fn refresh_contacts(&mut self, cx: &mut Cx) {
        let total = self.state.contacts.len();
        self.contact_page = self
            .contact_page
            .min(total.saturating_sub(1) / CONTACT_ROWS.len());
        let contacts: Vec<ContactLocal> = self
            .state
            .contacts
            .iter()
            .skip(self.contact_page * CONTACT_ROWS.len())
            .take(CONTACT_ROWS.len())
            .cloned()
            .collect();
        self.show(cx, ids!(ct_prev), self.contact_page > 0);
        self.show(
            cx,
            ids!(ct_next),
            (self.contact_page + 1) * CONTACT_ROWS.len() < total,
        );
        self.contact_rows = contacts.iter().map(|c| c.id).collect();
        let today = today_days();
        let friends = self.state.friend_wishlists(today);
        self.contact_wish = contacts
            .iter()
            .map(|c| {
                friends
                    .iter()
                    .find(|w| w.owner == c.label && w.is_open(today))
                    .map(|w| w.id)
            })
            .collect();
        self.set_text(cx, ids!(ct_head), &format!("我的联系人（{}）", total));
        self.set_text(
            cx,
            ids!(ca_btn),
            if self.editing_contact.is_some() {
                "保存"
            } else {
                "添加"
            },
        );
        self.show(cx, ids!(ct_cancel_edit), self.editing_contact.is_some());
        for (j, row) in CONTACT_ROWS.iter().enumerate() {
            let Some(c) = contacts.get(j) else {
                self.show(cx, &[*row], false);
                continue;
            };
            self.show(cx, &[*row], true);
            let initial: String = c
                .label
                .chars()
                .next()
                .map(|ch| ch.to_string())
                .unwrap_or_default();
            self.set_text(cx, &[*row, live_id!(cr_initial)], &initial);
            self.set_text(cx, &[*row, live_id!(cr_name)], &c.label);
            // 有进行中的心愿单就先说心愿单（比送过几份更值得一眼看到），按钮也换成「看心愿单」。
            let sub = if !contacts::choices(c).is_empty() {
                contacts::choices(c)
                    .iter()
                    .map(|(_, v)| v.clone())
                    .collect::<Vec<_>>()
                    .join(" · ")
            } else {
                match self.state.wish_hint(&c.label, today) {
                    Some(h) => h,
                    None => {
                        let (sent, recv) = self.state.gift_counts(&c.label);
                        format!("送过 {sent} 份 · 收到 {recv} 份")
                    }
                }
            };
            self.set_text(cx, &[*row, live_id!(cr_sub)], &sub);
            let has_wish = self.contact_wish.get(j).copied().flatten().is_some();
            let _ = has_wish;
            self.set_text(cx, &[*row, live_id!(cr_send)], "送礼");
        }
        self.apply_list_state(
            cx,
            ids!(ct_empty),
            "还没有熟人",
            "从本机导入",
            contacts.len(),
        );
        let more = total.saturating_sub(CONTACT_ROWS.len());
        self.show(cx, ids!(ct_more), more > 0);
        self.set_text(
            cx,
            ids!(ct_more),
            &format!("第 {} 页，共 {} 位联系人", self.contact_page + 1, total),
        );
        self.show(cx, ids!(ca_err), self.add_error.is_some());
        if let Some(e) = self.add_error {
            self.set_text(cx, ids!(ca_err), e.text());
        }
    }

    fn add_contact(&mut self, cx: &mut Cx) {
        let label = self.input_text(cx, ids!(ca_input));
        let phones = self.input_text(cx, ids!(ct_phone));
        let emails = self.input_text(cx, ids!(ct_email));
        match contacts::entry(&label, &phones, &emails) {
            Ok(mut c) => {
                if let Some(id) = self.editing_contact.take() {
                    c.id = id;
                    if let Some(old) = self.state.contacts.iter_mut().find(|v| v.id == id) {
                        *old = c;
                    }
                } else {
                    contacts::merge(&mut self.state.contacts, vec![c]);
                }
                self.state.save();
                self.add_error = None;
                for id in [live_id!(ca_input), live_id!(ct_phone), live_id!(ct_email)] {
                    self.set_text(cx, &[id], "");
                }
                self.toast(cx, "联系人已保存");
            }
            Err(e) => {
                self.toast(cx, e);
                return;
            }
        }
        self.refresh_contacts(cx);
    }

    fn stage_contacts(&mut self, cx: &mut Cx, import: contacts::Import) {
        let duplicates = import
            .contacts
            .iter()
            .filter(|c| {
                self.state
                    .contacts
                    .iter()
                    .any(|old| contacts::same_address(old, c))
            })
            .count();
        let names = import
            .contacts
            .iter()
            .take(12)
            .map(|c| {
                format!(
                    "{} · {}",
                    c.label,
                    contacts::choices(c)
                        .iter()
                        .map(|(_, v)| v.clone())
                        .collect::<Vec<_>>()
                        .join(" / ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.set_text(
            cx,
            ids!(ct_preview_text),
            &format!(
                "可导入 {} 位；无效 {} 行；已有 {} 位将合并。\n{}{}",
                import.contacts.len(),
                import.invalid,
                duplicates,
                names,
                if import.contacts.len() > 12 {
                    "\n更多联系人将在确认后导入。"
                } else {
                    ""
                }
            ),
        );
        self.pending_contacts = Some(import);
        self.show(cx, ids!(ct_preview), true);
        self.redraw(cx);
    }

    fn import_local(&mut self, cx: &mut Cx) {
        if self.contact_import_receiver.is_some() {
            self.toast(cx, "正在读取系统通讯录，请稍候");
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.contact_import_receiver = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(contacts::system_contacts());
        });
        self.contact_import_poll = cx.start_interval(0.1);
        self.toast(cx, "正在读取系统通讯录；系统可能请求访问权限");
    }

    fn import_vcard(&mut self, cx: &mut Cx) {
        cx.open_select_file_dialog(
            FileDialog::new()
                .set_id(live_id!(contact_import))
                .set_title("导入联系人（vCard / CSV）".into())
                .add_filter("联系人".into(), vec!["vcf".into(), "csv".into()]),
        );
    }

    // ---- 我 / 钱包 / 设置 ----

    fn refresh_me(&mut self, cx: &mut Cx) {
        let bal = yuan(self.state.balance());
        self.set_text(cx, ids!(me_bal_v), &bal);
        self.set_text(cx, ids!(ag2_v), &bal);
        let sent = self.state.sent().len();
        let recv = self.state.received(today_days()).len();
        let done = self.state.pacts.iter().filter(|p| p.state() == PactState::Done).count();
        self.set_text(cx, ids!(ms_sent_v), &sent.to_string());
        self.set_text(cx, ids!(ms_recv_v), &recv.to_string());
        self.set_text(cx, ids!(ms_pact_v), &done.to_string());
        self.my_wish_row(cx, ids!(row_wish));
        let n = self.state.ledger.len();
        self.set_row(cx, ids!(row_wallet), "钱包与流水", "折现、退差、退款都记在这里", &format!("{n} 笔"));
        let nick = self.state.settings.nickname.clone();
        let display = if self.profile.display_name.is_empty() { nick.clone() } else { self.profile.display_name.clone() };
        self.set_row(cx, ids!(row_profile), "账户资料", "名字、手机号、邮箱、头像和收货地址", &display);
        self.set_row(cx, ids!(row_settings), "设置", "深浅、称呼、通知、数据", &nick);
        self.set_row(cx, ids!(row_about), "关于礼遇", "重看开场三屏", "");
        // ---- #7 账号/设置分层导航:desktop 左分类+右详情 / mobile 列表分屏 ----
        self.refresh_avatar(cx);
        self.refresh_account_nav(cx);
        // AI 草稿闸:有待本人确认的草稿就显示确认条(商品/寄语/确认/取消)。
        let draft = self.draft_pending.clone();
        let show_draft = draft.as_ref().is_some_and(|d| d.status == ai_draft::DraftStatus::AwaitingConfirm);
        self.show(cx, ids!(me_draft), show_draft);
        if show_draft {
            let d = draft.unwrap();
            let it = crate::data::item(d.item_index);
            self.set_text(cx, ids!(me_draft_t), &format!("{} · {}", it.name, crate::data::yuan(it.price)));
            let note = if d.note.is_empty() { "（无寄语）".to_string() } else { format!("寄语:{}", d.note) };
            self.set_text(cx, ids!(me_draft_note), &note);
        }
    }

    fn open_account_section(&mut self, cx: &mut Cx, section: AccountSection) {
        if self.account_has_unsaved_changes(cx) {
            self.toast(cx, "有未保存的修改，请保存或取消");
            self.refresh_account_nav(cx);
            return;
        }
        if self.account_edit.is_some() {
            self.cancel_account_edit(cx);
        }
        self.tab = 4;
        self.account_section = Some(section);
        self.profile_err = None;
        self.nick_err = None;
        self.account_nav = Some(account_nav::AccountNavModel::new());
        self.overlay = None;
        self.back_stack.clear();
        self.send_from = 4;
        match section {
            AccountSection::Wishes => self.open_wishes(cx),
            AccountSection::Wallet => {
                self.refresh_wallet(cx);
                self.open_overlay(cx, Overlay::Wallet);
            }
            AccountSection::Received | AccountSection::Sent => {
                let sent = section == AccountSection::Sent;
                let _ = self.gift_client.refresh();
                self.refresh_account_gifts(cx, sent);
                self.open_overlay(
                    cx,
                    if sent {
                        Overlay::SentGifts
                    } else {
                        Overlay::ReceivedGifts
                    },
                );
            }
            AccountSection::Drafts => {
                self.refresh_me(cx);
                self.update_page_visibility(cx);
            }
        }
        self.refresh_account_nav(cx);
    }

    /// Render dedicated account lists with independent widget IDs and row mappings.
    /// Sharing the role-specific API client does not couple these pages to page_box.
    fn refresh_account_gifts(&mut self, cx: &mut Cx, sent: bool) {
        let rows = if sent {
            self.gift_client
                .outbox
                .iter()
                .map(|g| {
                    let title = CATALOG
                        .get(g.product_id as usize)
                        .map(|p| p.name)
                        .unwrap_or("商品");
                    (
                        g.id,
                        Some(g.product_id),
                        format!("{} · 送给 {}", title, g.recipient_name),
                        "已送出的礼物".to_string(),
                        format!(
                            "{} · 物流签收 {} · TA 确认 {}",
                            g.state,
                            if g.carrier_delivered { "是" } else { "否" },
                            if g.recipient_confirmed { "是" } else { "否" }
                        ),
                    )
                })
                .collect::<Vec<_>>()
        } else {
            self.gift_client
                .inbox
                .iter()
                .map(|g| {
                    let title = g
                        .product_id
                        .and_then(|i| CATALOG.get(i as usize))
                        .map(|p| p.name)
                        .unwrap_or("一份神秘礼物");
                    (
                        g.id,
                        g.product_id,
                        title.to_string(),
                        if g.product_id.is_some() {
                            "已揭晓的礼物".into()
                        } else {
                            "拆开前不展示商品和送礼人".into()
                        },
                        g.state.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let rows = if self.gift_client.online {
            rows
        } else {
            Vec::new()
        };
        if sent {
            self.sent_gift_rows = rows.iter().map(|r| r.0).collect();
        } else {
            self.received_gift_rows = rows.iter().map(|r| r.0).collect();
        }
        let (widgets, empty, more, note) = if sent {
            (
                &SENT_GIFT_ROWS,
                live_id!(sent_gift_empty),
                live_id!(sent_gift_more),
                live_id!(sent_gift_note),
            )
        } else {
            (
                &RECEIVED_GIFT_ROWS,
                live_id!(received_gift_empty),
                live_id!(received_gift_more),
                live_id!(received_gift_note),
            )
        };
        self.set_text(
            cx,
            &[note],
            if sent {
                "送出的礼物 · 按送礼人权限显示"
            } else {
                "收到的礼物 · 按收礼人权限显示"
            },
        );
        for (j, row) in widgets.iter().enumerate() {
            self.show(cx, &[*row], j < rows.len());
            if let Some((_, image, title, subtitle, state)) = rows.get(j) {
                self.set_img(cx, &[*row, live_id!(bx_img)], *image);
                self.set_text(cx, &[*row, live_id!(bx_title)], title);
                self.set_text(cx, &[*row, live_id!(bx_sub)], subtitle);
                self.set_text(cx, &[*row, live_id!(bx_state)], state);
            }
        }
        self.apply_list_state(
            cx,
            &[empty],
            if !self.gift_client.online {
                "礼物加载失败"
            } else if sent {
                "还没有送出的礼物"
            } else {
                "还没有收到的礼物"
            },
            if self.gift_client.online {
                ""
            } else {
                "请检查服务器连接后刷新"
            },
            rows.len(),
        );
        let overflow = rows.len().saturating_sub(widgets.len());
        self.show(cx, &[more], overflow > 0);
        self.set_text(cx, &[more], &format!("还有 {overflow} 份较早的礼物未显示"));
    }

    fn refresh_account_nav(&mut self, cx: &mut Cx) {
        use account_nav::AccountCategory as C;
        let desktop = self.last_size.x >= 820.0;
        let nav = self
            .account_nav
            .get_or_insert_with(account_nav::AccountNavModel::new);
        nav.set_layout(if desktop {
            account_nav::LayoutMode::Desktop
        } else {
            account_nav::LayoutMode::Mobile
        });
        let category = if nav.depth() > 1 {
            nav.selected()
        } else {
            None
        };
        let detail = category.is_some() || self.account_section.is_some() || self.overlay.is_some();
        let account_visible = self.tab == 4 && !self.auth_gate && self.intro.is_none();
        self.show(cx, ids!(me_list), account_visible && (desktop || !detail));
        self.show(
            cx,
            ids!(workspace_pages),
            !account_visible || desktop || detail,
        );
        self.show(cx, ids!(account_detail), desktop || detail);
        if let Some(mut v) = self.view.view(cx, ids!(me_list)).borrow_mut() {
            v.walk.width = if desktop {
                Size::Fixed(200.0)
            } else {
                Size::fill()
            };
        }
        self.show(
            cx,
            ids!(account_landing),
            category.is_none() && self.overlay.is_none(),
        );
        self.show(cx, ids!(account_about), category == Some(C::About));
        let drafts = self.account_section == Some(AccountSection::Drafts);
        for id in [
            live_id!(me_identity),
            live_id!(me_bal),
            live_id!(me_stats),
            live_id!(me_rows),
        ] {
            self.show(cx, &[id], !drafts);
        }
        self.show(
            cx,
            ids!(account_draft_empty),
            drafts && self.draft_pending.is_none(),
        );
        self.show(cx, ids!(acc_back), !desktop || self.account_edit.is_some());
        self.set_text(
            cx,
            ids!(acc_title),
            self.account_section
                .map_or_else(|| category.map_or("我", C::label), AccountSection::label),
        );
        for (i, cat) in C::ALL.iter().enumerate() {
            self.set_text(cx, &[account_ui::LIST_CATEGORY_IDS[i]], cat.label());
        }
        for (i, id) in account_ui::LIST_CATEGORY_IDS.iter().enumerate() {
            let selected = self.account_section.is_none() && category == Some(C::ALL[i]);
            let chip = self.view.check_box(cx, &[*id]);
            if chip.active(cx) != selected {
                chip.set_active(cx, selected, Animate::Yes);
            }
        }
        for section in AccountSection::ALL {
            let chip = self.view.check_box(cx, &[section.id()]);
            let selected = self.account_section == Some(section);
            if chip.active(cx) != selected {
                chip.set_active(cx, selected, Animate::Yes);
            }
        }
        let editing = self.account_edit;
        if let Some(field) = editing {
            self.set_text(
                cx,
                ids!(acc_title),
                match field {
                    0 => "修改显示名",
                    1 => "绑定手机号",
                    2 => "绑定邮箱",
                    3 => "编辑收货地址",
                    _ => "修改称呼",
                },
            );
        }
        self.show(cx, ids!(pf_status), editing.is_none());
        self.show(cx, ids!(pf_contact_note), editing.is_none());
        self.show(
            cx,
            ids!(pf_signout_row),
            category == Some(C::AccountContact) && editing.is_none(),
        );
        self.show(
            cx,
            ids!(ap_head),
            category == Some(C::General) && editing.is_none(),
        );
        self.show(
            cx,
            ids!(ap_card),
            category == Some(C::General) && editing.is_none(),
        );
        if let Some(mut v) = self.view.view(cx, ids!(account_detail)).borrow_mut() {
            v.walk.width = if desktop {
                Size::Fixed(self.content_w.clamp(280.0, 720.0))
            } else {
                Size::fill()
            };
        }
        let error = self.profile_err.or(self.nick_err);
        self.show(cx, ids!(account_error), error.is_some());
        if let Some(error) = error {
            self.set_text(cx, ids!(account_error), error);
        }
        self.show(cx, ids!(account_edit_bar), editing.is_some());
        for (id, cat) in [
            (live_id!(pf_name_head), C::Profile),
            (live_id!(pf_name_card), C::Profile),
            (live_id!(pf_contact_head), C::AccountContact),
            (live_id!(pf_contact_card), C::AccountContact),
            (live_id!(pf_signout_row), C::AccountContact),
            (live_id!(pf_address_head), C::ShippingAddress),
            (live_id!(pf_address_card), C::ShippingAddress),
            (live_id!(ap_head), C::General),
            (live_id!(ap_card), C::General),
            (live_id!(nk_head), C::General),
            (live_id!(nk_card), C::General),
            (live_id!(ntf_head), C::Notifications),
            (live_id!(ntf_card), C::Notifications),
            (live_id!(data_head), C::DataManagement),
            (live_id!(data_card), C::DataManagement),
        ] {
            self.show(cx, &[id], category == Some(cat));
        }
        self.show(
            cx,
            ids!(pf_signout_row),
            category == Some(C::AccountContact) && editing.is_none(),
        );
        self.show(
            cx,
            ids!(ap_head),
            category == Some(C::General) && editing.is_none(),
        );
        self.show(
            cx,
            ids!(ap_card),
            category == Some(C::General) && editing.is_none(),
        );
        self.show(cx, ids!(pf_address_list), editing != Some(3));
        for (i, id) in PROFILE_ADDRESS_ROWS.iter().enumerate() {
            self.show(
                cx,
                &[*id],
                editing != Some(3) && i < self.profile.addresses.len(),
            );
        }
        if editing == Some(0) {
            self.show(cx, ids!(pf_avatar_delete), false);
            self.show(cx, ids!(pf_avatar_edit), false);
        }
        self.show(cx, ids!(pf_name_summary), editing != Some(0));
        self.show(cx, ids!(pf_avatar_note), editing != Some(0));
        self.show(cx, ids!(pf_avatar_row), editing != Some(0));
        self.show(cx, ids!(nk_input_editor), editing == Some(4));
        self.show(cx, ids!(nk_value), editing != Some(4));
        self.show(cx, ids!(nk_edit), editing != Some(4));
        self.show(cx, ids!(nk_save), false);
        self.set_text(cx, ids!(nk_value), &self.state.settings.nickname.clone());
        self.show(cx, ids!(account_drafts), true);
        self.show(cx, ids!(pf_name_editor), editing == Some(0));
        for (id, field) in [
            (live_id!(pf_phone_editor), 1),
            (live_id!(pf_phone_code_editor), 1),
            (live_id!(pf_email_editor), 2),
            (live_id!(pf_email_code_editor), 2),
            (live_id!(pf_addr_name_editor), 3),
            (live_id!(pf_addr_phone_editor), 3),
            (live_id!(pf_addr_text_editor), 3),
        ] {
            self.show(cx, &[id], editing == Some(field));
        }
        self.show(cx, ids!(pf_phone_summary), editing.is_none());
        self.show(cx, ids!(pf_email_summary), editing.is_none());
        self.show(cx, ids!(pf_addr_new), editing != Some(3));
        // The fixed action bar is the only save/cancel area for text editors.
        for id in [
            live_id!(pf_save),
            live_id!(pf_phone_save),
            live_id!(pf_email_save),
            live_id!(pf_addr_add),
            live_id!(pf_addr_cancel),
        ] {
            self.show(cx, &[id], false);
        }
        self.set_text(
            cx,
            ids!(account_save),
            if matches!(editing, Some(1 | 2)) {
                "确认绑定"
            } else {
                "保存"
            },
        );
        self.set_text(cx, ids!(pf_name_value), &self.profile.display_name.clone());
        self.set_text(
            cx,
            ids!(pf_phone_value),
            &format!(
                "手机号：{}{}",
                if self.profile.phone.is_empty() {
                    "未绑定"
                } else {
                    &self.profile.phone
                },
                if !self.profile.phone.is_empty() && !self.profile.phone_verified { "（未验证）" } else { "" }
            ),
        );
        self.set_text(
            cx,
            ids!(pf_email_value),
            &format!(
                "邮箱：{}{}",
                if self.profile.email.is_empty() {
                    "未绑定"
                } else {
                    &self.profile.email
                },
                if !self.profile.email.is_empty() && !self.profile.email_verified { "（未验证）" } else { "" }
            ),
        );
        self.set_text(
            cx,
            ids!(me_identity_name),
            &self.profile.display_name.clone(),
        );
        self.set_text(cx, ids!(account_name), &self.profile.display_name.clone());
    }

    fn account_has_unsaved_changes(&mut self, cx: &mut Cx) -> bool {
        if self.avatar_session.is_some() {
            return true;
        }
        match self.account_edit {
            Some(0) => self.input_text(cx, ids!(pf_name)) != self.profile.display_name,
            Some(1) => {
                self.input_text(cx, ids!(pf_phone)) != self.profile.phone
                    || !self.input_text(cx, ids!(pf_phone_code)).is_empty()
            }
            Some(2) => {
                self.input_text(cx, ids!(pf_email)) != self.profile.email
                    || !self.input_text(cx, ids!(pf_email_code)).is_empty()
            }
            Some(3) => {
                let original = self
                    .profile
                    .addresses
                    .iter()
                    .find(|a| Some(a.id) == self.profile_edit_address)
                    .cloned();
                let name = original.as_ref().map_or("", |a| a.recipient_name.as_str());
                let phone = original.as_ref().map_or("", |a| a.phone.as_str());
                let address = original.as_ref().map_or("", |a| a.address.as_str());
                self.input_text(cx, ids!(pf_addr_name)) != name
                    || self.input_text(cx, ids!(pf_addr_phone)) != phone
                    || self.input_text(cx, ids!(pf_addr_text)) != address
            }
            Some(4) => self.input_text(cx, ids!(nk_input)) != self.state.settings.nickname,
            _ => false,
        }
    }

    fn account_shortcut_clicked(&mut self, cx: &mut Cx, id: LiveId, actions: &Actions) -> bool {
        self.view.check_box(cx, &[id]).changed(actions).is_some()
    }

    fn cancel_account_edit(&mut self, cx: &mut Cx) {
        self.account_edit = None;
        if let Some(nav) = self.account_nav.as_mut() {
            nav.cancel_edit();
        }
        self.set_text(cx, ids!(nk_input), &self.state.settings.nickname.clone());
        self.profile_edit_address = None;
        self.profile_err = None;
        self.nick_err = None;
        self.avatar_session = None;
        self.set_text(cx, ids!(pf_name), &self.profile.display_name.clone());
        self.set_text(cx, ids!(pf_phone), &self.profile.phone.clone());
        self.set_text(cx, ids!(pf_email), &self.profile.email.clone());
        self.set_text(cx, ids!(pf_phone_code), "");
        self.set_text(cx, ids!(pf_email_code), "");
        self.refresh_profile(cx);
        self.view.widget(cx, ids!(page_me)).redraw(cx);
    }

    fn refresh_wallet(&mut self, cx: &mut Cx) {
        self.set_text(cx, ids!(wl_v), &yuan(self.state.balance()));
        let rows: Vec<LedgerEntry> = self.state.ledger_desc().into_iter().cloned().collect();
        for (j, row) in LEDGER_ROWS.iter().enumerate() {
            let Some(e) = rows.get(j) else {
                self.show(cx, &[*row], false);
                continue;
            };
            self.show(cx, &[*row], true);
            self.set_text(cx, &[*row, live_id!(ld_text)], &e.title);
            let date = if e.note.is_empty() {
                fmt_md(e.day)
            } else {
                format!("{} · {}", fmt_md(e.day), e.note)
            };
            self.set_text(cx, &[*row, live_id!(ld_date)], &date);
            let (amt, c) = if e.amount > 0 {
                (format!("+{}", yuan(e.amount)), self.pal.good)
            } else if e.amount < 0 {
                (yuan(e.amount), self.pal.ink)
            } else {
                (format!("外付 {}", yuan(e.external)), self.pal.ink_3)
            };
            self.set_text(cx, &[*row, live_id!(ld_amt)], &amt);
            self.tint_text(cx, &[*row, live_id!(ld_amt)], c);
        }
        self.apply_list_state(cx, ids!(wl_empty), "还没有流水", "", rows.len());
    }

    fn open_settings(&mut self, cx: &mut Cx) {
        if self.account_nav.as_ref().and_then(|n| n.selected()).is_none() { self.account_nav.get_or_insert_with(account_nav::AccountNavModel::new).select(account_nav::AccountCategory::General); }
        self.reset_armed = false;
        self.export_note = None;
        self.nick_err = None;
        let nick = self.state.settings.nickname.clone();
        self.set_text(cx, ids!(nk_input), &nick);
        self.refresh_settings(cx);
        self.open_overlay(cx, Overlay::Settings);
    }

    fn refresh_settings(&mut self, cx: &mut Cx) {
        let ti = if theme::mode() == ThemeMode::Dark { 0 } else { 1 };
        self.set_chip_group(cx, &THEME_SEGS, ti);
        self.show(cx, ids!(nk_err), self.nick_err.is_some());
        if let Some(e) = self.nick_err {
            self.set_text(cx, ids!(nk_err), e);
        }
        let s = self.state.settings.clone();
        self.set_switch(
            cx,
            ids!(row_ntf_gift),
            "礼物快过期提醒",
            "收到的礼物还剩 2 天没拆时提醒一次",
            s.notify_gift,
        );
        self.set_switch(
            cx,
            ids!(row_ntf_pact),
            "契约到期提醒",
            "你答应的契约到期前一天提醒一次",
            s.notify_pact,
        );
        self.set_switch(
            cx,
            ids!(row_ntf_wish),
            "熟人心愿单提醒",
            "熟人的日子前 3 天、心愿单上还有没人送的，提醒一次",
            s.notify_wish,
        );
        let note = self.export_note.clone().unwrap_or_else(|| "把本机记录导出成一个 JSON 文件".into());
        self.set_row(cx, ids!(row_export), "导出数据", &note, "");
        self.set_row(cx, ids!(row_reset), "恢复演示数据", "清空本机记录，换回一套演示数据", "");
        self.show(cx, ids!(reset_confirm), self.reset_armed);
    }

    // ---- 认证闸 ----

    /// 进认证闸（首次启动、登出、401 之后）。清掉来路，只留这一屏。
    fn enter_auth_gate(&mut self, cx: &mut Cx, err: Option<&'static str>) {
        self.auth_gate = true;
        self.auth_register = false;
        self.overlay = None;
        self.back_stack.clear();
        self.intro = None;
        self.profile_err = err;
        self.update_page_visibility(cx);
        self.refresh_auth(cx);
    }

    /// 刷新认证界面：登录 / 注册互切的字段与文案、测试服务器提示。
    fn refresh_auth(&mut self, cx: &mut Cx) {
        let reg = self.auth_register;
        self.show(cx, ids!(au_server_err), self.server_error.is_some());
        self.show(cx, ids!(au_retry), self.server_error.is_some());
        if let Some(error) = self.server_error {
            self.set_text(cx, ids!(au_server_err), error);
        }
        self.show(cx, ids!(au_code_wrap), reg);
        self.set_text(cx, ids!(au_submit), if reg { "注册并登录" } else { "登录" });
        self.set_text(
            cx,
            ids!(au_switch),
            if reg { "已有账号？去登录" } else { "没有账号？去注册" },
        );
        // 固定密码 / 验证码说明只在测试服务器上出现。
        self.show(cx, ids!(au_test_note), profile_client::is_test_server());
        self.show(cx, ids!(au_err), self.profile_err.is_some());
        if let Some(e) = self.profile_err {
            self.set_text(cx, ids!(au_err), e);
        }
    }

    fn show_auth_error(&mut self, cx: &mut Cx, err: &'static str) {
        self.profile_err = Some(err);
        self.refresh_auth(cx);
    }

    /// 登录 / 注册之后：重置按账号隔离的客户端缓存，关闸进应用。
    fn after_account_switch(&mut self, cx: &mut Cx) {
        self.pending_contacts = None;
        self.editing_contact = None;
        self.contact_import_receiver = None;
        cx.stop_timer(self.contact_import_poll);
        self.contact_import_poll = Timer::empty();
        self.contact_page = 0;
        self.direct_contact_page = 0;
        for field in [
            live_id!(ca_input),
            live_id!(ct_phone),
            live_id!(ct_email),
            live_id!(ca_recipient_label),
            live_id!(ca_recipient_value),
        ] {
            self.set_text(cx, &[field], "");
        }
        self.show(cx, ids!(ct_preview), false);
        self.auth_gate = false;
        self.server_error = None;
        self.profile_err = None;
        self.commerce = Commerce::default();
        self.gift_client = GiftClient::default();
        // 通知按账号刷新:清掉上一账号已弹过的记录与残留通知条,关闸后按新账号重新查。
        self.notice_sent.clear();
        self.close_notice(cx);
        self.profile = profile_client::load(&self.state.settings.nickname);
        self.refresh_all(cx);
        self.set_tab(cx, 0);
        if !self.state.settings.onboarded {
            self.open_intro(cx, 0);
        }
        self.toast(
            cx,
            if self.auth_register {
                "注册并登录成功"
            } else {
                "已登录"
            },
        );
        self.update_page_visibility(cx);
    }

    /// 把头像字节解码成纹理。统一走 AvatarEditSession 归一化（EXIF 校正 /
    /// 尺寸上限在载入时已处理）再编码成 PNG，用仓内已验证的
    /// ImageBuffer::from_png + into_new_mip_texture 上纹理。
    fn avatar_texture(&mut self, cx: &mut Cx, bytes: &[u8]) -> Option<Texture> {
        let session = avatar::AvatarEditSession::load_bytes(bytes).ok()?;
        let enc = session
            .fit_within(128)
            .encode_final(avatar::AvatarFormat::Png)
            .ok()?;
        ImageBuffer::from_png(&enc.bytes)
            .ok()
            .map(|b| b.into_new_mip_texture(cx))
    }

    /// AvatarError → 界面用的静态文案（Msg = &'static str）。
    fn avatar_err(e: &avatar::AvatarError) -> Msg {
        match e {
            avatar::AvatarError::Io(_) => "读取图片文件失败",
            avatar::AvatarError::InputTooLarge => "图片文件过大（超过 32MB）",
            avatar::AvatarError::DimensionsTooLarge => "图片尺寸过大",
            avatar::AvatarError::UnsupportedFormat => "仅支持 JPEG / PNG / WebP",
            avatar::AvatarError::CorruptImage => "图片损坏或不是合法图像",
            avatar::AvatarError::InvalidCrop => "裁切区域无效",
            avatar::AvatarError::EncodeFailed => "头像编码失败，请换一张图",
            avatar::AvatarError::OutputTooLarge => "编码结果超过大小上限",
        }
    }

    /// 当前应显示的头像字节：编辑中 > 待上传本地字节 > 已上传（GET 服务端图像）。
    /// 返回值连同来源说明一起给状态行用。
    fn avatar_preview_bytes(&self) -> Option<(Vec<u8>, &'static str)> {
        if let Some(session) = &self.avatar_session {
            // 按当前锚点从原图即时裁切预览(原图保留,锚点切换不破坏),再缩到 512 上限。
            let cropped = session.crop_anchor(self.avatar_crop_anchor).ok();
            if let Some(s) = cropped {
                if let Ok(enc) = s.fit_within_max().encode_final(avatar::AvatarFormat::Png) {
                    return Some((enc.bytes, "预览中，未确认"));
                }
            }
        }
        if let Some(bytes) = &self.avatar_pending_bytes {
            return Some((bytes.clone(), "未同步"));
        }
        // 已上传头像:GET 服务端图像字节回显(avatar_url 是 /api/v1/media/avatars/<hex>)。
        // 走 URL 缓存(见 refresh_avatar),只读不重复请求。
        if !self.profile.avatar_url.is_empty() && !profile_client::avatar_pending(&self.profile) {
            if let Some((url, bytes)) = &self.avatar_server_cache {
                if *url == self.profile.avatar_url {
                    return Some((bytes.clone(), "已同步"));
                }
            }
        }
        None
    }

    /// 刷新资料页头像区：预览图、状态文案、按钮可用性。
    fn refresh_avatar(&mut self, cx: &mut Cx) {
        let has_server_avatar =
            !self.profile.avatar_url.is_empty() && !profile_client::avatar_pending(&self.profile);
        let default_avatar = self
            .profile
            .avatar_url
            .starts_with("/api/v1/media/default-avatars/");
        // 已上传头像按 URL 缓存:URL 变化才同步 GET(带 2MiB 上限),避免反复刷新卡界面/无界内存。
        if has_server_avatar {
            let stale = self
                .avatar_server_cache
                .as_ref()
                .map_or(true, |(u, _)| *u != self.profile.avatar_url);
            if stale {
                self.avatar_server_cache =
                    profile_client::fetch_avatar_bytes(&self.profile.avatar_url)
                        .map(|b| (self.profile.avatar_url.clone(), b));
            }
        } else {
            self.avatar_server_cache = None;
        }
        let pending = profile_client::avatar_pending(&self.profile);
        let editing = self.avatar_session.is_some();

        // 预览图：编辑/待上传用本地字节；已上传头像服务端是标识不是可解码字节，先显占位。
        let bytes = self.avatar_preview_bytes().map(|(bytes, _)| bytes);
        if self.avatar_texture_bytes != bytes {
            self.avatar_tex = bytes
                .as_ref()
                .and_then(|bytes| self.avatar_texture(cx, bytes));
            self.avatar_texture_bytes = bytes;
            // Both views share the same texture; ordinary navigation never re-decodes it.
            self.view
                .image(cx, ids!(pf_avatar_thumb))
                .set_texture(cx, self.avatar_tex.clone());
            self.view
                .image(cx, ids!(account_avatar))
                .set_texture(cx, self.avatar_tex.clone());
        }

        let status = if self.avatar_busy {
            "上传中…".to_string()
        } else if editing {
            let (w, h) = self
                .avatar_session
                .as_ref()
                .map(|s| s.dimensions())
                .unwrap_or((0, 0));
            format!("编辑中 · {w}×{h} · 确认后上传")
        } else if pending {
            "未同步 · 头像已保存在本机，联网后点「重试上传」".to_string()
        } else if default_avatar {
            "默认头像 · 根据账号固定生成".to_string()
        } else if has_server_avatar {
            "头像已同步到服务器".to_string()
        } else {
            "还没有头像，显示默认占位图".to_string()
        };
        self.set_text(cx, ids!(pf_avatar_status), &status);
        self.show(cx, ids!(pf_avatar_edit), editing);
        self.show(
            cx,
            ids!(pf_avatar_delete),
            (has_server_avatar && !default_avatar) || pending,
        );
        // 重试上传按钮文案：pending 时把「确认上传」变成重试入口（编辑会话为空也能点）。
        if pending && !editing {
            self.set_text(cx, ids!(pf_avatar_load), "重试上传");
        } else {
            self.set_text(cx, ids!(pf_avatar_load), "选择图片…");
        }
    }

    /// 确认上传：编码最终字节 → 调 profile_client::upload_avatar。
    /// 断网时字节由 profile_client 落盘，这里也留一份用于本机预览。
    fn avatar_upload(&mut self, cx: &mut Cx) {
        let Some(session) = self.avatar_session.take() else {
            // 没有编辑会话 = 重试待上传（读回本地字节）。
            if profile_client::avatar_pending(&self.profile) {
                let Some(path) = profile_client::avatar_pending_path() else { return };
                let Ok(bytes) = std::fs::read(&path) else {
                    self.profile_err = Some("待上传头像本机缓存已丢失，请重新选图");
                    self.refresh_avatar(cx);
                    return;
                };
                return self.avatar_upload_bytes(cx, bytes, "image/png");
            }
            return;
        };
        // 确认时按当前锚点从原图裁切编码(原图已按需旋转,此处取最终裁图),再缩到 512 上限。
        let cropped = match session.crop_anchor(self.avatar_crop_anchor) {
            Ok(s) => s.fit_within_max(),
            Err(e) => {
                // 裁切失败保留 session 可重试(恢复原图会话)。
                self.avatar_session = Some(session);
                self.profile_err = Some(Self::avatar_err(&e));
                self.refresh_avatar(cx);
                return;
            }
        };
        // 默认输出 JPEG（体积小、服务端与 UI 的 512×512 约束一致）；保留 alpha 的图用 PNG。
        let enc = match cropped.encode_final(avatar::AvatarFormat::Jpeg) {
            Ok(e) => e,
            Err(e) => {
                // 编码失败保留 session 可重试,不丢编辑成果。
                self.avatar_session = Some(session);
                self.profile_err = Some(match e {
                    avatar::AvatarError::OutputTooLarge => "编码结果超过大小上限",
                    _ => "头像编码失败，请换一张图",
                });
                self.refresh_avatar(cx);
                return;
            }
        };
        self.avatar_upload_bytes(cx, enc.bytes, enc.content_type);
    }

    fn avatar_upload_bytes(&mut self, cx: &mut Cx, bytes: Vec<u8>, content_type: &'static str) {
        self.avatar_busy = true;
        self.refresh_avatar(cx);
        match profile_client::upload_avatar(&mut self.profile, &bytes, content_type) {
            Ok(_) => {
                self.avatar_busy = false;
                self.avatar_pending_bytes = None;
                self.toast(cx, "头像已上传");
            }
            Err(e) => {
                self.avatar_busy = false;
                if profile_client::avatar_pending(&self.profile) {
                    // 断网待上传：留本地字节用于预览，状态行由 refresh_avatar 明示。
                    self.avatar_pending_bytes = Some(bytes);
                    self.toast(cx, "网络不可用，头像已保存在本机（未同步）");
                } else {
                    self.profile_err = Some(e);
                }
            }
        }
        self.refresh_avatar(cx);
        self.refresh_profile(cx);
    }

    /// 删除头像：回默认占位；清本地编辑与待上传状态。
    fn avatar_remove(&mut self, cx: &mut Cx) {
        self.avatar_session = None;
        self.avatar_pending_bytes = None;
        match profile_client::delete_avatar(&mut self.profile) {
            Ok(()) => self.toast(cx, "头像已删除，恢复默认"),
            Err(e) => {
                if !self.profile.online {
                    self.toast(cx, "头像已在本机删除，联网后同步");
                } else {
                    self.profile_err = Some(e);
                }
            }
        }
        self.refresh_avatar(cx);
        self.refresh_profile(cx);
    }

    fn open_profile(&mut self, cx: &mut Cx) {
        if self.account_nav.as_ref().and_then(|n| n.selected()).is_none() { self.account_nav.get_or_insert_with(account_nav::AccountNavModel::new).select(account_nav::AccountCategory::Profile); }
        self.profile = profile_client::load(&self.state.settings.nickname);
        self.profile_err = None;
        self.profile_edit_address = None;
        // 联网且有会话时,若上次断网删除留下了「待删除」意图,先重试服务端同步。
        // 重试失败(仍断网/服务端错)时不再用远端资料覆盖——保留待删除状态与提示,
        // 避免旧 avatar_url 重新显示却标「已同步」。
        if profile_client::avatar_delete_pending() {
            let synced = profile_client::retry_delete_avatar();
            if synced {
                self.profile = profile_client::load(&self.state.settings.nickname);
            } else {
                // 待删除未同步:本地视图为「已删除待同步」,不显示服务端旧头像。
                self.profile.avatar_url = String::new();
                self.profile_err = Some("头像删除待同步:联网后自动重试");
            }
        }
        // 重启后恢复待上传预览：本机还有未同步字节就显示出来并明示。
        self.avatar_session = None;
        self.avatar_busy = false;
        self.avatar_pending_bytes = if profile_client::avatar_pending(&self.profile) {
            profile_client::avatar_pending_path().and_then(|p| std::fs::read(p).ok())
        } else {
            None
        };
        let p = self.profile.clone();
        self.set_text(cx, ids!(pf_name), &p.display_name);
        self.set_text(cx, ids!(pf_phone), &p.phone);
        self.set_text(cx, ids!(pf_email), &p.email);
        self.set_text(cx, ids!(pf_phone_code), "");
        self.set_text(cx, ids!(pf_email_code), "");
        self.refresh_profile(cx);
        self.open_overlay(cx, Overlay::Profile);
    }

    fn refresh_profile(&mut self, cx: &mut Cx) {
        let status = if self.profile.online {
            format!("已连接礼遇服务器 · 当前账号 {}", profile_client::active_identifier())
        } else {
            format!("服务器连接失败 · 当前账号 {}", profile_client::active_identifier())
        };
        self.set_text(cx, ids!(pf_status), &status);
        let summary = if self.profile.addresses.is_empty() {
            "还没有保存地址".to_string()
        } else {
            format!("共 {} 条地址{}", self.profile.addresses.len(),
                if self.profile.addresses.len() > PROFILE_ADDRESS_ROWS.len() { " · 仅显示前 6 条" } else { "" })
        };
        self.set_text(cx, ids!(pf_address_list), &summary);
        let rows = self.profile.addresses.iter().take(PROFILE_ADDRESS_ROWS.len())
            .map(|a| format!("{}{} · {} · {}", if a.is_default { "默认 · " } else { "" },
                a.recipient_name, a.phone, a.address)).collect::<Vec<_>>();
        for (i, row_id) in PROFILE_ADDRESS_ROWS.iter().enumerate() {
            self.show(cx, &[*row_id], i < rows.len());
            if let Some(text) = rows.get(i) {
                self.set_text(cx, &[*row_id, PROFILE_ADDRESS_TEXT[i]], text);
            }
        }
        self.set_text(cx, ids!(pf_addr_add), if self.profile_edit_address.is_some() { "保存地址修改" } else { "添加地址" });
        self.show(cx, ids!(pf_addr_cancel), self.profile_edit_address.is_some());
        self.refresh_avatar(cx);
        self.refresh_account_nav(cx);
        self.show(cx, ids!(pf_err), false);
        if let Some(err) = self.profile_err { self.set_text(cx, ids!(pf_err), err); }
    }

    fn contact_delivery_choices(&self) -> Vec<(String, String, String)> {
        self.state
            .contacts
            .iter()
            .flat_map(|c| {
                contacts::choices(c)
                    .into_iter()
                    .map(move |(kind, value)| (c.label.clone(), kind, value))
            })
            .collect()
    }

    fn open_direct_order(&mut self, cx: &mut Cx, product: Option<u16>) {
        if self.commerce.order.as_ref().is_some_and(|o| o.status == "pending") {
            self.toast(cx, "请先完成当前待支付订单");
        } else {
            self.commerce.order = None;
            self.direct_product = product;
        }
        self.direct_error = None;
        self.refresh_direct_order(cx);
        self.nav_to(cx, Overlay::DirectOrder);
    }

    fn refresh_direct_order(&mut self, cx: &mut Cx) {
        self.set_text(cx, ids!(ca_status), "单件礼物 · 以服务器价格生成订单");
        self.show(cx, ids!(ca_pick), self.direct_product.is_some());
        if let Some(product) = self.direct_product {
            self.set_text(cx, ids!(ca_pick_title),
                &format!("{} · 按服务器报价结算", item(product).name));
        }
        let all_choices = self.contact_delivery_choices();
        self.direct_contact_page = self.direct_contact_page
            .min(all_choices.len().saturating_sub(1) / CART_FRIEND_CHIPS.len());
        self.show(cx, ids!(ca_choice_prev), self.direct_contact_page > 0);
        self.show(cx, ids!(ca_choice_next),
            (self.direct_contact_page + 1) * CART_FRIEND_CHIPS.len() < all_choices.len());
        for (i, chip) in CART_FRIEND_CHIPS.iter().enumerate() {
            let choice = all_choices.get(self.direct_contact_page * CART_FRIEND_CHIPS.len() + i);
            self.show(cx, &[*chip], choice.is_some());
            if let Some((label, _, value)) = choice {
                self.set_text(cx, &[*chip], &format!("{label} · {value}"));
            }
        }
        self.set_text(cx, ids!(ca_friends_note),
            "选联系人或手填手机号/邮箱；无需先注册或加好友。单次只送一件礼物。");
        self.show(cx, ids!(ca_checkout), self.direct_product.is_some()
            && !self.commerce.order.as_ref().is_some_and(|o| o.status == "pending"));
        self.show(cx, ids!(ca_order), self.commerce.order.is_some());
        if let Some(order) = self.commerce.order.clone() {
            let state = match order.status.as_str() {
                "pending" => "待测试支付",
                "paid_test" => "已完成测试支付",
                _ => "状态待刷新",
            };
            self.set_text(cx, ids!(ca_order_text),
                &format!("服务器订单 #{} · {} · {}", order.id.abs(), yuan(order.total_cents), state));
            self.show(cx, ids!(ca_pay), order.status == "pending");
        }
        self.show(cx, ids!(ca_error), self.direct_error.is_some());
        if let Some(e) = self.direct_error { self.set_text(cx, ids!(ca_error), e); }
    }

    // ---- 事件 ----

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        let today = today_days();
        if self.clicked(cx,ids!(ct_prev),actions){self.contact_page=self.contact_page.saturating_sub(1);self.refresh_contacts(cx);}
        if self.clicked(cx,ids!(ct_next),actions){self.contact_page+=1;self.refresh_contacts(cx);}
        if self.clicked(cx,ids!(ca_choice_prev),actions){self.direct_contact_page=self.direct_contact_page.saturating_sub(1);self.refresh_direct_order(cx);}
        if self.clicked(cx,ids!(ca_choice_next),actions){self.direct_contact_page+=1;self.refresh_direct_order(cx);}

        for action in actions {if let Some(fda)=action.downcast_ref::<FileDialogAction>(){if fda.id()==live_id!(contact_import){if let Some(path)=fda.path(){match contacts::load_file(path){Ok(import)=>self.stage_contacts(cx,import),Err(e)=>self.toast(cx,e)}}}}}
        if self.clicked(cx,ids!(ct_import_cancel),actions){self.pending_contacts=None;self.show(cx,ids!(ct_preview),false);}
        if self.clicked(cx,ids!(ct_import_confirm),actions){if let Some(import)=self.pending_contacts.take(){let added=contacts::merge(&mut self.state.contacts,import.contacts);self.state.save();self.show(cx,ids!(ct_preview),false);self.refresh_contacts(cx);self.toast(cx,&format!("已导入：新增 {added} 位，其余相同联系方式已合并"));}}
        if self.clicked(cx,ids!(ct_cancel_edit),actions){self.editing_contact=None;for id in [live_id!(ca_input),live_id!(ct_phone),live_id!(ct_email)]{self.set_text(cx,&[id],"");}self.refresh_contacts(cx);}
        for (button,field,kind) in [(live_id!(pf_phone_request),live_id!(pf_phone),"phone"),(live_id!(pf_email_request),live_id!(pf_email),"email")] {
            if self.clicked(cx,&[button],actions){let value=self.input_text(cx,&[field]);match profile_client::request_contact_code("bind",kind,&value){Ok(Some(code))=>self.toast(cx,&format!("测试通道验证码：{code}")),Ok(None)=>self.toast(cx,"验证码已加入投递队列，请查收"),Err(e)=>self.toast(cx,e)}}
        }
        // ---- 头像文件选择对话框结果：选中 → 载入编辑会话；取消 → 不动状态、不上传 ----
        for action in actions {
            if let Some(fda) = action.downcast_ref::<FileDialogAction>() {
                if fda.id() == live_id!(avatar_pick) {
                    match fda {
                        FileDialogAction::FileSelected { .. } => {
                            if let Some(path) = fda.path() {
                                match avatar::AvatarEditSession::load_path(path) {
                                    Ok(session) => {
                                        self.avatar_crop_anchor = 4; // 重置为居中
                                        self.set_text(cx, ids!(pf_avatar_anchor), "裁切位置：居中");
                                        self.avatar_session = Some(session);
                                        self.profile_err = None;
                                    }
                                    Err(e) => {
                                        self.profile_err = Some(Self::avatar_err(&e));
                                        self.avatar_session = None;
                                    }
                                }
                            }
                        }
                        FileDialogAction::FileCancelled { .. } => {}
                        _ => {}
                    }
                    self.refresh_avatar(cx);
                }
            }
        }

        // ---- 认证闸：全屏时只处理自己的几个按钮，别的一律不响应 ----
        if self.auth_gate {
            if self.clicked(cx, ids!(au_retry), actions) {
                self.server_error = profile_client::check_server().err();
                if self.server_error.is_none() {
                    self.profile_err = None;
                    if profile_client::has_choice() {
                        self.after_account_switch(cx);
                    } else {
                        self.refresh_auth(cx);
                    }
                } else {
                    self.refresh_auth(cx);
                }
                return;
            }
            if self.clicked(cx, ids!(au_switch), actions) {
                self.auth_register = !self.auth_register;
                self.refresh_auth(cx);
            }
            if self.clicked(cx,ids!(au_request_code),actions){let value=self.input_text(cx,ids!(au_identifier));let kind=if value.contains('@'){"email"}else{"phone"};match profile_client::request_contact_code("register",kind,&value){Ok(Some(code))=>{self.set_text(cx,ids!(au_code),&code);self.set_text(cx,ids!(au_err),&format!("测试通道验证码：{code}"));self.show(cx,ids!(au_err),true);},Ok(None)=>{self.set_text(cx,ids!(au_err),"验证码已加入投递队列，请查收");self.show(cx,ids!(au_err),true);},Err(e)=>self.show_auth_error(cx,e)}}
            let submit = self.clicked(cx, ids!(au_submit), actions)
                || self.view.text_input(cx, ids!(au_identifier)).returned(actions).is_some()
                || self.view.text_input(cx, ids!(au_password)).returned(actions).is_some()
                || (self.auth_register && self.view.text_input(cx, ids!(au_code)).returned(actions).is_some());
            if submit {
                let identifier = self.input_text(cx, ids!(au_identifier));
                let password = self.input_text(cx, ids!(au_password));
                let code = self.input_text(cx, ids!(au_code_wrap.au_code));
                let register = self.auth_register;
                match profile_client::sign_in(&identifier, &password, &code, register) {
                    Ok(()) => self.after_account_switch(cx),
                    Err(e) => self.show_auth_error(cx, e),
                }
            }
            return;
        }

        // 401 / token 失效：不装在线、不静默重登，清会话回认证闸。
        if profile_client::take_expired() {
            self.profile.online = false;
            self.enter_auth_gate(cx, Some("登录已过期，请重新登录"));
            return;
        }

        // 导航（侧栏与底部导航是同一组 Tab 的两份控件）
        for i in 0..TABS.len() {
            let a = self.toggled(cx, &[live_id!(sidebar), TABS[i]], actions);
            let b = self.toggled(cx, &[live_id!(tabbar), TABS[i]], actions);
            if a || b {
                self.set_tab(cx, i);
            }
        }
        if self.clicked(cx, ids!(tb_back), actions) {
            self.go_back(cx);
        }
        if self.clicked(cx, ids!(tb_action), actions) {
            if self.tab == 0 && self.gift_wish_seg {
                self.open_wish_edit(cx, None, None);
            } else {
                let first = self.gift_rows.first().copied().unwrap_or(0);
                self.open_send(cx, first, String::new(), None);
            }
        }
        if self.clicked(cx, ids!(tb_add), actions) {
            self.import_menu = !self.import_menu;
            self.update_page_visibility(cx);
        }
        if self.clicked(cx, ids!(im_local), actions) {
            self.import_menu = false;
            self.update_page_visibility(cx);
            self.import_local(cx);
        }
        if self.clicked(cx, ids!(im_file), actions) {
            self.import_menu = false;
            self.update_page_visibility(cx);
            self.import_vcard(cx);
        }
        if self.clicked(cx, ids!(ct_hit), actions) {
            self.import_menu = false;
            self.update_page_visibility(cx);
        }

        // 开场三屏
        if let Some(step) = self.intro {
            if self.clicked(cx, ids!(in_next), actions) {
                if step + 1 >= INTRO.len() {
                    self.close_intro(cx);
                } else {
                    self.intro = Some(step + 1);
                    self.refresh_intro(cx);
                }
            }
            if self.clicked(cx, ids!(in_back), actions) && step > 0 {
                self.intro = Some(step - 1);
                self.refresh_intro(cx);
            }
            if self.clicked(cx, ids!(in_skip), actions) {
                self.close_intro(cx);
            }
        }

        // 通知 / toast
        if self.clicked(cx, ids!(nt_close), actions) {
            if let Some(n)=&self.online_notice {let _=commerce_client::read_notification(n.id);}
            self.close_notice(cx);
        }
        if self.clicked(cx, ids!(nt_go), actions) {
            if let Some(n)=self.online_notice.clone(){let _=commerce_client::read_notification(n.id);self.close_notice(cx);if let Some(id)=n.gift_id{self.open_received(cx,id);}else{self.toast(cx,&n.body);}}
            if let Some(n) = self.notice.clone() {
                self.close_notice(cx);
                match n.kind {
                    NoticeKind::GiftExpiring => self.open_received(cx, n.target),
                    NoticeKind::PactDue => {
                        self.pact_theirs = false;
                        self.set_tab(cx, 2);
                    }
                    NoticeKind::WishSoon => self.open_wish(cx, n.target),
                }
            }
        }
        if self.clicked(cx, ids!(to_undo), actions) {
            self.close_toast(cx, true);
        }

        // ---- 挑礼 ----
        for (i, id) in CAT_CHIPS.iter().enumerate() {
            if self.toggled(cx, &[*id], actions) {
                self.cat_idx = i;
                self.refresh_gift(cx);
            }
        }
        for (j, id) in GIFT_SEGS.iter().enumerate() {
            if self.toggled(cx, &[*id], actions) {
                self.gift_wish_seg = j == 1;
                self.refresh_gift(cx);
                self.refresh_topbar(cx);
                self.view.view(cx, ids!(page_gift)).set_scroll_pos(cx, Vec2d::default());
            }
        }
        for (j, card) in GRID_CARDS.iter().enumerate() {
            if self.clicked(cx, &[*card, live_id!(pk_hit)], actions) {
                if let Some(i) = self.gift_rows.get(j).copied() {
                    self.open_product(cx, i, None);
                }
            }
        }

        // ---- 送礼页 ----
        if self.overlay == Some(Overlay::Send) {
            if self.clicked(cx, ids!(sd_change), actions) {
                self.pick_open = !self.pick_open;
                self.refresh_send(cx);
            }
            for (j, row) in PICK_ROWS.iter().enumerate() {
                if self.clicked(cx, &[*row, live_id!(gr_hit)], actions) {
                    let Some(i) = self.pick_rows.get(j).copied() else { continue };
                    self.draft.item = i;
                    self.pick_open = false;
                    self.send_error = None;
                    self.refresh_send(cx);
                }
            }
            for (j, id) in PEER_CHIPS.iter().enumerate() {
                if self.toggled(cx, &[*id], actions) {
                    self.draft.peer = self.send_peers.get(j).cloned().filter(|_| j < PEER_SLOTS).unwrap_or_default();
                    self.refresh_send(cx);
                }
            }
            for (j, id) in UNLOCK_SEGS.iter().enumerate() {
                if self.toggled(cx, &[*id], actions) {
                    self.draft.unlock = Unlock::ALL[j];
                    self.send_error = None;
                    self.refresh_send(cx);
                }
            }
            if self.clicked(cx, &[live_id!(sd_pact_row), live_id!(sw_hit)], actions) {
                self.contract_on = !self.contract_on;
                self.refresh_send(cx);
            }
            for (j, id) in PRESET_CHIPS.iter().enumerate() {
                if self.toggled(cx, &[*id], actions) {
                    self.set_text(cx, ids!(sd_pact_in), PACT_PRESETS[j].1);
                    self.refresh_presets(cx);
                }
            }
            if self.view.text_input(cx, ids!(sd_pact_in)).changed(actions).is_some() {
                self.refresh_presets(cx);
            }
            for (j, id) in DELIVER_SEGS.iter().enumerate() {
                if self.toggled(cx, &[*id], actions) {
                    let day = self.draft.wish.and_then(|(w, _)| self.state.wishlist(w)).map(|l| l.event_on);
                    self.draft.deliver_on = if j == 0 { day.filter(|&e| e > today) } else { None };
                    self.send_error = None;
                    self.refresh_send(cx);
                }
            }
            if self.clicked(cx, ids!(sd_go), actions) {
                self.submit_send(cx);
            }
        }

        // ---- 礼卡页 ----
        if self.overlay == Some(Overlay::Card) {
            for (j, id) in STYLE_SEGS.iter().enumerate() {
                if self.toggled(cx, &[*id], actions) {
                    self.card_style = if j == 0 { ShareStyle::Warm } else { ShareStyle::Night };
                    self.refresh_card(cx);
                }
            }
            if self.clicked(cx, ids!(cd_copy), actions) {
                if let Some(id) = self.card_gift {
                    self.copy_link(cx, id);
                }
            }
            if self.clicked(cx, ids!(cd_save), actions) {
                if let Some(scene) = self.card_scene() {
                    let msg = match share::save_card(&scene) {
                        Ok(Some(path)) => format!("已保存到 {}", path.display()),
                        Ok(None) => "未设置 MAKEPAD_HOME，无法保存".to_string(),
                        Err(e) => format!("保存失败：{e}"),
                    };
                    self.show(cx, ids!(cd_saved), true);
                    self.set_text(cx, ids!(cd_saved), &msg);
                }
            }
            if self.clicked(cx, ids!(cd_peek), actions) {
                self.open_preview(cx);
            }
            if self.clicked(cx, ids!(cd_done), actions) {
                if self.tab == 4 && self.account_section.is_some() { self.go_back(cx); }
                else { self.box_sent = true; self.set_tab(cx, 1); }
            }
        }

        for (sent, retry, empty, row_ids) in [
            (false, live_id!(received_gift_retry), live_id!(received_gift_empty), &RECEIVED_GIFT_ROWS),
            (true, live_id!(sent_gift_retry), live_id!(sent_gift_empty), &SENT_GIFT_ROWS),
        ] {
            if self.clicked(cx, &[retry], actions) || self.clicked(cx, &[empty, live_id!(em_action)], actions) {
                let _ = self.gift_client.refresh();
                self.refresh_account_gifts(cx, sent);
            }
            for (j, row) in row_ids.iter().enumerate() {
                if self.clicked(cx, &[*row, live_id!(bx_hit)], actions) {
                    let id = if sent { self.sent_gift_rows.get(j) } else { self.received_gift_rows.get(j) }.copied();
                    if let Some(id) = id { self.open_online_gift(cx, id, sent); }
                }
            }
        }

        // ---- 礼盒 ----
        for (j, id) in BOX_SEGS.iter().enumerate() {
            if self.toggled(cx, &[*id], actions) {
                self.box_sent = j == 1;
                self.refresh_box(cx);
            }
        }
        for (j, row) in BOX_ROWS.iter().enumerate() {
            if self.clicked(cx, &[*row, live_id!(bx_hit)], actions) {
                if let Some(id) = self.box_rows.get(j).copied() {
                    if self.gift_client.online {
                        self.open_online_gift(cx, id as i64, self.box_sent);
                    } else if self.box_sent {
                        self.open_sent(cx, id);
                    } else {
                        self.open_received(cx, id);
                    }
                }
            }
        }
        if self.clicked(cx, ids!(bx_empty.em_action), actions) {
            self.set_tab(cx, 0);
        }

        if self.overlay == Some(Overlay::OnlineGift) {
            if let Some(id) = self.online_gift_id {
                if self.clicked(cx, ids!(og_open), actions) {
                    self.online_gift_err = self.gift_client.open(id).err();
                    self.refresh_online_gift(cx);
                }
                let answer_enter = self.view.text_input(cx, ids!(og_answer)).returned(actions).is_some();
                if self.clicked(cx, ids!(og_answer_btn), actions) || answer_enter {
                    let answer = self.input_text(cx, ids!(og_answer));
                    self.online_gift_err = self.gift_client.answer(id, &answer).err();
                    if self.online_gift_err.is_none() { self.set_text(cx, ids!(og_answer), ""); }
                    self.refresh_online_gift(cx);
                }
                if self.clicked(cx, ids!(og_accept), actions) {
                    let agree = self.view.check_box(cx, ids!(og_agree)).active(cx);
                    let name = self.input_text(cx, ids!(og_ship_name));
                    let phone = self.input_text(cx, ids!(og_ship_phone));
                    let address = self.input_text(cx, ids!(og_ship_address));
                    self.online_gift_err = self.gift_client.accept(id, agree, &name, &phone, &address).err();
                    self.refresh_online_gift(cx);
                }
                if self.clicked(cx, ids!(og_confirm), actions) {
                    self.online_gift_err = self.gift_client.confirm(id).err();
                    self.refresh_online_gift(cx);
                }
            }
        }

        // ---- 拆礼页 ----
        if self.overlay == Some(Overlay::Open) && !self.open_preview {
            for (j, id) in CAND_BTNS.iter().enumerate() {
                if self.clicked(cx, &[*id], actions) {
                    if let Some(n) = self.cand_names.get(j).cloned() {
                        self.set_text(cx, ids!(od_input), &n);
                    }
                }
            }
            let enter = self.view.text_input(cx, ids!(od_input)).returned(actions).is_some();
            if self.clicked(cx, ids!(od_submit), actions) || enter {
                self.submit_answer(cx);
            }
            if self.clicked(cx, ids!(or_accept), actions) {
                self.enter_stage(cx, Stage::Accept);
            }
            if self.clicked(cx, ids!(or_swap), actions) {
                self.enter_stage(cx, Stage::Swap);
            }
            if self.clicked(cx, &[live_id!(oa_agree), live_id!(sw_hit)], actions) {
                self.accept_agree = !self.accept_agree;
                // 报的错多半就是「先打开我同意」，一动开关就收起来。
                self.open_err = None;
                self.refresh_open(cx);
            }
            if self.clicked(cx, ids!(oa_back), actions) || self.clicked(cx, ids!(ow_back), actions) {
                self.enter_stage(cx, Stage::Reveal);
            }
            if self.clicked(cx, ids!(oa_ok), actions) {
                self.submit_accept(cx);
            }
            for (j, id) in SWAP_SEGS.iter().enumerate() {
                if self.toggled(cx, &[*id], actions) {
                    self.swap_exchange = j == 1;
                    self.open_err = None;
                    self.refresh_open(cx);
                    self.refresh_topbar(cx);
                }
            }
            for (j, row) in SWAP_ROWS.iter().enumerate() {
                if self.clicked(cx, &[*row, live_id!(ch_hit)], actions) {
                    self.swap_pick = self.swap_rows.get(j).copied();
                    self.open_err = None;
                    self.refresh_open(cx);
                }
            }
            if self.clicked(cx, ids!(ow_ok), actions) {
                self.submit_swap(cx);
            }
            if self.clicked(cx, ids!(odn_return), actions) {
                self.start_return(cx);
            }
            if self.clicked(cx, ids!(odn_home), actions) {
                if self.tab == 4 && self.account_section.is_some() { self.go_back(cx); }
                else { self.box_sent = false; self.set_tab(cx, 1); }
            }
        }

        // ---- 送出详情 ----
        if self.overlay == Some(Overlay::Sent) {
            if let Some(id) = self.sent_gift {
                if self.clicked(cx, ids!(ss_copy), actions) {
                    self.copy_link(cx, id);
                }
                if self.clicked(cx, ids!(ss_view), actions) {
                    self.card_gift = Some(id);
                    self.card_style = ShareStyle::Warm;
                    self.refresh_card(cx);
                    self.open_overlay(cx, Overlay::Card);
                }
                if self.clicked(cx, ids!(ss_withdraw), actions) {
                    self.withdraw_armed = true;
                    self.refresh_sent(cx);
                }
                if self.clicked(cx, ids!(ss_cno), actions) {
                    self.withdraw_armed = false;
                    self.refresh_sent(cx);
                }
                if self.clicked(cx, ids!(ss_cyes), actions) {
                    self.withdraw_armed = false;
                    match self.state.withdraw(id, today) {
                        Ok(()) => self.toast(cx, "已撤回，钱退回了余额"),
                        Err(e) => self.toast(cx, e),
                    }
                    self.after_data_change(cx);
                    self.refresh_sent(cx);
                }
                if self.clicked(cx, ids!(ss_sim_go), actions) {
                    match self.state.simulate_step(id, today) {
                        Some(step) => self.toast(cx, step.text()),
                        None => self.toast(cx, "这份礼物已经走完了"),
                    }
                    self.after_data_change(cx);
                    self.refresh_sent(cx);
                }
            }
        }

        // ---- 契约 ----
        for (j, id) in PACT_SEGS.iter().enumerate() {
            if self.toggled(cx, &[*id], actions) {
                self.pact_theirs = j == 1;
                self.refresh_pacts(cx);
            }
        }
        for (j, row) in PACT_ROWS.iter().enumerate() {
            let Some(pid) = self.pact_rows.get(j).copied() else { continue };
            if self.clicked(cx, &[*row, live_id!(pc_done)], actions) {
                match self.state.fulfil_pact(pid) {
                    Ok(()) => self.toast(cx, "契约已兑现"),
                    Err(e) => self.toast(cx, e),
                }
                self.after_data_change(cx);
            }
            if self.clicked(cx, &[*row, live_id!(pc_nudge)], actions) {
                match self.state.nudge_pact(pid, today) {
                    Ok(()) => self.toast(cx, "已经提醒 TA 啦"),
                    Err(e) => self.toast(cx, e),
                }
                self.refresh_pacts(cx);
            }
            if self.clicked(cx, &[*row, live_id!(pc_waive)], actions) {
                match self.state.waive_pact(pid) {
                    Ok(()) => self.toast(cx, "免了，这条契约不用兑现了"),
                    Err(e) => self.toast(cx, e),
                }
                self.refresh_pacts(cx);
            }
        }
        if self.clicked(cx, ids!(pt_empty.em_action), actions) {
            self.set_tab(cx, 0);
        }

        // ---- 熟人 ----
        let add_enter = self.view.text_input(cx, ids!(ca_input)).returned(actions).is_some();
        if self.clicked(cx, ids!(ca_btn), actions) || add_enter {
            self.add_contact(cx);
        }
        if self.clicked(cx, ids!(ct_empty.em_action), actions) {
            self.import_local(cx);
        }
        for (j, row) in CONTACT_ROWS.iter().enumerate() {
            let Some(cid) = self.contact_rows.get(j).copied() else { continue };
            if self.clicked(cx,&[*row,live_id!(cr_edit)],actions){if let Some(c)=self.state.contact(cid).cloned(){self.editing_contact=Some(cid);self.set_text(cx,ids!(ca_input),&c.label);self.set_text(cx,ids!(ct_phone),&c.phones.unwrap_or_default().join(";"));self.set_text(cx,ids!(ct_email),&c.emails.unwrap_or_default().join(";"));self.refresh_contacts(cx);}}
            if self.clicked(cx,&[*row,live_id!(cr_send)],actions){if let Some(c)=self.state.contact(cid).cloned(){
                self.set_text(cx,ids!(ca_recipient_label),&c.label);self.set_text(cx,ids!(ca_recipient_value),contacts::choices(&c).first().map(|(_,v)|v.as_str()).unwrap_or(""));
                self.direct_product=None;self.set_tab(cx,0);self.toast(cx,"已选择收礼人，请选商品并直接送礼；可更换投递联系方式");
            }}
            if self.clicked(cx, &[*row, live_id!(cr_del)], actions) {
                if let Some(snap) = self.state.remove_contact(cid) {
                    self.refresh_contacts(cx);
                    self.offer_undo(cx, snap);
                }
            }
        }

        for section in AccountSection::ALL {
            if self.account_shortcut_clicked(cx, section.id(), actions) {
                self.open_account_section(cx, section);
            }
        }
        if self.clicked(cx, ids!(account_intro), actions) { self.open_intro(cx,0); }
        // ---- 我 ----
        if self.clicked(cx, &[live_id!(row_wallet), live_id!(st_hit)], actions)
            || self.clicked(cx, ids!(me_wallet), actions)
        {
            if self.tab == 4 { self.open_account_section(cx, AccountSection::Wallet); }
            else { self.refresh_wallet(cx); self.open_overlay(cx, Overlay::Wallet); }
        }
        if self.clicked(cx, &[live_id!(row_settings), live_id!(st_hit)], actions) {
            self.open_settings(cx);
        }
        if self.clicked(cx, &[live_id!(row_profile), live_id!(st_hit)], actions) {
            self.open_profile(cx);
        }
        if self.clicked(cx, ids!(pd_direct), actions) && self.overlay == Some(Overlay::Product) {
            self.open_direct_order(cx, Some(self.pd_item));
        }
        if self.clicked(cx, &[live_id!(row_wish), live_id!(st_hit)], actions)
            || self.clicked(cx, &[live_id!(row_mywish), live_id!(st_hit)], actions)
            || self.clicked(cx, ids!(ag3_go), actions)
        {
            if self.tab == 4 { self.open_account_section(cx, AccountSection::Wishes); }
            else { self.open_wishes(cx); }
        }
        if self.clicked(cx, &[live_id!(row_about), live_id!(st_hit)], actions) {
            self.open_intro(cx, 0);
        }
        if self.clicked(cx, ids!(wl_top), actions) {
            self.state.top_up(today);
            self.toast(cx, &format!("演示充值 {} 已到账", yuan(TOP_UP_AMOUNT)));
            self.after_data_change(cx);
            self.refresh_gift(cx);
        }

        // ---- 设置 ----
        if self.overlay == Some(Overlay::Settings) {
            for (j, id) in THEME_SEGS.iter().enumerate() {
                if self.toggled(cx, &[*id], actions) {
                    let m = if j == 0 { ThemeMode::Dark } else { ThemeMode::Light };
                    self.state.settings.set_theme_mode(m);
                    self.state.save();
                    self.pending_theme = Some(m);
                    self.set_chip_group(cx, &THEME_SEGS, j);
                }
            }
            let nick_enter = self.view.text_input(cx, ids!(nk_input)).returned(actions).is_some();
            if (self.clicked(cx, ids!(nk_save), actions) || nick_enter) && self.account_edit != Some(4) {
                let name = self.input_text(cx, ids!(nk_input));
                match self.state.set_nickname(&name) {
                    Ok(()) => {
                        self.nick_err = None;
                        self.toast(cx, "称呼已保存");
                        self.refresh_me(cx);
                    }
                    Err(e) => self.nick_err = Some(e),
                }
                self.refresh_settings(cx);
            }
            if self.clicked(cx, &[live_id!(row_ntf_gift), live_id!(sw_hit)], actions) {
                self.state.settings.notify_gift = !self.state.settings.notify_gift;
                self.state.save();
                self.refresh_settings(cx);
            }
            if self.clicked(cx, &[live_id!(row_ntf_pact), live_id!(sw_hit)], actions) {
                self.state.settings.notify_pact = !self.state.settings.notify_pact;
                self.state.save();
                self.refresh_settings(cx);
            }
            if self.clicked(cx, &[live_id!(row_ntf_wish), live_id!(sw_hit)], actions) {
                self.state.settings.notify_wish = !self.state.settings.notify_wish;
                self.state.save();
                self.refresh_settings(cx);
            }
            if self.clicked(cx, &[live_id!(row_export), live_id!(st_hit)], actions) {
                self.export_note = Some(match self.state.export_data() {
                    Some(p) => format!("已导出到 {}", p.display()),
                    None => "未设置 MAKEPAD_HOME，无法导出".to_string(),
                });
                self.refresh_settings(cx);
            }
            if self.clicked(cx, &[live_id!(row_reset), live_id!(st_hit)], actions) {
                self.reset_armed = !self.reset_armed;
                self.refresh_settings(cx);
            }
            if self.clicked(cx, ids!(rc_cancel), actions) {
                self.reset_armed = false;
                self.refresh_settings(cx);
            }
            if self.clicked(cx, ids!(rc_ok), actions) {
                self.reset_armed = false;
                self.state.reset_demo(today);
                let nick = self.state.settings.nickname.clone();
                self.set_text(cx, ids!(nk_input), &nick);
                self.notice_sent.clear();
                self.refresh_all(cx);
                self.toast(cx, "已换回演示数据");
            }
        }

        if self.overlay == Some(Overlay::Profile) {
            // 登出：清凭据 → 清本机账号缓存 → 回认证闸。登录 / 注册已挪到认证闸。
            if self.clicked(cx, ids!(pf_signout), actions) {
                // 账号切换:先让草稿闸把旧账号未终态草稿作废,再清凭据。
                let old_account = profile_client::active_identifier();
                if let Some(g) = self.draft_gate.as_ref() {
                    g.switch_account(&old_account, "", 0);
                }
                self.draft_pending = None;
                profile_client::logout();
                profile_client::clear_local_cache();
                self.commerce = Commerce::default();
                self.gift_client = GiftClient::default();
                // 账号隔离：礼物 / 契约 / 流水 / 心愿单等本地缓存不留给下一个账号。
                self.state = LiyuState::demo(today);
                self.state.save();
                self.enter_auth_gate(cx, None);
                self.toast(cx, "已登出");
            }
            for (i, row_id) in PROFILE_ADDRESS_ROWS.iter().enumerate() {
                let Some(address) = self.profile.addresses.get(i).cloned() else { continue };
                if self.clicked(cx, &[*row_id, PROFILE_ADDRESS_ACTIONS[i], PROFILE_ADDRESS_EDIT[i]], actions) {
                    self.profile_edit_address = Some(address.id);
                    self.account_edit = Some(3);
                    self.set_text(cx, ids!(pf_addr_name), &address.recipient_name);
                    self.set_text(cx, ids!(pf_addr_phone), &address.phone);
                    self.set_text(cx, ids!(pf_addr_text), &address.address);
                    self.profile_err = None;
                    self.refresh_profile(cx);
                }
                if self.clicked(cx, &[*row_id, PROFILE_ADDRESS_ACTIONS[i], PROFILE_ADDRESS_DELETE[i]], actions) {
                    self.profile_err = profile_client::delete_address(&mut self.profile, address.id).err();
                    if self.profile_edit_address == Some(address.id) { self.profile_edit_address = None; }
                    if self.profile_err.is_none() { self.toast(cx, "地址已删除"); }
                    self.refresh_profile(cx);
                }
            }
            // ---- 头像工坊：选图 → 预览 → 裁切/旋转 → 确认/取消 ----
            if self.clicked(cx, ids!(pf_avatar_load), actions) {
                if profile_client::avatar_pending(&self.profile) && self.avatar_session.is_none() {
                    // 待上传状态下「重试上传」：直接读本地字节重新上传，不需要再选图。
                    self.avatar_upload(cx);
                } else {
                    // 原生 OS 文件选择对话框（取消 → FileCancelled，不动状态、不上传）。
                    let dialog = FileDialog::new()
                        .set_id(live_id!(avatar_pick))
                        .set_title("选择头像".into())
                        .add_filter(
                            "图片".into(),
                            vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into()],
                        );
                    cx.open_select_file_dialog(dialog);
                }
            }
            if self.clicked(cx, ids!(pf_avatar_anchor), actions) {
                // 九宫格位置循环:0 左上 → 4 居中 → 8 右下。只更新锚点标记,
                // 不破坏 avatar_session(未裁原图保留);预览/编码按锚点从原图即时裁切。
                self.avatar_crop_anchor = (self.avatar_crop_anchor + 1) % 9;
                if self.avatar_session.is_some() {
                    let name = ["左上","上中","右上","左中","居中","右中","左下","下中","右下"][self.avatar_crop_anchor as usize];
                    self.set_text(cx, ids!(pf_avatar_anchor), &format!("裁切位置:{name}"));
                    self.profile_err = None;
                    self.refresh_avatar(cx);
                }
            }
            if self.clicked(cx, ids!(pf_avatar_rotate), actions) {
                if let Some(session) = self.avatar_session.take() {
                    self.avatar_session = Some(session.rotate_quarters(1));
                    self.profile_err = None;
                    self.refresh_avatar(cx);
                }
            }
            if self.clicked(cx, ids!(pf_avatar_confirm), actions) {
                if self.avatar_session.is_some() {
                    self.avatar_upload(cx);
                }
            }
            if self.clicked(cx, ids!(pf_avatar_cancel), actions) {
                self.avatar_session = None;
                self.profile_err = None;
                self.refresh_avatar(cx);
            }
            if self.clicked(cx, ids!(pf_avatar_delete), actions) {
                self.avatar_remove(cx);
            }

            if self.clicked(cx, ids!(pf_save), actions) {
                let name = self.input_text(cx, ids!(pf_name));
                // 头像不再走 URL 输入框：保存时带上当前 avatar_url（服务端标识
                // 或 pending: 占位），由 save_profile 按长度校验后写回。
                let avatar = self.profile.avatar_url.clone();
                self.profile_err = profile_client::save_profile(&mut self.profile, &name, &avatar).err();
                if self.profile_err.is_none() { self.toast(cx, "资料已保存"); self.refresh_me(cx); }
                self.refresh_profile(cx);
            }
            if self.clicked(cx, ids!(pf_phone_save), actions) {
                let value = self.input_text(cx, ids!(pf_phone));
                let code = self.input_text(cx, ids!(pf_phone_code));
                self.profile_err = profile_client::bind_contact(&mut self.profile, true, &value, &code).err();
                if self.profile_err.is_none() { self.toast(cx, "手机号已保存"); }
                self.refresh_profile(cx);
            }
            if self.clicked(cx, ids!(pf_email_save), actions) {
                let value = self.input_text(cx, ids!(pf_email));
                let code = self.input_text(cx, ids!(pf_email_code));
                self.profile_err = profile_client::bind_contact(&mut self.profile, false, &value, &code).err();
                if self.profile_err.is_none() { self.toast(cx, "邮箱已保存"); }
                self.refresh_profile(cx);
            }
            if self.clicked(cx, ids!(pf_addr_add), actions) {
                let name = self.input_text(cx, ids!(pf_addr_name));
                let phone = self.input_text(cx, ids!(pf_addr_phone));
                let address = self.input_text(cx, ids!(pf_addr_text));
                self.profile_err = if let Some(id) = self.profile_edit_address {
                    profile_client::update_address(&mut self.profile, id, &name, &phone, &address).err()
                } else {
                    profile_client::add_address(&mut self.profile, &name, &phone, &address).err()
                };
                if self.profile_err.is_none() {
                    self.toast(cx, "地址已保存");
                    self.profile_edit_address = None;
                    self.set_text(cx, ids!(pf_addr_name), "");
                    self.set_text(cx, ids!(pf_addr_phone), "");
                    self.set_text(cx, ids!(pf_addr_text), "");
                }
                self.refresh_profile(cx);
            }
            if self.clicked(cx, ids!(pf_addr_cancel), actions) {
                self.profile_edit_address = None;
                self.profile_err = None;
                self.set_text(cx, ids!(pf_addr_name), "");
                self.set_text(cx, ids!(pf_addr_phone), "");
                self.set_text(cx, ids!(pf_addr_text), "");
                self.refresh_profile(cx);
            }
        }
        // AI 草稿闸:本人确认(仅本机记录为已确认,不发送/不扣款)或取消。
        if self.clicked(cx, ids!(me_draft_confirm), actions) {
            let account = profile_client::active_identifier();
            let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            // 先算 outcome 并取回草稿(结束 draft_gate 不可变借用),再 mutate self。
            let (confirmed, updated) = if let (Some(g), Some(d)) = (self.draft_gate.as_ref(), self.draft_pending.clone()) {
                (g.confirm(&account, &d.draft_id, now_ms).is_confirmed(), g.draft(&d.draft_id))
            } else {
                (false, None)
            };
            if self.draft_pending.is_some() {
                self.toast(cx, if confirmed { "草稿已确认(本机记录,未发送)" } else { "草稿已不可确认(超时/已取消)" });
                self.draft_pending = updated;
                self.refresh_me(cx);
            }
        }
        if self.clicked(cx, ids!(me_draft_cancel), actions) {
            let account = profile_client::active_identifier();
            let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            let cancelled = if let (Some(g), Some(d)) = (self.draft_gate.as_ref(), self.draft_pending.clone()) {
                g.cancel(&account, &d.draft_id, now_ms);
                true
            } else {
                false
            };
            if cancelled {
                self.toast(cx, "草稿已取消");
                self.draft_pending = None;
                self.refresh_me(cx);
            }
        }

        for (i, cat) in account_nav::AccountCategory::ALL.iter().enumerate() {
            if self.toggled(cx, &[account_ui::LIST_CATEGORY_IDS[i]], actions) {
                if self.account_has_unsaved_changes(cx) {
                    self.toast(cx, "有未保存的修改，请保存或取消");
                    self.refresh_account_nav(cx);
                    continue;
                }
                if self.account_edit.is_some() { self.cancel_account_edit(cx); }
                self.account_section = None;
                self.back_stack.clear();
                self.account_nav.get_or_insert_with(account_nav::AccountNavModel::new).select(*cat);
                match i {
                    0..=2 if self.overlay != Some(Overlay::Profile) => self.open_profile(cx),
                    3..=5 if self.overlay != Some(Overlay::Settings) => self.open_settings(cx),
                    0..=5 => {},
                    _ => { self.overlay = None; self.update_page_visibility(cx); }
                }
                self.refresh_account_nav(cx);
            }
        }
        if self.clicked(cx, ids!(acc_back), actions) { self.go_back(cx); }
        for (id, field) in [(live_id!(pf_name_edit),0),(live_id!(pf_phone_edit),1),(live_id!(pf_email_edit),2),(live_id!(pf_addr_new),3),(live_id!(nk_edit),4)] {
            if self.clicked(cx, &[id], actions) {
                self.account_edit = Some(field);
                let initial = match field {0=>self.profile.display_name.clone(),1=>self.profile.phone.clone(),2=>self.profile.email.clone(),4=>self.state.settings.nickname.clone(),_=>String::new()};
                self.account_nav.as_mut().unwrap().begin_edit(field.to_string(), initial);
                self.profile_err = None;
                if field == 3 {
                    self.profile_edit_address = None;
                    self.set_text(cx, ids!(pf_addr_name), "");
                    self.set_text(cx, ids!(pf_addr_phone), "");
                    self.set_text(cx, ids!(pf_addr_text), "");
                }
                self.refresh_account_nav(cx);
                self.view.widget(cx, ids!(page_me)).redraw(cx);
            }
        }
        if self.clicked(cx, ids!(account_cancel), actions) { self.cancel_account_edit(cx); }
        let account_enter = match self.account_edit {
            Some(0)=>self.view.text_input(cx,ids!(pf_name)).returned(actions).is_some(),
            Some(1)=>self.view.text_input(cx,ids!(pf_phone_code)).returned(actions).is_some(),
            Some(2)=>self.view.text_input(cx,ids!(pf_email_code)).returned(actions).is_some(),
            Some(3)=>self.view.text_input(cx,ids!(pf_addr_text)).returned(actions).is_some(),
            Some(4)=>self.view.text_input(cx,ids!(nk_input)).returned(actions).is_some(),
            _=>false,
        };
        if self.clicked(cx, ids!(account_save), actions) || account_enter {
            let err = match self.account_edit {
                Some(0) => { let value=self.input_text(cx,ids!(pf_name)); let avatar=self.profile.avatar_url.clone(); profile_client::save_profile(&mut self.profile,&value,&avatar).err() },
                Some(field @ (1|2)) => { let value=self.input_text(cx, if field==1 {ids!(pf_phone)} else {ids!(pf_email)}); let code=self.input_text(cx, if field==1 {ids!(pf_phone_code)} else {ids!(pf_email_code)}); profile_client::bind_contact(&mut self.profile,field==1,&value,&code).err() },
                Some(4) => { let value=self.input_text(cx,ids!(nk_input)); let e=self.state.set_nickname(&value).err(); if e.is_none() {self.state.save();} e },
                Some(3) => { let name=self.input_text(cx,ids!(pf_addr_name)); let phone=self.input_text(cx,ids!(pf_addr_phone)); let address=self.input_text(cx,ids!(pf_addr_text)); if let Some(id)=self.profile_edit_address {profile_client::update_address(&mut self.profile,id,&name,&phone,&address).err()} else {profile_client::add_address(&mut self.profile,&name,&phone,&address).err()} },
                _ => None,
            };
            self.profile_err = err;
            if err.is_none() {
                let value=match self.account_edit {Some(0)=>self.profile.display_name.clone(),Some(1)=>self.profile.phone.clone(),Some(2)=>self.profile.email.clone(),Some(4)=>self.state.settings.nickname.clone(),_=>String::new()};
                self.account_edit=None;
                if let Some(nav)=self.account_nav.as_mut() {nav.update_draft(value);nav.commit_edit();}
                self.profile_edit_address=None;
                self.toast(cx,"已保存");
            }
            self.refresh_profile(cx);
            self.view.widget(cx, ids!(page_me)).redraw(cx);
        }

        if self.overlay == Some(Overlay::DirectOrder) {
            let choices=self.contact_delivery_choices().into_iter()
                .skip(self.direct_contact_page*CART_FRIEND_CHIPS.len())
                .take(CART_FRIEND_CHIPS.len()).collect::<Vec<_>>();
            for (i,chip) in CART_FRIEND_CHIPS.iter().enumerate(){
                if self.toggled(cx,&[*chip],actions){
                    if let Some((label,_,value))=choices.get(i){
                        self.direct_contact_idx=i;
                        self.set_text(cx,ids!(ca_recipient_label),label);
                        self.set_text(cx,ids!(ca_recipient_value),value);
                        self.set_chip_group(cx,&CART_FRIEND_CHIPS,i);
                    }
                }
            }
            if self.clicked(cx, ids!(ca_checkout), actions) {
                let value=self.input_text(cx,ids!(ca_recipient_value));
                let label=self.input_text(cx,ids!(ca_recipient_label));
                let kind=if value.contains('@'){"email"}else{"phone"};
                self.direct_error=if let Some(product)=self.direct_product{
                    self.commerce.create_order(product,kind,&value,&label).err()
                }else{Some("请先选择商品")};
                if self.direct_error.is_none(){self.toast(cx,"订单已生成，可测试支付");}
                self.refresh_direct_order(cx);
            }
            if self.clicked(cx, ids!(ca_pay), actions) {
                self.direct_error=self.commerce.pay_test().err();
                if self.direct_error.is_none(){self.toast(cx,"测试支付完成，礼物已送出");}
                self.refresh_direct_order(cx);
            }
        }

        // 商品详情、结算、心愿单几张页的动作在 wishui.rs。
        self.handle_wish_actions(cx, actions);
    }
}

impl Widget for LiyuView {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let full = cx.peek_walk_turtle(walk);
        // 以本 tile 实际拿到的尺寸（而非窗口尺寸）决定手机 / 平板 / 桌面形态。
        if full.size.x > 1.0 && full.size.y > 1.0 {
            self.update_responsive(cx, full.size);
        }
        self.view.draw_walk(cx, scope, walk)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.handle_keyboard_navigation(cx, event) {
            return;
        }
        // Consume child actions here, including when hosted inside an isolated module.
        let actions = cx.capture_actions(|cx| self.view.handle_event(cx, event, scope));
        if !actions.is_empty() {
            self.handle_actions(cx, &actions);
        }
        if !self.initialized {
            // 演示便利：状态目录缺 contacts.vcf 时补一份示例，让导入按钮开箱可点。
            // Real file import never manufactures a contacts file.
            self.pal = Pal::read(cx);
            self.state.sweep(today_days());
            // 先恢复本机会话:上次登录过就直接回账号;没有凭据 → 认证闸。
            profile_client::restore_session();
            self.server_error = profile_client::check_server().err();
            self.auth_gate = self.server_error.is_some() || !profile_client::has_choice();
            if self.server_error.is_none() {
                self.profile = profile_client::load(&self.state.settings.nickname);
            }
            self.refresh_all(cx);
            self.set_tab(cx, 0);
            if !self.auth_gate && !self.state.settings.onboarded {
                self.open_intro(cx, 0);
            }
            if let Some(note) = self.state.load_note.take() {
                self.toast(cx, note);
            }
            self.notice_poll = cx.start_interval(60.0);
            self.initialized = true;
            self.poll_notices(cx);
        }
        // 独立窗口换主题走的是 app_main 的 LiveEdit，Rebake 会把 DSL 里的文案刷回去。
        if let Event::LiveEdit = event {
            self.after_restyle(cx);
        }
        // 窗口关闭:未确认的 AI 草稿作废(旧确认不可复用)。
        if let Event::WindowCloseRequested(_) = event {
            let account = profile_client::active_identifier();
            let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            if let (Some(g), Some(d)) = (self.draft_gate.as_ref(), self.draft_pending.clone()) {
                g.window_closed(&account, &d.draft_id, now_ms);
                self.draft_pending = None;
            }
        }
        if self.contact_import_poll.is_event(event).is_some(){
            let result=self.contact_import_receiver.as_ref().and_then(|rx|rx.try_recv().ok());
            if let Some(result)=result{self.contact_import_receiver=None;cx.stop_timer(self.contact_import_poll);self.contact_import_poll=Timer::empty();match result{Ok(import)=>self.stage_contacts(cx,import),Err(e)=>self.toast(cx,e)}}
        }
        if self.notice_poll.is_event(event).is_some() {
            self.poll_notices(cx);
        }
        if self.toast_timer.is_event(event).is_some() {
            self.toast_timer = Timer::empty();
            self.close_toast(cx, false);
        }
        if let Event::Actions(actions) = event {
            self.handle_actions(cx, actions);
        }
        // 这一拍的 action 都走完了，现在换主题才不会把半路用到的 widget 引用换掉。
        if let Some(mode) = self.pending_theme.take() {
            self.apply_theme(cx, mode);
        }
    }
}

// ---------------------------------------------------------------------------
// 小函数

fn focus_index(current: Option<usize>, count: usize, backwards: bool) -> usize {
    match current {
        Some(index) if backwards => (index + count - 1) % count,
        Some(index) => (index + 1) % count,
        None if backwards => count - 1,
        None => 0,
    }
}
// ---------------------------------------------------------------------------

/// 把一条 id 路径接上一个子 id（makepad 的查找是「往下找同名后代」）。
fn join(base: &[LiveId], id: LiveId) -> Vec<LiveId> {
    let mut v = Vec::with_capacity(base.len() + 1);
    v.extend_from_slice(base);
    v.push(id);
    v
}

/// 「送给TA」→「送给 TA」：拉丁字母开头的称呼和前面的汉字之间留一格。
fn spaced(name: &str) -> String {
    if name.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) {
        format!(" {name}")
    } else {
        name.to_string()
    }
}

/// 最后到手的那件的目录下标（换购过就是新的那件）。
fn final_index(g: &Gift) -> u16 {
    if g.swap_item != u16::MAX {
        g.swap_item
    } else {
        g.item
    }
}

fn kind_text(it: &CatalogItem) -> &'static str {
    if it.physical {
        "实物"
    } else {
        "电子券"
    }
}

/// 「实物 · 250g 一盒」。
fn item_sub(it: &CatalogItem) -> String {
    format!("{} · {}", kind_text(it), it.spec)
}

/// 目录里最便宜的一件（回礼预算连最便宜的都不够时的兜底）。
fn cheapest_item() -> u16 {
    (0..CATALOG.len() as u16).min_by_key(|&i| item(i).price).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 模块形态
// ---------------------------------------------------------------------------

pub struct LiyuModule;
pub static LIYU_MODULE: LiyuModule = LiyuModule;

impl AppModule for LiyuModule {
    fn id(&self) -> &'static str {
        "liyu"
    }
    fn label(&self) -> &'static str {
        "礼遇 LiYu"
    }
    fn register(&self, vm: &mut ScriptVm) {
        // 色板先进 VM：后面两个 script_mod 里的预设都按 `liyu.<角色>` 取色。
        theme::install(vm);
        canvas::script_mod(vm);
        account_ui::script_mod(vm);
        script_mod(vm);
    }
    fn open_schema(&self) -> OpenSchema {
        OpenSchema::new(1)
    }
    fn capabilities(&self) -> &'static [&'static str] {
        &[]
    }
    fn create(&self, vm: &mut ScriptVm, _open: ValidatedOpen, _handles: InstanceHandles) -> InstanceParts {
        let value = script_eval!(vm, {
            use mod.widgets.*
            LiyuView {}
        });
        let root = WidgetRef::script_from_value(vm, value);
        InstanceParts {
            root: root.clone(),
            executor: Box::new(LiyuExecutor { root }),
            shutdown: Box::new(|_| {}),
        }
    }
}

/// 模块形态的 executor：借用视图取礼盒的匿名汇总再应答。
struct LiyuExecutor {
    root: WidgetRef,
}

impl ServiceExecutor for LiyuExecutor {
    fn manifest(&self) -> ServiceManifest {
        // 只读投影(ai.rs)+ 草稿闸(ai_draft.rs)两份清单合并发布。
        merge_manifests(ai::manifest(), ai_draft::manifest())
    }
    fn execute(&mut self, _cx: &mut Cx, call: &ServiceCall) -> ExecOutcome {
        let result = self
            .root
            .borrow_mut::<LiyuView>()
            .map(|mut view| view.ai_answer(_cx, call))
            .unwrap_or_else(|| ToolResult::unavailable(&call.call_id, "礼遇窗口已关闭"));
        ExecOutcome::Done(result)
    }
}

#[cfg(test)]
mod layout_tests {
    #[test]
    fn keyboard_focus_starts_and_wraps_in_both_directions() {
        assert_eq!(super::focus_index(None, 5, false), 0);
        assert_eq!(super::focus_index(None, 5, true), 4);
        assert_eq!(super::focus_index(Some(4), 5, false), 0);
        assert_eq!(super::focus_index(Some(0), 5, true), 4);
        assert_eq!(super::focus_index(Some(0), 1, true), 0);
    }

    use super::*;

    fn size(x: f64, y: f64) -> Vec2d {
        Vec2d { x, y }
    }

    #[test]
    fn account_categories_render_and_edit_cancel_preserves_values_at_all_sizes() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let root = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            LIYU_MODULE.register(vm);
            let value = script_eval!(vm, { use mod.widgets.* LiyuView {} });
            WidgetRef::script_from_value(vm, value)
        });
        let mut view = root.borrow_mut::<LiyuView>().unwrap();
        view.tab = 4;
        view.auth_gate = false;
        view.profile.display_name = "原来的名字".into();
        for (width, height) in [
            (412.0, 892.0),
            (820.0, 700.0),
            (1000.0, 700.0),
            (1280.0, 800.0),
        ] {
            view.last_size = size(width, height);
            view.shaping = Some(shaping_for(view.last_size));
            view.account_nav = Some(account_nav::AccountNavModel::new());
            view.overlay = None;
            view.update_page_visibility(&mut cx);
            assert!(view.view.widget(&cx, ids!(me_list)).visible());
            for cat in account_nav::AccountCategory::ALL {
                view.account_nav.as_mut().unwrap().select(cat);
                view.overlay = match cat {
                    account_nav::AccountCategory::Profile
                    | account_nav::AccountCategory::AccountContact
                    | account_nav::AccountCategory::ShippingAddress => Some(Overlay::Profile),
                    account_nav::AccountCategory::About => None,
                    _ => Some(Overlay::Settings),
                };
                view.update_page_visibility(&mut cx);
                assert!(view.view.widget(&cx, ids!(page_me)).visible());
                assert!(view.view.widget(&cx, ids!(account_detail)).visible());
                assert_eq!(
                    view.view.widget(&cx, ids!(me_list)).visible(),
                    width >= 820.0
                );
                assert_eq!(
                    view.view.widget(&cx, ids!(pf_name_card)).visible(),
                    cat == account_nav::AccountCategory::Profile
                );
                assert_eq!(
                    view.view.widget(&cx, ids!(pf_contact_card)).visible(),
                    cat == account_nav::AccountCategory::AccountContact
                );
                assert_eq!(
                    view.view.widget(&cx, ids!(pf_address_card)).visible(),
                    cat == account_nav::AccountCategory::ShippingAddress
                );
                assert_eq!(
                    view.view.widget(&cx, ids!(ntf_card)).visible(),
                    cat == account_nav::AccountCategory::Notifications
                );
            }
            view.account_nav
                .as_mut()
                .unwrap()
                .select(account_nav::AccountCategory::Profile);
            view.overlay = Some(Overlay::Profile);
            view.account_edit = Some(0);
            view.set_text(&mut cx, ids!(pf_name), "尚未保存");
            view.refresh_account_nav(&mut cx);
            assert!(view.view.widget(&cx, ids!(account_edit_bar)).visible());
            assert!(view.view.widget(&cx, ids!(pf_name_editor)).visible());
            assert!(!view.view.widget(&cx, ids!(pf_name_summary)).visible());
            view.go_back(&mut cx);
            assert_eq!(view.account_edit, Some(0));
            view.cancel_account_edit(&mut cx);
            assert_eq!(view.profile.display_name, "原来的名字");
            assert_eq!(view.input_text(&mut cx, ids!(pf_name)), "原来的名字");
            assert!(!view.view.widget(&cx, ids!(account_edit_bar)).visible());
            // Opening the editor alone is not a change: Back closes it normally.
            view.account_edit = Some(0);
            view.account_nav
                .as_mut()
                .unwrap()
                .begin_edit("0", "原来的名字");
            assert!(!view.account_has_unsaved_changes(&mut cx));
            view.go_back(&mut cx);
            assert_eq!(view.account_edit, None);
            assert_eq!(view.account_nav.as_ref().unwrap().depth(), 2);
            view.go_back(&mut cx);
            assert!(view.account_nav.as_ref().unwrap().depth() == 1);
        }
    }

    #[test]
    fn account_data_menu_keeps_desktop_menu_and_mobile_return_at_all_sizes() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let root = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            LIYU_MODULE.register(vm);
            let value = script_eval!(vm, { use mod.widgets.* LiyuView {} });
            WidgetRef::script_from_value(vm, value)
        });
        let mut view = root.borrow_mut::<LiyuView>().unwrap();
        view.tab = 4;
        view.auth_gate = false;
        view.box_rows = vec![999];
        view.box_sent = true;
        for (width, height) in [
            (412.0, 892.0),
            (820.0, 700.0),
            (1000.0, 700.0),
            (1280.0, 800.0),
        ] {
            view.last_size = size(width, height);
            view.shaping = Some(shaping_for(view.last_size));
            view.account_nav = Some(account_nav::AccountNavModel::new());
            view.update_page_visibility(&mut cx);
            for section in AccountSection::ALL {
                let uid = view.view.widget(&cx, &[section.id()]).widget_uid();
                let actions =
                    cx.capture_actions(|cx| cx.widget_action(uid, CheckBoxAction::Change(true)));
                view.handle_actions(&mut cx, &actions);
                assert_eq!(view.tab, 4);
                assert_eq!(view.account_section, Some(section));
                assert!(view.view.widget(&cx, ids!(workspace_pages)).visible());
                assert_eq!(
                    view.view.widget(&cx, ids!(me_list)).visible(),
                    width >= 820.0
                );
                let page = match section {
                    AccountSection::Wishes => live_id!(page_wishes),
                    AccountSection::Received => live_id!(page_account_received),
                    AccountSection::Sent => live_id!(page_account_sent),
                    AccountSection::Drafts => live_id!(page_me),
                    AccountSection::Wallet => live_id!(page_wallet),
                };
                assert!(
                    view.view.widget(&cx, &[page]).visible(),
                    "{}",
                    section.label()
                );
                if width >= 820.0 {
                    assert!(view.content_w <= width - SIDEBAR_W - 200.0 - 20.0);
                    let tile = view.view.view(&cx, ids!(wp0)).borrow().unwrap().walk.width;
                    assert!(matches!(tile, Size::Fixed(w) if w < view.content_w / 2.0));
                }
                assert!(!view.view.widget(&cx, ids!(page_box)).visible());
                assert_eq!(view.box_rows, vec![999]);
                assert!(view.box_sent);
                for other in AccountSection::ALL {
                    assert_eq!(
                        view.view.check_box(&cx, &[other.id()]).active(&cx),
                        other == section
                    );
                }
                for id in account_ui::LIST_CATEGORY_IDS {
                    assert!(!view.view.check_box(&cx, &[id]).active(&cx));
                }
                view.go_back(&mut cx);
                assert_eq!(view.tab, 4);
                assert_eq!(view.account_section, None);
                assert_eq!(view.overlay, None);
                assert!(view.view.widget(&cx, ids!(me_list)).visible());
                assert_eq!(
                    view.view.widget(&cx, ids!(workspace_pages)).visible(),
                    width >= 820.0
                );
            }
        }
    }

    #[test]
    fn data_menu_does_not_discard_an_unsaved_profile_edit_or_its_selection() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let root = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            LIYU_MODULE.register(vm);
            let value = script_eval!(vm, { use mod.widgets.* LiyuView {} });
            WidgetRef::script_from_value(vm, value)
        });
        let mut view = root.borrow_mut::<LiyuView>().unwrap();
        view.tab = 4;
        view.last_size = size(1000.0, 700.0);
        view.shaping = Some(shaping_for(view.last_size));
        view.account_nav = Some(account_nav::AccountNavModel::new());
        view.account_nav
            .as_mut()
            .unwrap()
            .select(account_nav::AccountCategory::Profile);
        view.account_nav.as_mut().unwrap().begin_edit("0", "原名");
        view.profile.display_name = "原名".into();
        view.overlay = Some(Overlay::Profile);
        view.account_edit = Some(0);
        view.set_text(&mut cx, ids!(pf_name), "未保存的名字");
        view.update_page_visibility(&mut cx);
        let uid = view.view.widget(&cx, ids!(account_wallet)).widget_uid();
        let actions = cx.capture_actions(|cx| cx.widget_action(uid, CheckBoxAction::Change(true)));
        view.handle_actions(&mut cx, &actions);
        assert_eq!(view.overlay, Some(Overlay::Profile));
        assert_eq!(view.account_section, None);
        assert_eq!(view.account_edit, Some(0));
        assert_eq!(view.input_text(&mut cx, ids!(pf_name)), "未保存的名字");
        assert!(view.view.check_box(&cx, ids!(acc_mcat0)).active(&cx));
        assert!(!view.view.check_box(&cx, ids!(account_wallet)).active(&cx));
    }

    #[test]
    fn account_wishlist_child_returns_to_right_panel_before_menu() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let root = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            LIYU_MODULE.register(vm);
            let value = script_eval!(vm, { use mod.widgets.* LiyuView {} });
            WidgetRef::script_from_value(vm, value)
        });
        let mut view = root.borrow_mut::<LiyuView>().unwrap();
        view.tab = 4;
        view.last_size = size(1000.0, 700.0);
        view.shaping = Some(shaping_for(view.last_size));
        view.open_account_section(&mut cx, AccountSection::Wishes);
        let uid = view.view.widget(&cx, ids!(mw_new)).widget_uid();
        let actions = cx.capture_actions(|cx| {
            cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
        });
        view.handle_actions(&mut cx, &actions);
        assert_eq!(view.overlay, Some(Overlay::WishEdit));
        assert!(view.view.widget(&cx, ids!(me_list)).visible());
        assert_eq!(view.back_stack, vec![Overlay::Wishes]);
        view.go_back(&mut cx);
        assert_eq!(view.overlay, Some(Overlay::Wishes));
        assert!(view.view.widget(&cx, ids!(page_wishes)).visible());
        assert!(view.view.widget(&cx, ids!(me_list)).visible());
        view.go_back(&mut cx);
        assert_eq!(view.overlay, None);
        view.set_tab(&mut cx, 0);
        view.open_wishes(&mut cx);
        assert!(!view.view.widget(&cx, ids!(me_list)).visible());
        assert!(view.view.widget(&cx, ids!(page_wishes)).visible());
    }

    #[test]
    fn account_gift_pages_have_independent_role_rows_and_preserve_box() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let root = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            LIYU_MODULE.register(vm);
            let value = script_eval!(vm, { use mod.widgets.* LiyuView {} });
            WidgetRef::script_from_value(vm, value)
        });
        let mut view = root.borrow_mut::<LiyuView>().unwrap();
        view.box_rows = vec![999];
        view.gift_client.online = true;
        view.gift_client.inbox = vec![gift_client::ReceivedGift {
            id: 11,
            state: "sealed".into(),
            product_id: None,
            sender_name: Some("未揭晓的送礼人".into()),
            ..Default::default()
        }];
        view.gift_client.outbox = vec![gift_client::SentGift {
            id: 22,
            product_id: 0,
            recipient_name: "收礼人".into(),
            state: "sent".into(),
            ..Default::default()
        }];
        view.refresh_account_gifts(&mut cx, false);
        view.refresh_account_gifts(&mut cx, true);
        assert_eq!(view.received_gift_rows, vec![11]);
        assert_eq!(view.sent_gift_rows, vec![22]);
        assert_eq!(view.box_rows, vec![999]);
        assert_eq!(
            view.input_text(&mut cx, ids!(received_gift_0.bx_title)),
            "一份神秘礼物"
        );
        assert!(view
            .input_text(&mut cx, ids!(sent_gift_0.bx_title))
            .contains("收礼人"));
        assert_ne!(
            view.view
                .widget(&cx, ids!(page_account_received))
                .widget_uid(),
            view.view.widget(&cx, ids!(page_box)).widget_uid()
        );
        assert_ne!(
            view.view.widget(&cx, ids!(page_account_sent)).widget_uid(),
            view.view
                .widget(&cx, ids!(page_account_received))
                .widget_uid()
        );
        view.gift_client.online = false;
        view.refresh_account_gifts(&mut cx, false);
        assert!(view.received_gift_rows.is_empty());
        assert!(!view.view.widget(&cx, ids!(received_gift_0)).visible());
    }

    #[test]
    fn account_edit_actions_preserve_input_and_chrome_at_all_sizes() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let root = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            LIYU_MODULE.register(vm);
            let value = script_eval!(vm, { use mod.widgets.* LiyuView {} });
            WidgetRef::script_from_value(vm, value)
        });
        let mut view = root.borrow_mut::<LiyuView>().unwrap();
        view.tab = 4;
        view.auth_gate = false;
        view.profile.display_name = "原来的名字".into();
        for (width, height) in [
            (412.0, 892.0),
            (820.0, 700.0),
            (1000.0, 700.0),
            (1280.0, 800.0),
        ] {
            view.last_size = size(width, height);
            view.shaping = Some(shaping_for(view.last_size));
            view.account_nav = Some(account_nav::AccountNavModel::new());
            for (id, field, category, input) in [
                (
                    live_id!(pf_name_edit),
                    0,
                    account_nav::AccountCategory::Profile,
                    live_id!(pf_name),
                ),
                (
                    live_id!(pf_phone_edit),
                    1,
                    account_nav::AccountCategory::AccountContact,
                    live_id!(pf_phone),
                ),
                (
                    live_id!(pf_email_edit),
                    2,
                    account_nav::AccountCategory::AccountContact,
                    live_id!(pf_email),
                ),
                (
                    live_id!(pf_addr_new),
                    3,
                    account_nav::AccountCategory::ShippingAddress,
                    live_id!(pf_addr_text),
                ),
                (
                    live_id!(nk_edit),
                    4,
                    account_nav::AccountCategory::General,
                    live_id!(nk_input),
                ),
            ] {
                view.account_nav.as_mut().unwrap().select(category);
                view.overlay = Some(if field == 4 {
                    Overlay::Settings
                } else {
                    Overlay::Profile
                });
                view.update_page_visibility(&mut cx);
                let sidebar_visible = view.view.widget(&cx, ids!(sidebar)).visible();
                let topbar_visible = view.view.widget(&cx, ids!(topbar)).visible();
                let uid = view.view.widget(&cx, &[id]).widget_uid();
                let actions = cx.capture_actions(|cx| {
                    cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
                });
                view.handle_actions(&mut cx, &actions);
                assert_eq!(view.account_edit, Some(field));
                assert!(view.view.widget(&cx, ids!(account_edit_bar)).visible());
                view.set_text(&mut cx, &[input], "未保存的输入");
                let text_input = view.view.text_input(&cx, &[input]);
                text_input.set_cursor(
                    &mut cx,
                    makepad_widgets::text::selection::Cursor {
                        index: 3,
                        prefer_next_row: false,
                    },
                    false,
                );
                let before = text_input.selection();
                // Repeat a refresh and identical text assignment: neither clears the draft nor moves the cursor.
                view.refresh_account_nav(&mut cx);
                view.set_text(&mut cx, &[input], "未保存的输入");
                assert_eq!(view.input_text(&mut cx, &[input]), "未保存的输入");
                assert_eq!(text_input.selection().cursor.index, before.cursor.index);
                assert_eq!(
                    view.view.widget(&cx, ids!(sidebar)).visible(),
                    sidebar_visible
                );
                assert_eq!(
                    view.view.widget(&cx, ids!(topbar)).visible(),
                    topbar_visible
                );
                let uid = view.view.widget(&cx, ids!(account_cancel)).widget_uid();
                let actions = cx.capture_actions(|cx| {
                    cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
                });
                view.handle_actions(&mut cx, &actions);
                assert_eq!(view.account_edit, None);
                assert_eq!(view.profile.display_name, "原来的名字");
                assert_eq!(
                    view.view
                        .view(&cx, ids!(account_detail))
                        .borrow()
                        .unwrap()
                        .walk
                        .margin
                        .left,
                    0.0
                );
            }
        }
    }

    #[test]
    fn hidden_lists_refresh_on_entry_instead_of_every_data_change() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let root = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            LIYU_MODULE.register(vm);
            let value = script_eval!(vm, { use mod.widgets.* LiyuView {} });
            WidgetRef::script_from_value(vm, value)
        });
        let mut view = root.borrow_mut::<LiyuView>().unwrap();
        view.auth_gate = false;
        view.tab = 0;
        view.set_text(&mut cx, ids!(me_bal_v), "hidden sentinel");
        view.after_data_change(&mut cx);
        assert_eq!(view.input_text(&mut cx, ids!(me_bal_v)), "hidden sentinel");
        view.set_tab(&mut cx, 4);
        assert_eq!(
            view.input_text(&mut cx, ids!(me_bal_v)),
            yuan(view.state.balance())
        );
    }

    #[test]
    fn unchanged_avatar_reuses_texture_and_changed_bytes_replace_it() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let root = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            LIYU_MODULE.register(vm);
            let value = script_eval!(vm, { use mod.widgets.* LiyuView {} });
            WidgetRef::script_from_value(vm, value)
        });
        let mut view = root.borrow_mut::<LiyuView>().unwrap();
        view.avatar_pending_bytes = Some(PRODUCT_PNGS[0].to_vec());
        view.refresh_avatar(&mut cx);
        let first = view.avatar_tex.clone();
        assert!(first.is_some());
        view.refresh_avatar(&mut cx);
        assert_eq!(view.avatar_tex, first);
        view.avatar_pending_bytes = Some(PRODUCT_PNGS[1].to_vec());
        view.refresh_avatar(&mut cx);
        assert_ne!(view.avatar_tex, first);
        view.avatar_pending_bytes = None;
        view.refresh_avatar(&mut cx);
        assert!(view.avatar_tex.is_none());
        assert!(view.avatar_texture_bytes.is_none());
    }

    #[test]
    fn navigation_returns_after_authentication_without_a_resize() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let root = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            LIYU_MODULE.register(vm);
            let value = script_eval!(vm, {
                use mod.widgets.*
                LiyuView {}
            });
            WidgetRef::script_from_value(vm, value)
        });
        let mut view = root.borrow_mut::<LiyuView>().unwrap();
        for width in [412.0, 1000.0, 1280.0] {
            view.last_size = size(width, 700.0);
            view.shaping = Some(shaping_for(view.last_size));
            view.auth_gate = true;
            view.update_page_visibility(&mut cx);
            assert!(!view.view.widget(&cx, ids!(sidebar)).visible());
            assert!(!view.view.widget(&cx, ids!(tabbar)).visible());

            view.auth_gate = false;
            view.overlay = Some(Overlay::Send);
            view.update_page_visibility(&mut cx);
            assert_eq!(view.view.widget(&cx, ids!(sidebar)).visible(), width >= PHONE_MAX);
            assert_eq!(view.view.widget(&cx, ids!(tabbar)).visible(), width < PHONE_MAX);
            assert!(view.view.widget(&cx, ids!(topbar)).visible());
        }
    }

    #[test]
    fn the_layout_follows_the_surface_it_is_actually_given() {
        assert_eq!(shaping_for(size(412.0, 892.0)).shape, Shape::Phone);
        assert_eq!(shaping_for(size(820.0, 700.0)).shape, Shape::Tablet);
        assert_eq!(shaping_for(size(1280.0, 800.0)).shape, Shape::Desktop);
    }

    #[test]
    fn merged_manifest_has_six_tools_with_draft_act_risk() {
        // 合并清单 = 只读投影 5 个 Read + 草稿闸 1 个 Act(prepare_gift_draft),无重复名。
        let m = merge_manifests(ai::manifest(), ai_draft::manifest());
        m.validate().unwrap();
        assert_eq!(m.tools.len(), 6, "合并后应有 5 只读 + 1 草稿闸");
        use makepad_app_module::makepad_ai_services::wire::Risk;
        let read = m.tools.iter().filter(|t| t.risk == Risk::Read).count();
        let act = m.tools.iter().filter(|t| t.risk == Risk::Act).count();
        assert_eq!(read, 5, "5 个只读工具");
        assert_eq!(act, 1, "1 个 Risk::Act 草稿工具");
        let draft = m.tool("prepare_gift_draft").expect("草稿工具在合并清单里");
        assert_eq!(draft.risk, Risk::Act);
        // brief 不再声称「只有五个只读工具」。
        assert!(m.brief.contains("prepare_gift_draft"), "brief 应提及可准备草稿");
        assert!(m.brief.contains("本机"), "brief 应说明草稿只在本机");
    }

    #[test]
    fn the_aside_column_only_appears_when_the_desktop_shape_has_room() {
        assert!(shaping_for(size(1280.0, 800.0)).aside);
        assert!(!shaping_for(size(820.0, 700.0)).aside);
        assert!(!shaping_for(size(412.0, 892.0)).aside);
    }

    #[test]
    fn a_short_surface_is_flagged_so_the_intro_icons_fold() {
        assert!(shaping_for(size(1400.0, 440.0)).short);
        assert!(!shaping_for(size(1400.0, 800.0)).short);
    }

    #[test]
    fn every_catalog_item_has_a_card_and_a_picture() {
        assert_eq!(GRID_CARDS.len(), CATALOG.len());
        assert_eq!(PICK_CARDS.len(), CATALOG.len());
        assert_eq!(PRODUCT_PNGS.len(), CATALOG.len());
        assert_eq!(CAT_CHIPS.len(), Category::ALL.len() + 1);
        assert_eq!(PICK_CHIPS.len(), Category::ALL.len() + 1);
        // 「同类换一件」一次列一整个品类。
        for c in Category::ALL {
            assert!(catalog_in(Some(c)).len() <= PICK_ROWS.len(), "{:?}", c);
        }
        assert_eq!(SWAP_ROWS.len(), EXCHANGE_CHOICES);
        assert_eq!(CAND_BTNS.len(), CANDIDATE_COUNT);
        assert_eq!(PRESET_CHIPS.len(), PACT_PRESETS.len());
        assert!(cheapest_item() < CATALOG.len() as u16);
    }

    #[test]
    fn every_wish_option_has_a_control() {
        assert_eq!(OCC_CHIPS.len(), OCCASIONS.len());
        assert_eq!(KIND_CHIPS.len(), WISH_KINDS.len());
        assert_eq!(BUDGET_CHIPS.len(), WISH_BUDGETS.len());
        assert_eq!(WISH_ROWS.len(), WISH_MAX_ITEMS);
        assert_eq!(EDIT_ROWS.len(), WISH_MAX_ITEMS);
        assert_eq!(PAY_CHIPS.len(), PAY_METHODS.len());
        // 含糊心愿的每个品类在目录里都至少有一件，送礼的人才有得挑。
        for k in WISH_KINDS {
            assert!(!wish_candidates(&WishItem::vague(k, 0, "")).is_empty(), "{k}");
        }
    }

    #[test]
    fn contact_import_edit_and_delivery_selection_use_real_widget_actions() {
        for width in [412.0, 820.0, 1000.0, 1280.0] {
            let mut cx = Cx::new(Box::new(|_, _| {}));
            let root = cx.with_vm(|vm| {
                makepad_widgets::script_mod(vm);
                LIYU_MODULE.register(vm);
                let value = script_eval!(vm, { use mod.widgets.* LiyuView {} });
                WidgetRef::script_from_value(vm, value)
            });
            let mut view = root.borrow_mut::<LiyuView>().unwrap();
            view.auth_gate = false;
            view.last_size = size(width, 800.0);
            view.shaping = Some(shaping_for(view.last_size));
            view.tab = 3;
            view.state.contacts.clear();
            let csv = "name,phone,email\n同名,13800138000,a@example.test\n同名,+14155550123,b@example.test\n无效,xxx,\n";
            view.stage_contacts(&mut cx, contacts::parse_csv(csv).unwrap());
            assert!(view.state.contacts.is_empty());
            assert!(view.view.widget(&cx, ids!(ct_preview)).visible());
            let uid = view.view.widget(&cx, ids!(ct_import_cancel)).widget_uid();
            let actions = cx.capture_actions(|cx| {
                cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
            });
            view.handle_actions(&mut cx, &actions);
            assert!(view.pending_contacts.is_none());
            assert!(view.state.contacts.is_empty());
            view.stage_contacts(&mut cx, contacts::parse_csv(csv).unwrap());
            let uid = view.view.widget(&cx, ids!(ct_import_confirm)).widget_uid();
            let actions = cx.capture_actions(|cx| {
                cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
            });
            view.handle_actions(&mut cx, &actions);
            assert_eq!(view.state.contacts.len(), 2);
            assert_eq!(
                contacts::choices(&view.state.contacts[0])[0].1,
                "+8613800138000"
            );
            let uid = view.view.widget(&cx, ids!(c0.cr_edit)).widget_uid();
            let actions = cx.capture_actions(|cx| {
                cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
            });
            view.handle_actions(&mut cx, &actions);
            view.set_text(&mut cx, ids!(ct_email), "changed@example.test");
            let uid = view.view.widget(&cx, ids!(ca_btn)).widget_uid();
            let actions = cx.capture_actions(|cx| {
                cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
            });
            view.handle_actions(&mut cx, &actions);
            assert_eq!(
                view.state.contacts[0].emails.as_ref().unwrap(),
                &vec!["changed@example.test".to_string()]
            );
            assert_eq!(view.state.contacts.len(), 2);
            let uid = view.view.widget(&cx, ids!(c0.cr_send)).widget_uid();
            let actions = cx.capture_actions(|cx| {
                cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
            });
            view.handle_actions(&mut cx, &actions);
            assert_eq!(view.tab, 0);
            assert_eq!(
                view.input_text(&mut cx, ids!(ca_recipient_value)),
                "+8613800138000"
            );
            view.overlay = Some(Overlay::Product);
            view.pd_item = 0;
            view.update_page_visibility(&mut cx);
            let uid = view.view.widget(&cx, ids!(pd_direct)).widget_uid();
            let actions = cx.capture_actions(|cx| {
                cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
            });
            view.handle_actions(&mut cx, &actions);
            assert_eq!(view.overlay, Some(Overlay::DirectOrder));
            assert!(view.view.widget(&cx, ids!(page_direct_order)).visible());
            let uid = view.view.check_box(&cx, ids!(ca_f1)).widget_uid();
            let actions =
                cx.capture_actions(|cx| cx.widget_action(uid, CheckBoxAction::Change(true)));
            view.handle_actions(&mut cx, &actions);
            assert_eq!(
                view.input_text(&mut cx, ids!(ca_recipient_value)),
                "changed@example.test"
            );
            // Invalid addresses are rejected without creating an order.
            view.set_text(&mut cx, ids!(ca_recipient_value), "invalid");
            let uid = view.view.widget(&cx, ids!(ca_checkout)).widget_uid();
            let actions = cx.capture_actions(|cx| {
                cx.widget_action(uid, ButtonAction::Clicked(KeyModifiers::default()))
            });
            view.handle_actions(&mut cx, &actions);
            assert!(view.direct_error.is_some());
            assert!(view.commerce.order.is_none());
        }
    }

    #[test]
    fn every_product_picture_decodes() {
        for (i, png) in PRODUCT_PNGS.iter().enumerate() {
            assert!(ImageBuffer::from_png(png).is_ok(), "p{i:02}.png");
        }
        assert!(ImageBuffer::from_png(MYSTERY_PNG).is_ok());
    }
}
