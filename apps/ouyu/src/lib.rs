//! 偶遇 OuYu —— OctoSense 桌面里的演示社交应用（2026-09-17 新版设计 Phase 1）。
//!
//! 核心循环：发布模糊去向 → 匿名机会 → 正常生活中相遇 → 双方互认 → 可选相遇礼
//! → 私人回忆。发布即参与、撤回即退出，没有参与开关；发现页只有匿名光圈，
//! 不知道是谁、几个人。数据模型见 data.rs，设计文档见 ouyu/design/。
pub use makepad_widgets;
use makepad_widgets::*;
use makepad_widgets::makepad_draw::turtle::RowAlign;
use makepad_app_module::{
    AppModule, ExecOutcome, InstanceHandles, InstanceParts, OpenSchema,
    ServiceExecutor, ValidatedOpen,
    makepad_ai_services::wire::{ServiceCall, ServiceManifest, ToolResult},
};

pub mod ai;
pub mod areas;
pub mod canvas;
pub mod data;
pub mod share;

use canvas::{ChartData, OuyuChart, OuyuShareCard};
use data::*;
use share::ShareStyle;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.OuyuView = set_type_default() do #(OuyuView::register_widget(vm)) {
        ..mod.widgets.RectView
        width: Fill height: Fill
        show_bg: true
        draw_bg.color: #x0b1220
        flow: Right

        // Tab 切换淡入: 用应用底色从不透明到透明扫过页面区。
        draw_fade +: { color: #x0b1220 draw_depth: 6.0 }

        // ---- 左侧 208px 侧栏 ----
        sidebar := RoundedView {
            width: 208 height: Fill
            flow: Down
            padding: Inset{left: 12.0, right: 12.0, top: 20.0, bottom: 14.0}
            spacing: 6.0
            draw_bg +: {
                color: #x0d1626
                border_radius: 0.0
                border_size: 0.0
            }

            logo := View {
                width: Fill height: Fit
                flow: Down
                spacing: 2.0
                margin: Inset{left: 6.0, bottom: 18.0}
                title := Label {
                    text: "偶遇 OuYu"
                    draw_text +: {
                        color: #e7edf8
                        text_style +: { font_size: 18.0 }
                    }
                }
                slogan := Label {
                    text: "给生活留一点偶然"
                    draw_text +: {
                        color: #a4b2c9
                        text_style +: { font_size: 12.5 }
                    }
                }
            }
            tab_discover := OuyuTabIcon { text: "发现" draw_icon +: { svg: crate_resource("self:resources/icons/nav-discover.svg") } }
            tab_meet := OuyuTabIcon { text: "相遇" draw_icon +: { svg: crate_resource("self:resources/icons/nav-meet.svg") } }
            tab_contacts := OuyuTabIcon { text: "熟人" draw_icon +: { svg: crate_resource("self:resources/icons/nav-contacts.svg") } }
            tab_memories := OuyuTabIcon { text: "回忆" draw_icon +: { svg: crate_resource("self:resources/icons/nav-memories.svg") } }
            tab_achieve := OuyuTabIcon { text: "我" draw_icon +: { svg: crate_resource("self:resources/icons/nav-me.svg") } }
            sb_spacer := View { width: Fill height: Fill }
            sb_mode_label := Label {
                text: "界面模式"
                margin: Inset{left: 6.0, bottom: 4.0}
                draw_text +: {
                    color: #6b7a99
                    text_style +: { font_size: 12.5 }
                }
            }
            sb_mode := OuyuBtn { width: Fill text: "自动" }
            sb_note := Label {
                width: Fill
                text: "设计草图 · 模拟数据"
                margin: Inset{left: 6.0}
                draw_text +: {
                    wrap: Words
                    color: #x7b8aa3
                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                }
            }
        }

        // ---- 内容壳：手机模式的顶栏 / 底部导航挂在这里 ----
        shell := RoundedView {
            width: Fill height: Fill
            flow: Down
            draw_bg +: {
                color: #x0b1220
                border_color: #x26364c
                border_size: 0.0
                border_radius: 0.0
            }

            // ---- 手机模式顶栏（宽屏隐藏：宽屏的标题在左侧栏）----
            topbar := View {
                visible: false
                width: Fill height: 58
                flow: Right
                align: Align{x: 0.0, y: 0.5}
                padding: Inset{left: 18.0, right: 12.0}
                spacing: 8.0
                tb_title := Label {
                    width: Fill
                    text: "偶遇 OuYu"
                    draw_text +: {
                        color: #ffca91
                        text_style +: { font_size: 15.0 }
                    }
                }
                tb_mode := OuyuBtn { text: "自动" }
            }
            // ---- 内容区 + 右列解释栏 ----
            main := View {
                width: Fill height: Fill
                flow: Right
                spacing: 16.0
                padding: Inset{left: 20.0, right: 20.0, top: 20.0, bottom: 16.0}

                content := View {
                    width: Fill height: Fill
                    flow: Overlay

                    pages := View {
                        width: Fill height: Fill
                        flow: Down

                    // ---- 发现页（首页：发布 + 匿名机会 + 回声 + 互认入口）----
                    page_discover := ScrollYView {
                        width: Fill height: Fill
                        flow: Down
                        spacing: 16.0

                        hero := View {
                            width: Fill height: Fit
                            flow: Down
                            spacing: 6.0
                            hero_title := Label {
                                width: Fill
                                text: "也许，刚好遇见。"
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 24.0 line_spacing: 1.35 }
                                }
                            }
                            hero_sub := Label {
                                width: Fill
                                text: "照常过你的一天。给重逢留一点空间。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                        }

                        // 时间选择：今天 / 明天 / 本周 + 一周日期条。
                        // 进发现页第一眼要回答的问题是「什么时候出门」，
                        // 排序跟着它走（docs/02 第二节）。
                        time_card := OuyuCard2 {
                            width: Fill height: Fit
                            flow: Down
                            padding: 14.0
                            spacing: 12.0
                            tc_head := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                tc_title := OuyuH3 { width: Fill text: "什么时候出门？" }
                                tc_hint := OuyuMuted { width: Fit text: "只影响排序" }
                            }
                            seg_track := OuyuSegTrack {
                                sg0 := OuyuSeg { text: "今天" }
                                sg1 := OuyuSeg { text: "明天" }
                                sg2 := OuyuSeg { text: "本周" }
                            }
                            day_strip := View {
                                width: Fill height: Fit
                                flow: Right
                                spacing: 3.0
                                d0 := OuyuDay { text: "今天" draw_icon +: { svg: crate_resource("self:resources/icons/dot.svg") } }
                                d1 := OuyuDay { text: "明天" draw_icon +: { svg: crate_resource("self:resources/icons/dot.svg") } }
                                d2 := OuyuDay { text: "后天" draw_icon +: { svg: crate_resource("self:resources/icons/dot.svg") } }
                                d3 := OuyuDay { text: "周四" draw_icon +: { svg: crate_resource("self:resources/icons/dot.svg") } }
                                d4 := OuyuDay { text: "周五" draw_icon +: { svg: crate_resource("self:resources/icons/dot.svg") } }
                                d5 := OuyuDay { text: "周六" draw_icon +: { svg: crate_resource("self:resources/icons/dot.svg") } }
                                d6 := OuyuDay { text: "周日" draw_icon +: { svg: crate_resource("self:resources/icons/dot.svg") } }
                            }
                        }

                        // 排行：这一天最可能遇见的地方。分档，不是人数。
                        rank_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: Inset{left: 6.0, right: 6.0, top: 16.0, bottom: 14.0}
                            spacing: 8.0
                            rk_head := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                padding: Inset{left: 12.0, right: 12.0}
                                spacing: 8.0
                                rk_title := OuyuH2 { width: Fill text: "最可能遇见的地方" }
                                rk_badge := OuyuBadgeBlue { width: Fit text: "匿名区域机会" }
                            }
                            rk_note := OuyuMuted {
                                width: Fill
                                margin: Inset{left: 12.0, right: 12.0, bottom: 2.0}
                                text: "只分档，不给人数、身份或距离。"
                            }
                            rk_list := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 4.0
                                rk0 := OuyuAreaRow { }
                                rk1 := OuyuAreaRow { }
                                rk2 := OuyuAreaRow { }
                                rk3 := OuyuAreaRow { }
                                rk4 := OuyuAreaRow { }
                                rk5 := OuyuAreaRow { }
                            }
                            rk_empty := OuyuEmpty {
                                visible: false
                                em_icon := OuyuIcon {
                                    icon_walk: Walk{ width: 28.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/nav-discover.svg") color: #x3c4c6b }
                                }
                                em_text := Label {
                                    width: Fit
                                    text: "这一天还没有足够的机会"
                                    draw_text +: { color: #a4b2c9 text_style +: { font_size: 13.0 } }
                                }
                            }
                            rk_more_head := OuyuGroupHead {
                                margin: Inset{left: 12.0, right: 12.0, top: 8.0, bottom: 2.0}
                                text: "其它片区"
                            }
                            rk_more := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 4.0
                                rm0 := OuyuAreaRow { }
                                rm1 := OuyuAreaRow { }
                                rm2 := OuyuAreaRow { }
                            }
                            sign_card := OuyuCard2 {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                margin: Inset{left: 12.0, right: 12.0, top: 8.0}
                                padding: 14.0
                                spacing: 6.0
                                sign_title := OuyuWarmText { text: "城市小签" }
                                sign_text := OuyuBody { width: Fill text: "今天的小签：去一家没进过的书店，只翻三页。" }
                                sign_note := OuyuMuted { width: Fill text: "与他人行程无关。" }
                            }
                        }

                        // 我的去向：一条紧凑状态条，不再是常驻的 400px 表单。
                        mine_card := OuyuCard2 {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            mn_icon := OuyuIcon {
                                draw_icon +: { svg: crate_resource("self:resources/icons/location.svg") color: #a4b2c9 }
                            }
                            mn_col := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 2.0
                                mn_title := OuyuBody { width: Fill text: "还没写你的去向" }
                                mn_sub := OuyuMuted { width: Fill text: "写了才会进入别人的匿名机会。" }
                            }
                            mn_go := OuyuBtnPrimarySm { width: Fit text: "发布" }
                            mn_edit := OuyuBtn { visible: false width: Fit text: "修改" }
                            mn_withdraw := OuyuBtnDanger { visible: false width: Fit text: "撤回" }
                        }

                        echo_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 18.0
                            spacing: 10.0
                            echo_title := Label {
                                width: Fill
                                text: "留一个轻轻的回声"
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            echo_note := Label {
                                width: Fill
                                text: "匿名、无已读，随行程到期。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            echo_row := View {
                                width: Fill height: Fit
                                flow: Right{wrap: true}
                                wrap_spacing: 8.0
                                spacing: 8.0
                                echo0 := OuyuChip { text: "咖啡" }
                                echo1 := OuyuChip { text: "散步" }
                                echo2 := OuyuChip { text: "吃饭" }
                            }
                            echo_status := Label {
                                visible: false
                                width: Fill
                                text: "已留下回声 · 随行程到期"
                                draw_text +: {
                                    wrap: Words
                                    color: #ffca91
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }

                        met_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 18.0
                            spacing: 12.0
                            met_col := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 4.0
                                met_title := Label {
                                    text: "真的碰到朋友了？"
                                    draw_text +: {
                                        color: #e7edf8
                                        text_style +: { font_size: 14.0 }
                                    }
                                }
                                met_sub := Label {
                                    width: Fill
                                    text: "各自确认，留个纪念。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                            met_go := OuyuBtnPrimary { text: "我们碰到了" }
                        }
                    }

                    // ---- 发布向导（三步：什么时候 / 哪一带 / 想做什么）----
                    //
                    // 发布是一个动作，不是首页上常驻的表单：从发现页的「发布」
                    // 进来，退出即丢草稿（docs/02 第二节）。
                    // 底栏不进滚动区：第 2 步的片区列表有十几行，按钮若跟着内容
                    // 走，人得先滚一屏才能点「下一步」。
                    page_publish := View {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 12.0

                        pw_scroll := ScrollYView {
                        width: Fill height: Fill
                        flow: Down
                        spacing: 16.0

                        pw_top := View {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            spacing: 8.0
                            pw_back := OuyuIconBtn {
                                draw_icon +: { svg: crate_resource("self:resources/icons/chevron-left.svg") }
                            }
                            pw_title := OuyuH1 { width: Fill text: "写一下你的去向" }
                            pw_step := OuyuMuted { width: Fit text: "1 / 3" }
                        }
                        pw_sub := OuyuMuted {
                            width: Fill
                            text: "别人只看到一行模糊文字，没有昵称、头像和位置。"
                        }

                        // 第 1 步：时间
                        pw_s1 := View {
                            width: Fill height: Fit
                            flow: Down
                            spacing: 14.0
                            s1_card := OuyuCard {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 10.0
                                s1_q1 := OuyuH3 { text: "哪一天？" }
                                s1_days := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    pd0 := OuyuChip { text: "今天" }
                                    pd1 := OuyuChip { text: "今天" }
                                    pd2 := OuyuChip { text: "今天" }
                                    pd3 := OuyuChip { text: "今天" }
                                    pd4 := OuyuChip { text: "今天" }
                                    pd5 := OuyuChip { text: "今天" }
                                    pd6 := OuyuChip { text: "今天" }
                                }
                                s1_q2 := OuyuH3 { margin: Inset{top: 6.0} text: "大概什么时候？" }
                                s1_slots := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    ps0 := OuyuChip { text: "上午" }
                                    ps1 := OuyuChip { text: "下午" }
                                    ps2 := OuyuChip { text: "晚间" }
                                }
                                s1_note := OuyuMuted {
                                    width: Fill
                                    text: "只到上午 / 下午 / 晚间，不给具体钟点。"
                                }
                            }
                        }

                        // 第 2 步：片区（搜索 + 最近去过 + 筛选 + 列表）
                        pw_s2 := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 10.0
                            s2_bar := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                s2_search := OuyuInput { }
                                s2_filter := OuyuBtn {
                                    width: Fit
                                    text: "筛选"
                                    draw_icon +: { svg: crate_resource("self:resources/icons/filter.svg") }
                                }
                            }
                            s2_filters := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                s2_kind_head := OuyuGroupHead { text: "片区类型" }
                                s2_kinds := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 6.0
                                    spacing: 6.0
                                    pk0 := OuyuChip { text: "全部" }
                                    pk1 := OuyuChip { text: "商圈" }
                                    pk2 := OuyuChip { text: "公园" }
                                    pk3 := OuyuChip { text: "滨水" }
                                    pk4 := OuyuChip { text: "文化" }
                                    pk5 := OuyuChip { text: "园区" }
                                    pk6 := OuyuChip { text: "校园" }
                                    pk7 := OuyuChip { text: "枢纽" }
                                    pk8 := OuyuChip { text: "生活" }
                                }
                                s2_dist_head := OuyuGroupHead { text: "行政区" }
                                s2_dists := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 6.0
                                    spacing: 6.0
                                    pg0 := OuyuChip { text: "全部" }
                                    pg1 := OuyuChip { text: "朝阳区" }
                                    pg2 := OuyuChip { text: "海淀区" }
                                    pg3 := OuyuChip { text: "东城区" }
                                    pg4 := OuyuChip { text: "西城区" }
                                    pg5 := OuyuChip { text: "丰台区" }
                                    pg6 := OuyuChip { text: "石景山区" }
                                    pg7 := OuyuChip { text: "通州区" }
                                    pg8 := OuyuChip { text: "昌平区" }
                                    pg9 := OuyuChip { text: "大兴区" }
                                    pg10 := OuyuChip { text: "顺义区" }
                                    pg11 := OuyuChip { text: "房山区" }
                                    pg12 := OuyuChip { text: "门头沟区" }
                                    pg13 := OuyuChip { text: "怀柔区" }
                                    pg14 := OuyuChip { text: "密云区" }
                                    pg15 := OuyuChip { text: "平谷区" }
                                    pg16 := OuyuChip { text: "延庆区" }
                                }
                            }
                            s2_recent_head := OuyuGroupHead { text: "最近去过" }
                            s2_recent := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 4.0
                                pr0 := OuyuAreaRow { }
                                pr1 := OuyuAreaRow { }
                                pr2 := OuyuAreaRow { }
                                pr3 := OuyuAreaRow { }
                                pr4 := OuyuAreaRow { }
                            }
                            s2_list_head := OuyuGroupHead { text: "全部片区" }
                            s2_list := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 4.0
                                pa0 := OuyuAreaRow { }
                                pa1 := OuyuAreaRow { }
                                pa2 := OuyuAreaRow { }
                                pa3 := OuyuAreaRow { }
                                pa4 := OuyuAreaRow { }
                                pa5 := OuyuAreaRow { }
                                pa6 := OuyuAreaRow { }
                                pa7 := OuyuAreaRow { }
                                pa8 := OuyuAreaRow { }
                                pa9 := OuyuAreaRow { }
                                pa10 := OuyuAreaRow { }
                                pa11 := OuyuAreaRow { }
                                pa12 := OuyuAreaRow { }
                                pa13 := OuyuAreaRow { }
                                pa14 := OuyuAreaRow { }
                                pa15 := OuyuAreaRow { }
                                pa16 := OuyuAreaRow { }
                                pa17 := OuyuAreaRow { }
                            }
                            s2_empty := OuyuEmpty {
                                visible: false
                                em_icon := OuyuIcon {
                                    icon_walk: Walk{ width: 28.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/search.svg") color: #x3c4c6b }
                                }
                                em_text := Label {
                                    width: Fit
                                    text: "没有匹配的片区"
                                    draw_text +: { color: #a4b2c9 text_style +: { font_size: 13.0 } }
                                }
                            }
                            s2_more := OuyuMuted { width: Fill text: "" }
                        }

                        // 第 3 步：意愿 + 预览
                        pw_s3 := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 14.0
                            s3_card := OuyuCard {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 10.0
                                s3_q := OuyuH3 { text: "想做点什么？" }
                                s3_intents := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    pi0 := OuyuChip { text: "随意走走" }
                                    pi1 := OuyuChip { text: "顺路办事" }
                                    pi2 := OuyuChip { text: "就想出门" }
                                }
                            }
                            s3_prev_head := OuyuGroupHead { text: "别人看到的就是这一行" }
                            s3_card2 := OuyuCard2 {
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 8.0
                                s3_text := OuyuH2 { width: Fill text: "今天下午 · 三里屯一带 · 随意走走" }
                                s3_note := OuyuMuted {
                                    width: Fill
                                    text: "随时可撤回。"
                                }
                            }
                        }

                        }

                        pw_bar := View {
                            width: Fill height: Fit
                            flow: Right{wrap: true}
                            wrap_spacing: 8.0
                            spacing: 10.0
                            align: Align{x: 0.0, y: 0.5}
                            pw_prev := OuyuBtn { visible: false width: Fit text: "上一步" }
                            pw_next := OuyuBtnPrimary { width: Fit text: "下一步" }
                            pw_cancel := OuyuLink { width: Fit text: "放弃" }
                        }
                    }

                    // ---- 相遇页（现场互认流程）----
                    // ---- 相遇页（现场互认）----
                    //
                    // 四个用户可见的态，一次只露一个：① 选人 → ② 定位门槛
                    // → ③ 等待 → ④ 结果。六条结局（成功 / 同地不成立 / 无库存
                    // / 超时 / 信息不一致 / 未授权）都停在同一张结果屏上。
                    page_meet := ScrollYView {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 16.0

                        mp_title := Label {
                            width: Fill
                            text: "这次，真的遇见了。"
                            draw_text +: {
                                wrap: Words
                                color: #e7edf8
                                text_style +: { font_size: 24.0 line_spacing: 1.35 }
                            }
                        }
                        mp_sub := Label {
                            width: Fill
                            text: "先在线下认出彼此，再各自确认。"
                            draw_text +: {
                                wrap: Words
                                color: #a4b2c9
                                text_style +: { font_size: 14.0 line_spacing: 1.35 }
                            }
                        }

                        // ---- ① 选人 ----
                        meet_pick := View {
                            width: Fill height: Fit
                            flow: Down
                            spacing: 10.0
                            pick_card := OuyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: Inset{left: 6.0, right: 6.0, top: 14.0, bottom: 14.0}
                                spacing: 4.0
                                pc_label := Label {
                                    width: Fill
                                    margin: Inset{left: 12.0, right: 12.0, bottom: 4.0}
                                    text: "在场的是谁？"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                mr0 := OuyuPersonRow { }
                                mr1 := OuyuPersonRow { }
                                mr2 := OuyuPersonRow { }
                                mr3 := OuyuPersonRow { }
                                mr4 := OuyuPersonRow { }
                                mr5 := OuyuPersonRow { }
                                pc_empty := Label {
                                    visible: false
                                    width: Fill
                                    margin: Inset{left: 12.0, right: 12.0}
                                    text: "还没有熟人，先到「熟人」页导入。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                            pick_go := OuyuBtnPrimary {
                                width: Fill
                                text: "确认相遇"
                                draw_icon +: { svg: crate_resource("self:resources/icons/chevron-right.svg") }
                            }
                            // 不确认也能留一笔：做成不起眼的文字链接，不跟主按钮抢。
                            pick_plain := OuyuLink {
                                width: Fill
                                text: "本次不确认，只记一笔"
                            }
                            pick_done := Label {
                                visible: false
                                width: Fill
                                text: "已记住这次相遇"
                                draw_text +: {
                                    color: #9be2bf
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                        }

                        // ---- ② 定位门槛（硬门槛：不授权就不建会话）----
                        meet_gate := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 12.0
                            gate_card := OuyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 18.0
                                spacing: 10.0
                                gt_icon := OuyuIconWarm {
                                    icon_walk: Walk{ width: 26.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/location.svg") }
                                }
                                gt_title := Label {
                                    width: Fill
                                    text: "需要定位一次，确认你们在同一个地方"
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 18.0 line_spacing: 1.35 }
                                    }
                                }
                                gt_why := Label {
                                    width: Fill
                                    text: "相遇礼只发给真的在同一处碰上的两个人。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                gt_how := Label {
                                    width: Fill
                                    text: "只在你点确认的那一刻读一次。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                gt_where := Label {
                                    width: Fill
                                    text: "只用来比对是否同地，比对完即丢弃，不上传不留存。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                gt_row := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    gate_allow := OuyuBtnPrimary { text: "开启定位并确认" }
                                    gate_deny := OuyuBtn { text: "暂不开启" }
                                }
                                gt_denied := Label {
                                    visible: false
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #ffca91
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                gt_alt := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    gate_plain := OuyuBtn { text: "只记一笔回忆" }
                                }
                                gate_cancel := OuyuLink { width: Fit text: "返回" }
                            }
                        }

                        // ---- ③ 等待对方确认 ----
                        meet_wait := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 12.0
                            wait_card := OuyuCard {
                                width: Fill height: Fit
                                flow: Down
                                align: Align{x: 0.5, y: 0.0}
                                padding: 18.0
                                spacing: 12.0
                                wt_ring := OuyuRing { }
                                wt_title := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 17.0 line_spacing: 1.35 }
                                    }
                                }
                                wt_count := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #ffca91
                                        text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                    }
                                }
                                wt_note := Label {
                                    width: Fill
                                    text: "没有已读和在线状态。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                                wt_more := OuyuLink { width: Fit text: "对方没有偶遇？" }
                                wt_fold := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 6.0
                                    wf_tip := Label {
                                        width: Fill
                                        text: "让对方输入这串码，或打开链接："
                                        draw_text +: {
                                            wrap: Words
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    wf_code := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: {
                                            wrap: Words
                                            color: #e7edf8
                                            text_style +: { font_size: 26.0 line_spacing: 1.35 }
                                        }
                                    }
                                    wf_url := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: {
                                            wrap: Words
                                            color: #82b5ff
                                            text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                        }
                                    }
                                }
                                wt_cancel := OuyuLink { width: Fit text: "取消本次确认" }
                            }
                        }

                        // ---- ④ 结果（成功与五条异常共用）----
                        meet_result := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 12.0
                            res_card := OuyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 18.0
                                spacing: 8.0
                                rs_mark := View {
                                    width: Fit height: Fit
                                    rs_ok := View {
                                        width: Fit height: Fit
                                        rs_ok_icon := OuyuIcon {
                                            icon_walk: Walk{ width: 40.0 height: Fit }
                                            draw_icon +: { svg: crate_resource("self:resources/icons/check-circle.svg") color: #9be2bf }
                                        }
                                    }
                                    rs_no := View {
                                        visible: false
                                        width: Fit height: Fit
                                        rs_no_icon := OuyuIcon {
                                            icon_walk: Walk{ width: 40.0 height: Fit }
                                            draw_icon +: { svg: crate_resource("self:resources/icons/alert-circle.svg") color: #ffca91 }
                                        }
                                    }
                                }
                                rs_title := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 18.0 line_spacing: 1.35 }
                                    }
                                }
                                rs_sub := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                            }
                            coupon_card := RoundedView {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                padding: 18.0
                                spacing: 10.0
                                draw_bg +: {
                                    color: #ffca91
                                    border_radius: 20.0
                                }
                                ck_head := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    ck_venue := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: {
                                            wrap: Words
                                            color: #x5a3d1e
                                            text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                        }
                                    }
                                    ck_badge := Label {
                                        text: "虚构示例"
                                        draw_text +: {
                                            color: #x5a3d1e
                                            text_style +: { font_size: 11.0 }
                                        }
                                    }
                                }
                                ck_offer := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #x2b1c0d
                                        text_style +: { font_size: 24.0 line_spacing: 1.35 }
                                    }
                                }
                                ck_terms := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #x5a3d1e
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                                ck_meta := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 6.0
                                    spacing: 12.0
                                    ck_token := Label {
                                        text: ""
                                        draw_text +: {
                                            color: #x2b1c0d
                                            text_style +: { font_size: 13.0 }
                                        }
                                    }
                                    ck_expiry := Label {
                                        text: ""
                                        draw_text +: {
                                            color: #x5a3d1e
                                            text_style +: { font_size: 13.0 }
                                        }
                                    }
                                }
                                ck_status := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #x2b1c0d
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                ck_row := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    ck_redeem := OuyuBtnWarm { text: "到店核销" }
                                }
                            }
                            shops_card := OuyuCard {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                padding: 18.0
                                spacing: 2.0
                                sv_head := Label {
                                    width: Fill
                                    text: "可用门店"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                sv0 := OuyuShopRow { }
                                sv1 := OuyuShopRow { }
                                sv2 := OuyuShopRow { }
                                sv_note := Label {
                                    width: Fill
                                    text: "只给大致远近，不给米数。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #x7b8aa3
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                            choice_card := OuyuCard {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                padding: 18.0
                                spacing: 10.0
                                mc_label := Label {
                                    width: Fill
                                    text: "这次回忆怎么留？"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                choice_row := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    ch_save := OuyuChip { text: "保存" }
                                    ch_hidden := OuyuChip { text: "隐藏" }
                                    ch_skip := OuyuChip { text: "不保存" }
                                }
                                ch_note := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                            res_row := View {
                                width: Fill height: Fit
                                flow: Right{wrap: true}
                                wrap_spacing: 8.0
                                spacing: 8.0
                                rs_finish := OuyuBtnPrimary { text: "完成" }
                                rs_retry := OuyuBtn { visible: false text: "再试一次" }
                                rs_plain := OuyuBtn { visible: false text: "只记一笔回忆" }
                            }
                        }
                    }

                    // ---- 熟人页 ----
                    page_contacts := ScrollYView {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

                        ct_title := Label {
                            width: Fill
                            text: "熟人，来自你的生活。"
                            draw_text +: {
                                wrap: Words
                                color: #e7edf8
                                text_style +: { font_size: 24.0 line_spacing: 1.35 }
                            }
                        }
                        ct_sub := Label {
                            width: Fill
                            text: "无需好友申请，也不显示对方是否安装。"
                            draw_text +: {
                                wrap: Words
                                color: #a4b2c9
                                text_style +: { font_size: 14.0 line_spacing: 1.35 }
                            }
                        }
                        ct_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 18.0
                            spacing: 12.0
                            ct_head := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                ct_head_text := Label {
                                    width: Fill
                                    text: "本机熟人 · 3 位"
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 18.0 line_spacing: 1.35 }
                                    }
                                }
                                ct_import := OuyuBtn { text: "导入联系人" }
                            }
                            ct_note := Label {
                                width: Fill
                                text: "次数含隐藏的回忆；成就页只算未隐藏的。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            // 手动添加：不是每个人都愿意让应用读整本通讯录，
                            // 也不是每个熟人都在通讯录里。
                            ct_add := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                ca_input := OuyuInput {
                                    empty_text: "写一个称呼，比如「老陈」"
                                }
                                ca_btn := OuyuBtn { text: "添加" }
                            }
                            ca_err := Label {
                                visible: false
                                width: Fill
                                text: ""
                                draw_text +: {
                                    wrap: Words
                                    color: #ff9eab
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            // 合并模式下的说明条。
                            ct_merge_bar := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                cm_text := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #ffca91
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                                cm_cancel := OuyuBtn { text: "取消合并" }
                            }
                            cl0 := OuyuGroupHead { visible: false text: "" }
                            ct0 := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_name := Label {
                                        width: 90
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    c_count := Label {
                                        width: Fill
                                        draw_text +: {
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                    c_view := OuyuBtn { text: "回忆" }
                                    c_merge := OuyuBtn { text: "合并" }
                                    c_delmem := OuyuBtn { text: "清空回忆" }
                                    c_del := OuyuBtnDanger { text: "删除" }
                                }
                                c_confirm := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_also := OuyuChip { visible: false text: "同时删除回忆" }
                                    c_ctext := Label {
                                        width: Fill
                                        draw_text +: {
                                            wrap: Words
                                            color: #ff9eab
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    c_yes := OuyuBtnDanger { text: "确认删除" }
                                    c_no := OuyuBtn { text: "取消" }
                                }
                            }
                            cl1 := OuyuGroupHead { visible: false text: "" }
                            ct1 := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_name := Label {
                                        width: 90
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    c_count := Label {
                                        width: Fill
                                        draw_text +: {
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                    c_view := OuyuBtn { text: "回忆" }
                                    c_merge := OuyuBtn { text: "合并" }
                                    c_delmem := OuyuBtn { text: "清空回忆" }
                                    c_del := OuyuBtnDanger { text: "删除" }
                                }
                                c_confirm := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_also := OuyuChip { visible: false text: "同时删除回忆" }
                                    c_ctext := Label {
                                        width: Fill
                                        draw_text +: {
                                            wrap: Words
                                            color: #ff9eab
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    c_yes := OuyuBtnDanger { text: "确认删除" }
                                    c_no := OuyuBtn { text: "取消" }
                                }
                            }
                            cl2 := OuyuGroupHead { visible: false text: "" }
                            ct2 := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_name := Label {
                                        width: 90
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    c_count := Label {
                                        width: Fill
                                        draw_text +: {
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                    c_view := OuyuBtn { text: "回忆" }
                                    c_merge := OuyuBtn { text: "合并" }
                                    c_delmem := OuyuBtn { text: "清空回忆" }
                                    c_del := OuyuBtnDanger { text: "删除" }
                                }
                                c_confirm := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_also := OuyuChip { visible: false text: "同时删除回忆" }
                                    c_ctext := Label {
                                        width: Fill
                                        draw_text +: {
                                            wrap: Words
                                            color: #ff9eab
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    c_yes := OuyuBtnDanger { text: "确认删除" }
                                    c_no := OuyuBtn { text: "取消" }
                                }
                            }
                            cl3 := OuyuGroupHead { visible: false text: "" }
                            ct3 := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_name := Label {
                                        width: 90
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    c_count := Label {
                                        width: Fill
                                        draw_text +: {
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                    c_view := OuyuBtn { text: "回忆" }
                                    c_merge := OuyuBtn { text: "合并" }
                                    c_delmem := OuyuBtn { text: "清空回忆" }
                                    c_del := OuyuBtnDanger { text: "删除" }
                                }
                                c_confirm := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_also := OuyuChip { visible: false text: "同时删除回忆" }
                                    c_ctext := Label {
                                        width: Fill
                                        draw_text +: {
                                            wrap: Words
                                            color: #ff9eab
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    c_yes := OuyuBtnDanger { text: "确认删除" }
                                    c_no := OuyuBtn { text: "取消" }
                                }
                            }
                            cl4 := OuyuGroupHead { visible: false text: "" }
                            ct4 := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_name := Label {
                                        width: 90
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    c_count := Label {
                                        width: Fill
                                        draw_text +: {
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                    c_view := OuyuBtn { text: "回忆" }
                                    c_merge := OuyuBtn { text: "合并" }
                                    c_delmem := OuyuBtn { text: "清空回忆" }
                                    c_del := OuyuBtnDanger { text: "删除" }
                                }
                                c_confirm := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_also := OuyuChip { visible: false text: "同时删除回忆" }
                                    c_ctext := Label {
                                        width: Fill
                                        draw_text +: {
                                            wrap: Words
                                            color: #ff9eab
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    c_yes := OuyuBtnDanger { text: "确认删除" }
                                    c_no := OuyuBtn { text: "取消" }
                                }
                            }
                            cl5 := OuyuGroupHead { visible: false text: "" }
                            ct5 := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 8.0
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_name := Label {
                                        width: 90
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    c_count := Label {
                                        width: Fill
                                        draw_text +: {
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                    c_view := OuyuBtn { text: "回忆" }
                                    c_merge := OuyuBtn { text: "合并" }
                                    c_delmem := OuyuBtn { text: "清空回忆" }
                                    c_del := OuyuBtnDanger { text: "删除" }
                                }
                                c_confirm := View {
                                    visible: false
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 8.0
                                    c_also := OuyuChip { visible: false text: "同时删除回忆" }
                                    c_ctext := Label {
                                        width: Fill
                                        draw_text +: {
                                            wrap: Words
                                            color: #ff9eab
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    c_yes := OuyuBtnDanger { text: "确认删除" }
                                    c_no := OuyuBtn { text: "取消" }
                                }
                            }
                            ct_empty := OuyuEmpty {
                                visible: false
                                em_icon := OuyuIcon {
                                    icon_walk: Walk{ width: 28.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/nav-contacts.svg") color: #x3c4c6b }
                                }
                                em_text := Label {
                                    width: Fit
                                    text: "还没有熟人"
                                    draw_text +: { color: #a4b2c9 text_style +: { font_size: 13.0 } }
                                }
                            }
                        }
                        vcf_row := View {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            spacing: 10.0
                            vcf_status := Label {
                                width: Fill
                                text: "从 contacts.vcf 导入，不查询安装状态"
                                draw_text +: {
                                    wrap: Words
                                    color: #x7b8aa3
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            vcf_btn := OuyuBtn { text: "导入 vCard" }
                        }
                        dir_head := View {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            spacing: 10.0
                            dir_label := Label {
                                width: Fill
                                text: "通讯录 · 4 人"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            dir_btn := OuyuBtn { text: "展开" }
                        }
                        dir_list := Label {
                            visible: false
                            width: Fill
                            draw_text +: {
                                wrap: Words
                                color: #x7b8aa3
                                text_style +: { font_size: 12.5 line_spacing: 1.35 }
                            }
                            text: ""
                        }
                    }

                    // ---- 回忆页 ----
                    page_memories := ScrollYView {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

                        mm_title := Label {
                            width: Fill
                            text: "留下一点，想记住的。"
                            draw_text +: {
                                wrap: Words
                                color: #e7edf8
                                text_style +: { font_size: 24.0 line_spacing: 1.35 }
                            }
                        }
                        mm_sub := Label {
                            width: Fill
                            text: "行程不留记录，这里只有你自己记的相遇。"
                            draw_text +: {
                                wrap: Words
                                color: #a4b2c9
                                text_style +: { font_size: 14.0 line_spacing: 1.35 }
                            }
                        }
                        filt_row := View {
                            width: Fill height: Fit
                            flow: Right{wrap: true}
                            wrap_spacing: 8.0
                            spacing: 8.0
                            filt_all := OuyuChip { text: "全部" }
                            filt0 := OuyuChip { text: "" }
                            filt1 := OuyuChip { text: "" }
                            filt2 := OuyuChip { text: "" }
                            filt3 := OuyuChip { text: "" }
                            filt4 := OuyuChip { text: "" }
                            filt5 := OuyuChip { text: "" }
                        }
                        mm_bar := View {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            spacing: 8.0
                            mm_search := OuyuInput {
                                empty_text: "搜称呼或备注"
                            }
                        }
                        // 搜索中显示：说清楚为什么有些东西搜不到。
                        mm_hidden_note := Label {
                            visible: false
                            width: Fill
                            text: "隐藏的回忆不参与搜索。"
                            draw_text +: {
                                wrap: Words
                                color: #x7b8aa3
                                text_style +: { font_size: 12.5 line_spacing: 1.35 }
                            }
                        }
                        sec_mem := Label {
                            text: "回忆"
                            draw_text +: {
                                color: #a4b2c9
                                text_style +: { font_size: 13.0 }
                            }
                        }
                        mh0 := OuyuGroupHead { visible: false text: "" }
                        mem0 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        mh1 := OuyuGroupHead { visible: false text: "" }
                        mem1 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        mh2 := OuyuGroupHead { visible: false text: "" }
                        mem2 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        mh3 := OuyuGroupHead { visible: false text: "" }
                        mem3 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        mh4 := OuyuGroupHead { visible: false text: "" }
                        mem4 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        mh5 := OuyuGroupHead { visible: false text: "" }
                        mem5 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        mh6 := OuyuGroupHead { visible: false text: "" }
                        mem6 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        mh7 := OuyuGroupHead { visible: false text: "" }
                        mem7 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        mem_empty := OuyuEmpty {
                            visible: false
                            em_icon := OuyuIcon {
                                icon_walk: Walk{ width: 28.0 height: Fit }
                                draw_icon +: { svg: crate_resource("self:resources/icons/nav-memories.svg") color: #x3c4c6b }
                            }
                            em_text := Label {
                                width: Fit
                                text: "这里暂时留白"
                                draw_text +: { color: #a4b2c9 text_style +: { font_size: 13.0 } }
                            }
                        }
                        sec_hid := Label {
                            width: Fill
                            text: "已隐藏"
                            draw_text +: {
                                wrap: Words
                                color: #a4b2c9
                                text_style +: { font_size: 13.0 line_spacing: 1.35 }
                            }
                        }
                        hh0 := OuyuGroupHead { visible: false text: "" }
                        hid0 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        hh1 := OuyuGroupHead { visible: false text: "" }
                        hid1 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        hh2 := OuyuGroupHead { visible: false text: "" }
                        hid2 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        hh3 := OuyuGroupHead { visible: false text: "" }
                        hid3 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        hh4 := OuyuGroupHead { visible: false text: "" }
                        hid4 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        hh5 := OuyuGroupHead { visible: false text: "" }
                        hid5 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        hh6 := OuyuGroupHead { visible: false text: "" }
                        hid6 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        hh7 := OuyuGroupHead { visible: false text: "" }
                        hid7 := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            m_date := Label {
                                width: 104
                                draw_text +: {
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        hid_empty := Label {
                            visible: false
                            width: Fill
                            text: "没有隐藏的回忆。"
                            draw_text +: {
                                wrap: Words
                                color: #a4b2c9
                                text_style +: { font_size: 12.5 line_spacing: 1.35 }
                            }
                        }
                        mm_note := Label {
                            width: Fill
                            text: "隐藏的仍会保存并计次，可随时恢复；隐藏不是加密。"
                            draw_text +: {
                                wrap: Words
                                color: #x7b8aa3
                                text_style +: { font_size: 12.5 line_spacing: 1.35 }
                            }
                        }
                    }

                    // ---- 成就页（06 节：摘要 + 频率曲线 + 里程碑 + 分享入口）----
                    page_achieve := ScrollYView {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

                        ac_title := Label {
                            width: Fill
                            text: "那些偶然，慢慢有了形状。"
                            draw_text +: {
                                wrap: Words
                                color: #e7edf8
                                text_style +: { font_size: 24.0 line_spacing: 1.35 }
                            }
                        }
                        ac_sub := Label {
                            width: Fill
                            text: "生活小成就，不是排名。"
                            draw_text +: {
                                wrap: Words
                                color: #a4b2c9
                                text_style +: { font_size: 14.0 line_spacing: 1.35 }
                            }
                        }
                        // 摘要行：数字大号暖杏（06 节口径写明在卡片上）。
                        sum_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 18.0
                            spacing: 10.0
                            sum_head := Label {
                                width: Fill
                                text: "本机统计"
                                draw_text +: {
                                    wrap: Words
                                    color: #ffca91
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            sum_note := Label {
                                width: Fill
                                text: "只算未隐藏的回忆，删除后重新计算。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            sum_row := View {
                                width: Fill height: Fit
                                flow: Right
                                sum_col0 := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 4.0
                                    sum_n0 := Label {
                                        text: "0"
                                        draw_text +: {
                                            color: #ffca91
                                            text_style +: { font_size: 26.0 }
                                        }
                                    }
                                    sum_l0 := Label {
                                        width: Fill
                                        text: "记住的相遇"
                                        draw_text +: {
                                            wrap: Words
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                }
                                sum_col1 := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 4.0
                                    sum_n1 := Label {
                                        text: "0"
                                        draw_text +: {
                                            color: #ffca91
                                            text_style +: { font_size: 26.0 }
                                        }
                                    }
                                    sum_l1 := Label {
                                        width: Fill
                                        text: "有相遇的日子"
                                        draw_text +: {
                                            wrap: Words
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                }
                                sum_col2 := View {
                                    width: Fill height: Fit
                                    flow: Down
                                    spacing: 4.0
                                    sum_n2 := Label {
                                        text: "0"
                                        draw_text +: {
                                            color: #ffca91
                                            text_style +: { font_size: 26.0 }
                                        }
                                    }
                                    sum_l2 := Label {
                                        width: Fill
                                        text: "近 8 周相遇"
                                        draw_text +: {
                                            wrap: Words
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                }
                            }
                        }
                        // 频率曲线：4 / 8 周切换 + 自绘折线 + 可展开数据表。
                        curve_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 18.0
                            spacing: 10.0
                            curve_head := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                curve_title := Label {
                                    width: Fill
                                    text: "相遇频率曲线"
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                    }
                                }
                                wk4 := OuyuChip { text: "4 周" }
                                wk8 := OuyuChip { text: "8 周" }
                            }
                            curve_sub := Label {
                                width: Fill
                                text: "最近 8 周，记住 0 次重逢"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            curve_chart := OuyuChart { width: Fill height: 200 }
                            curve_empty := Label {
                                visible: false
                                width: Fill
                                text: "下一次偶然，值得期待"
                                draw_text +: {
                                    wrap: Words
                                    color: #ffca91
                                    text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                }
                            }
                            curve_caption := Label {
                                width: Fill
                                text: "按周汇总，最后一周未结束；0 只表示没有可见记录。"
                                draw_text +: {
                                    wrap: Words
                                    color: #6b7a99
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            wk_toggle := OuyuBtn { text: "每周次数" }
                            wk_table := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                spacing: 6.0
                                wk_r0 := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    wk_d := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: { color: #a4b2c9 text_style +: { font_size: 12.5 } }
                                    }
                                    wk_c := Label {
                                        text: ""
                                        draw_text +: { color: #ffca91 text_style +: { font_size: 12.5 } }
                                    }
                                }
                                wk_r1 := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    wk_d := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: { color: #a4b2c9 text_style +: { font_size: 12.5 } }
                                    }
                                    wk_c := Label {
                                        text: ""
                                        draw_text +: { color: #ffca91 text_style +: { font_size: 12.5 } }
                                    }
                                }
                                wk_r2 := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    wk_d := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: { color: #a4b2c9 text_style +: { font_size: 12.5 } }
                                    }
                                    wk_c := Label {
                                        text: ""
                                        draw_text +: { color: #ffca91 text_style +: { font_size: 12.5 } }
                                    }
                                }
                                wk_r3 := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    wk_d := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: { color: #a4b2c9 text_style +: { font_size: 12.5 } }
                                    }
                                    wk_c := Label {
                                        text: ""
                                        draw_text +: { color: #ffca91 text_style +: { font_size: 12.5 } }
                                    }
                                }
                                wk_r4 := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    wk_d := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: { color: #a4b2c9 text_style +: { font_size: 12.5 } }
                                    }
                                    wk_c := Label {
                                        text: ""
                                        draw_text +: { color: #ffca91 text_style +: { font_size: 12.5 } }
                                    }
                                }
                                wk_r5 := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    wk_d := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: { color: #a4b2c9 text_style +: { font_size: 12.5 } }
                                    }
                                    wk_c := Label {
                                        text: ""
                                        draw_text +: { color: #ffca91 text_style +: { font_size: 12.5 } }
                                    }
                                }
                                wk_r6 := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    wk_d := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: { color: #a4b2c9 text_style +: { font_size: 12.5 } }
                                    }
                                    wk_c := Label {
                                        text: ""
                                        draw_text +: { color: #ffca91 text_style +: { font_size: 12.5 } }
                                    }
                                }
                                wk_r7 := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    wk_d := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: { color: #a4b2c9 text_style +: { font_size: 12.5 } }
                                    }
                                    wk_c := Label {
                                        text: ""
                                        draw_text +: { color: #ffca91 text_style +: { font_size: 12.5 } }
                                    }
                                }
                            }
                        }
                        // 里程碑：点亮 / 等自然发生，不用凑次数，无排名。
                        ms_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 18.0
                            spacing: 10.0
                            ms_head := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                ms_title := Label {
                                    width: Fill
                                    text: "我的小小里程碑"
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                    }
                                }
                                ms_badge := Label {
                                    text: "不用凑次数"
                                    draw_text +: {
                                        color: #ffca91
                                        text_style +: { font_size: 11.0 }
                                    }
                                }
                            }
                            ms_row := View {
                                width: Fill height: Fit
                                flow: Right
                                spacing: 12.0
                                ms0 := OuyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 14.0
                                    spacing: 6.0
                                    ms_icon := Label {
                                        text: "☆"
                                        draw_text +: {
                                            color: #ffca91
                                            text_style +: { font_size: 17.0 }
                                        }
                                    }
                                    ms_title := Label {
                                        text: "第一次刚刚好"
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 13.0 }
                                        }
                                    }
                                    ms_desc := Label {
                                        text: "记住一次重逢"
                                        draw_text +: {
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                    ms_state := Label {
                                        text: "等自然发生"
                                        draw_text +: {
                                            color: #6b7a99
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                }
                                ms1 := OuyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 14.0
                                    spacing: 6.0
                                    ms_icon := Label {
                                        text: "☆"
                                        draw_text +: {
                                            color: #ffca91
                                            text_style +: { font_size: 17.0 }
                                        }
                                    }
                                    ms_title := Label {
                                        text: "生活有回响"
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 13.0 }
                                        }
                                    }
                                    ms_desc := Label {
                                        text: "记住三次相遇"
                                        draw_text +: {
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                    ms_state := Label {
                                        text: "等自然发生"
                                        draw_text +: {
                                            color: #6b7a99
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                }
                                ms2 := OuyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 14.0
                                    spacing: 6.0
                                    ms_icon := Label {
                                        text: "☆"
                                        draw_text +: {
                                            color: #ffca91
                                            text_style +: { font_size: 17.0 }
                                        }
                                    }
                                    ms_title := Label {
                                        text: "把日常过成故事"
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 13.0 }
                                        }
                                    }
                                    ms_desc := Label {
                                        text: "七个有相遇的日子"
                                        draw_text +: {
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                    ms_state := Label {
                                        text: "等自然发生"
                                        draw_text +: {
                                            color: #6b7a99
                                            text_style +: { font_size: 12.5 }
                                        }
                                    }
                                }
                            }
                            ms_note := Label {
                                width: Fill
                                text: "里程碑随可见回忆变化，不保存也不扣分。"
                                draw_text +: {
                                    wrap: Words
                                    color: #6b7a99
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                        // 分享入口。
                        share_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 18.0
                            spacing: 12.0
                            share_text := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 6.0
                                share_t := Label {
                                    width: Fill
                                    text: "把生活里的偶然，分享给朋友。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                    }
                                }
                                share_b := Label {
                                    width: Fill
                                    text: "分享卡只有你的汇总，没有别人的身份。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                            sh_go := OuyuBtnPrimary { text: "生成分享卡" draw_icon +: { svg: crate_resource("self:resources/icons/share.svg") } }
                        }
                        // 「我」页的三个入口：券包、设置、关于。开发者选项收在设置里 ——
                        // 它属于「这台机器上的调试开关」，不属于第一屏。
                        hub_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 6.0}
                            spacing: 0.0
                            row_wallet := OuyuSetRow { }
                            row_settings := OuyuSetRow { }
                            row_about := OuyuSetRow { }
                        }
                    }

                    // ---- 我的券（「我」页进入的覆盖页）----
                    page_wallet := ScrollYView {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

                        wl_back := OuyuLink {
                            width: Fit
                            text: "返回「我」"
                            draw_icon +: { svg: crate_resource("self:resources/icons/chevron-left.svg") }
                        }
                        wl_title := Label {
                            width: Fill
                            text: "我的券"
                            draw_text +: { wrap: Words color: #e7edf8 text_style +: { font_size: 24.0 line_spacing: 1.35 } }
                        }
                        wl_sub := Label {
                            width: Fill
                            text: "商户赞助，确认相遇后发放，7 天内使用。"
                            draw_text +: { wrap: Words color: #a4b2c9 text_style +: { font_size: 14.0 line_spacing: 1.35 } }
                        }
                        // 02 B 节要求把这件事直接写在券包上，而不是藏进隐私政策。
                        wl_note := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.0}
                            padding: 14.0
                            spacing: 10.0
                            wl_note_icon := OuyuIcon {
                                icon_walk: Walk{ width: 16.0 height: Fit }
                                draw_icon +: { svg: crate_resource("self:resources/icons/lock.svg") color: #9be2bf }
                            }
                            wl_note_text := Label {
                                width: Fill
                                text: "券面只有商户和核销码，没有和谁、在哪、哪天。"
                                draw_text +: { wrap: Words color: #a4b2c9 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                        }
                        wl_state := OuyuEmpty {
                            visible: false
                            em_icon := OuyuIcon {
                                icon_walk: Walk{ width: 28.0 height: Fit }
                                draw_icon +: { svg: crate_resource("self:resources/icons/coupon.svg") color: #x3c4c6b }
                            }
                            em_text := Label {
                                width: Fit
                                text: "还没有相遇礼"
                                draw_text +: { color: #a4b2c9 text_style +: { font_size: 13.0 } }
                            }
                        }
                        wl_avail := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 10.0
                            wl_avail_head := OuyuGroupHead { text: "可用" }
                            wa0 := OuyuCouponCard { visible: false }
                            wa1 := OuyuCouponCard { visible: false }
                            wa2 := OuyuCouponCard { visible: false }
                            wa3 := OuyuCouponCard { visible: false }
                        }
                        wl_used := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 10.0
                            wl_used_head := OuyuGroupHead { text: "已核销" }
                            wu0 := OuyuCouponCard { visible: false }
                            wu1 := OuyuCouponCard { visible: false }
                            wu2 := OuyuCouponCard { visible: false }
                            wu3 := OuyuCouponCard { visible: false }
                        }
                        wl_gone := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 10.0
                            wl_gone_head := OuyuGroupHead { text: "已过期" }
                            wg0 := OuyuCouponCard { visible: false }
                            wg1 := OuyuCouponCard { visible: false }
                            wg2 := OuyuCouponCard { visible: false }
                            wg3 := OuyuCouponCard { visible: false }
                        }
                    }

                    // ---- 设置（「我」页进入的覆盖页）----
                    page_settings := ScrollYView {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

                        se_back := OuyuLink {
                            width: Fit
                            text: "返回「我」"
                            draw_icon +: { svg: crate_resource("self:resources/icons/chevron-left.svg") }
                        }
                        se_title := Label {
                            width: Fill
                            text: "设置"
                            draw_text +: { wrap: Words color: #e7edf8 text_style +: { font_size: 24.0 line_spacing: 1.35 } }
                        }

                        // ---- 定位权限 ----
                        loc_head := OuyuGroupHead { text: "定位" }
                        loc_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 6.0}
                            spacing: 0.0
                            row_loc := OuyuSetRow { }
                        }

                        // ---- 通知 ----
                        ntf_head := OuyuGroupHead { text: "通知" }
                        ntf_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 10.0}
                            spacing: 0.0
                            row_ntf_publish := OuyuSwitchRow { }
                            row_ntf_reward := OuyuSwitchRow { }
                            ntf_note := Label {
                                width: Fill
                                margin: Inset{left: 12.0, right: 12.0, top: 6.0}
                                text: "不做「附近有熟人」这类提醒，那等于实时位置广播。"
                                draw_text +: { wrap: Words color: #x7b8aa3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                        }

                        // ---- 数据与隐私 ----
                        data_head := OuyuGroupHead { text: "数据与隐私" }
                        data_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 10.0}
                            spacing: 0.0
                            row_export := OuyuSetRow { }
                            row_clear := OuyuSetRow { }
                            // 清除不可撤销，所以不走 5 秒 toast，走二次确认。
                            clear_confirm := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                margin: Inset{left: 12.0, right: 12.0, top: 4.0}
                                spacing: 8.0
                                cf_text := Label {
                                    width: Fill
                                    text: "将清掉本机全部数据，无法撤销。要先导出吗？"
                                    draw_text +: { wrap: Words color: #ff9eab text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                                }
                                cf_row := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 8.0
                                    cf_cancel := OuyuBtn { text: "再想想" }
                                    cf_ok := OuyuBtnDanger { text: "确认清除" }
                                }
                            }
                            data_note := Label {
                                width: Fill
                                margin: Inset{left: 12.0, right: 12.0, top: 6.0}
                                text: "所有数据只在本机；导出文件不含坐标和地点。"
                                draw_text +: { wrap: Words color: #x7b8aa3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                        }

                        // ---- 开发者选项 ----
                        dev_head := OuyuGroupHead { text: "开发者选项" }
                        // 开发者选项：草图场景开关，以及「替对方按一下」的模拟入口。
                        //
                        // 这些按钮一旦出现在相遇流程里，人就会以为确认是自己这台
                        // 手机说了算的。它们只属于这里。
                        dev_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 18.0
                            spacing: 10.0
                            dv_head := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                dv_icon := OuyuIcon {
                                    icon_walk: Walk{ width: 16.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/settings.svg") }
                                }
                                dv_title := Label {
                                    width: Fill
                                    text: "开发者选项"
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                    }
                                }
                            }
                            dv_sub := Label {
                                width: Fill
                                text: "仅供演示：真实版本里对方的确认来自对方手机。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                            dv_sparse := OuyuBtn { width: Fill text: "切换稀疏场景" }
                            dv_notice_label := Label {
                                width: Fill
                                text: "通知 · 让到期提醒现在就响"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                }
                            }
                            dv_notice := View {
                                width: Fill height: Fit
                                flow: Right{wrap: true}
                                wrap_spacing: 8.0
                                spacing: 8.0
                                dv_nt_pub := OuyuBtn { text: "去向快到期" }
                                dv_nt_rwd := OuyuBtn { text: "券快过期" }
                            }
                            dv_state_label := Label {
                                width: Fill
                                text: "列表四态"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                }
                            }
                            dv_state := View {
                                width: Fill height: Fit
                                flow: Right{wrap: true}
                                wrap_spacing: 8.0
                                spacing: 8.0
                                dv_ready := OuyuBtn { text: "正常" }
                                dv_loading := OuyuBtn { text: "加载中" }
                                dv_failed := OuyuBtn { text: "出错" }
                                dv_offline := OuyuBtn { text: "离线" }
                            }
                            dv_meet_label := Label {
                                width: Fill
                                text: "现场互认 · 替对方按一下"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                }
                            }
                            dv_meet := View {
                                width: Fill height: Fit
                                flow: Right{wrap: true}
                                wrap_spacing: 8.0
                                spacing: 8.0
                                dv_ok := OuyuBtn { text: "对方也确认" }
                                dv_mismatch := OuyuBtn { text: "信息不一致" }
                                dv_far := OuyuBtn { text: "同地不成立" }
                                dv_stock := OuyuBtn { text: "没有库存" }
                                dv_expire := OuyuBtn { text: "立即超时" }
                            }
                            dv_meet_note := Label {
                                width: Fill
                                text: "需先在相遇页进入等待态。"
                                draw_text +: {
                                    wrap: Words
                                    color: #x7b8aa3
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                        // ---- 关于 ----
                        about_head := OuyuGroupHead { text: "关于" }
                        about_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 18.0
                            spacing: 8.0
                            ab_name := Label {
                                width: Fill
                                text: "偶遇 OuYu · 设计稿 v0.4"
                                draw_text +: { wrap: Words color: #e7edf8 text_style +: { font_size: 15.0 line_spacing: 1.35 } }
                            }
                            ab_p1 := Label {
                                width: Fill
                                text: "偶遇只做一件事：让本来就可能发生的相遇更容易发生一点，然后退开。"
                                draw_text +: { wrap: Words color: #a4b2c9 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                            ab_p2 := Label {
                                width: Fill
                                text: "不做：谁在附近、人数与距离、实时位置、把隐藏回忆算进统计。"
                                draw_text +: { wrap: Words color: #x7b8aa3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                            // 跳过引导的人在这里能重看那三句话（02 七.1）。
                            ab_intro := OuyuBtn { text: "重看开场" }
                        }
                    }

                    // ---- 分享卡预览页（成就页「生成分享卡 →」进入；不是侧栏 Tab）----
                    page_share := ScrollYView {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

                        sp_title := Label {
                            width: Fill
                            text: "让朋友看见，你生活里的光。"
                            draw_text +: {
                                wrap: Words
                                color: #e7edf8
                                text_style +: { font_size: 24.0 line_spacing: 1.35 }
                            }
                        }
                        sp_sub := Label {
                            width: Fill
                            text: "先预览，再决定发不发。"
                            draw_text +: {
                                wrap: Words
                                color: #a4b2c9
                                text_style +: { font_size: 14.0 line_spacing: 1.35 }
                            }
                        }
                        sp_row := View {
                            width: Fill height: Fit
                            flow: Right
                            spacing: 14.0
                            sp_left := OuyuCard {
                                width: Fill height: Fit
                                flow: Down
                                padding: 18.0
                                spacing: 10.0
                                sp_head := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    sp_ht := Label {
                                        width: Fill
                                        text: "分享卡预览"
                                        draw_text +: {
                                            wrap: Words
                                            color: #e7edf8
                                            text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                        }
                                    }
                                    sp_badge := Label {
                                        text: "仅用可见回忆"
                                        draw_text +: {
                                            color: #ffca91
                                            text_style +: { font_size: 11.0 }
                                        }
                                    }
                                }
                                sp_center := View {
                                    width: Fill height: Fit
                                    align: Align{x: 0.5, y: 0.0}
                                    sh_card := OuyuShareCard { width: 300 height: 400 }
                                }
                                sp_btns := View {
                                    width: Fill height: Fit
                                    flow: Right{wrap: true}
                                    wrap_spacing: 8.0
                                    spacing: 10.0
                                    sh_save := OuyuBtnPrimary { text: "保存图片" }
                                    sh_back := OuyuBtn { text: "返回" }
                                }
                                sh_saved := Label {
                                    visible: false
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #ffca91
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                                sp_note := Label {
                                    width: Fill
                                    text: "保存后可在任意社交媒体自行发布。"
                                    draw_text +: {
                                        wrap: Words
                                        color: #6b7a99
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                            sp_right := View {
                                width: 300 height: Fit
                                flow: Down
                                spacing: 14.0
                                sp_style := OuyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 16.0
                                    spacing: 8.0
                                    sps_t := Label {
                                        text: "样式"
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    sps_row := View {
                                        width: Fill height: Fit
                                        flow: Right{wrap: true}
                                        wrap_spacing: 8.0
                                        spacing: 8.0
                                        sh_warm := OuyuChip { text: "暖杏" }
                                        sh_night := OuyuChip { text: "夜蓝" }
                                    }
                                }
                                sp_what := OuyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 16.0
                                    spacing: 8.0
                                    spw_t := Label {
                                        text: "内容"
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    spw_b := Label {
                                        width: Fill
                                        text: "汇总次数、成就文案、署名。"
                                        draw_text +: {
                                            wrap: Words
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    sh_curve := OuyuChip { text: "包含每周曲线" }
                                    sh_curve_hint := Label {
                                        visible: false
                                        width: Fill
                                        text: "会额外透露你的相遇频率"
                                        draw_text +: {
                                            wrap: Words
                                            color: #ffca91
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    spw_note := Label {
                                        width: Fill
                                        text: "曲线会透露近 8 周频率，不含联系人或地点。"
                                        draw_text +: {
                                            wrap: Words
                                            color: #6b7a99
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                }
                                sp_copy := OuyuCard {
                                    width: Fill height: Fit
                                    flow: Down
                                    padding: 16.0
                                    spacing: 8.0
                                    spc_t := Label {
                                        text: "文案灵感"
                                        draw_text +: {
                                            color: #e7edf8
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    spc_b := Label {
                                        width: Fill
                                        text: "不用专程约，刚好遇见。\n给生活留一点偶然。"
                                        draw_text +: {
                                            wrap: Words
                                            color: #ffca91
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    spc_note := Label {
                                        width: Fill
                                        text: "标记朋友前，先问问对方。"
                                        draw_text +: {
                                            wrap: Words
                                            color: #a4b2c9
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                }
                            }
                        }
                        sp_caution := OuyuCard {
                            width: Fill height: Fit
                            padding: 14.0
                            sp_caut := Label {
                                width: Fill
                                text: "汇总次数会透露你的活跃程度；已发出的图片不会随本机删除撤回。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                    }
                    }

                    // 删除提示：5 秒内可撤销，浮在所有页之上、导航之下。
                    //
                    // 不用弹窗问「确定删除吗」—— 删一条回忆不值得打断一次；
                    // 真正不可逆的（清除本机数据）才保留二次确认。
                    // ---- 一条回忆的详情（从回忆页某一行进来）----
                    //
                    // 隐藏 / 恢复 / 删除 / 写备注四件事都收进这里：这四个动作
                    // 全摆在列表行上的时候，一行要塞四个按钮，手机上永远在换行，
                    // 而且「删除」离手指太近。
                    page_memdetail := ScrollYView {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0
                        md_back := OuyuLink { text: "返回「回忆」" }
                        md_title := Label {
                            width: Fill
                            text: ""
                            draw_text +: {
                                wrap: Words
                                color: #e7edf8
                                text_style +: { font_size: 22.0 line_spacing: 1.35 }
                            }
                        }
                        md_date := Label {
                            width: Fill
                            text: ""
                            draw_text +: {
                                wrap: Words
                                color: #a4b2c9
                                text_style +: { font_size: 14.0 line_spacing: 1.35 }
                            }
                        }
                        md_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 18.0
                            spacing: 10.0
                            md_note_head := Label {
                                width: Fill
                                text: "你的备注"
                                draw_text +: { wrap: Words color: #e7edf8 text_style +: { font_size: 15.0 line_spacing: 1.35 } }
                            }
                            md_note := OuyuInput {
                                empty_text: "写一句只给你自己看的"
                            }
                            md_note_tip := Label {
                                width: Fill
                                text: "备注只在本机，不进统计和分享卡。"
                                draw_text +: { wrap: Words color: #x7b8aa3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                            md_save := OuyuBtnPrimary { text: "保存备注" }
                        }
                        md_act_head := OuyuGroupHead { text: "这一条" }
                        md_act := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 18.0
                            spacing: 10.0
                            md_toggle := OuyuBtn { width: Fill text: "隐藏这一条" }
                            md_hide_tip := Label {
                                width: Fill
                                text: "隐藏后不进搜索、提醒和成就，仍计一次相遇。"
                                draw_text +: { wrap: Words color: #a4b2c9 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                            md_del := OuyuBtnDanger { width: Fill text: "删除这一条" }
                            md_del_tip := Label {
                                width: Fill
                                text: "删除后 5 秒内可撤销；只删你这一份。"
                                draw_text +: { wrap: Words color: #x7b8aa3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                        }
                    }

                    // ---- 开场三屏（第一次打开，或从「我」页重看）----
                    //
                    // 盖住整块内容区，并且把侧栏 / 顶栏 / 底部导航一起藏起来 ——
                    // 这三句话要是能被一脚跨过去，就等于没讲。
                    page_intro := View {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0
                        in_top := View {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            spacing: 6.0
                            in_d0 := RoundedView {
                                width: 22 height: 4
                                draw_bg +: { color: #82b5ff border_radius: 2.0 }
                            }
                            in_d1 := RoundedView {
                                width: 22 height: 4
                                draw_bg +: { color: #x26364c border_radius: 2.0 }
                            }
                            in_d2 := RoundedView {
                                width: 22 height: 4
                                draw_bg +: { color: #x26364c border_radius: 2.0 }
                            }
                            in_gap := View { width: Fill height: Fit }
                            in_skip := OuyuLink { text: "跳过" }
                        }
                        in_mid := ScrollYView {
                            width: Fill height: Fill
                            flow: Down
                            spacing: 14.0
                            // 三枚图标写死在这里、按步显隐，而不是运行时换 svg ——
                            // script_apply_eval! 的作用域里没有 crate_resource
                            // （批次 4 同一个坑）。Icon 自己没有 visible，所以各裹一层 View。
                            in_icons := View {
                                width: Fit height: Fit
                                flow: Right
                                in_ic0 := View {
                                    width: Fit height: Fit
                                    in_i := OuyuIcon {
                                        icon_walk: Walk{ width: 40.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-discover.svg") color: #ffca91 }
                                    }
                                }
                                in_ic1 := View {
                                    visible: false
                                    width: Fit height: Fit
                                    in_i := OuyuIcon {
                                        icon_walk: Walk{ width: 40.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-meet.svg") color: #ffca91 }
                                    }
                                }
                                in_ic2 := View {
                                    visible: false
                                    width: Fit height: Fit
                                    in_i := OuyuIcon {
                                        icon_walk: Walk{ width: 40.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/lock.svg") color: #x9be2bf }
                                    }
                                }
                            }
                            in_title := Label {
                                width: Fill
                                text: ""
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 22.0 line_spacing: 1.35 }
                                }
                            }
                            in_body := Label {
                                width: Fill
                                text: ""
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                }
                            }
                            // 第 3 屏的隐私要点。前两屏收起来。
                            in_points := OuyuCard {
                                visible: false
                                width: Fill height: Fit
                                flow: Down
                                padding: 16.0
                                spacing: 10.0
                                ip0 := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #x9be2bf
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                ip1 := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #x9be2bf
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                ip2 := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #x9be2bf
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                ip3 := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #x9be2bf
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                            }
                        }
                        in_bar := View {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            spacing: 10.0
                            in_back := OuyuBtn { visible: false text: "上一步" }
                            in_gap2 := View { width: Fill height: Fit }
                            in_next := OuyuBtnPrimary { text: "下一步" }
                        }
                    }

                    // ---- 通知条 ----
                    //
                    // 两类通知（去向到期、券到期）都落在这里。不是系统通知：
                    // 这份草图跑在桌面上，没有可用的系统通知通道，所以先在应用内
                    // 把「什么时候该响、响什么」做对，换壳时只要换发送端。
                    notice_layer := View {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        align: Align{x: 0.5, y: 0.0}
                        nt_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            padding: 14.0
                            spacing: 10.0
                            draw_bg +: { color: #x152235 border_color: #x3a4f74 border_size: 1.0 }
                            nt_icon := OuyuIcon {
                                icon_walk: Walk{ width: 18.0 height: Fit }
                                draw_icon +: { svg: crate_resource("self:resources/icons/bell.svg") color: #ffca91 }
                            }
                            nt_col := View {
                                width: Fill height: Fit
                                flow: Down
                                spacing: 3.0
                                nt_title := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 13.5 line_spacing: 1.35 }
                                    }
                                }
                                nt_text := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                            nt_close := OuyuIconBtn {
                                draw_icon +: { svg: crate_resource("self:resources/icons/close.svg") }
                            }
                        }
                    }

                    toast_layer := View {
                        width: Fill height: Fill
                        flow: Down
                        align: Align{x: 0.5, y: 1.0}
                        padding: Inset{left: 2.0, right: 2.0, bottom: 2.0}
                        toast := OuyuToast { }
                    }

                    // 悬浮发布入口：只在发现页出现，压在页面之上、导航之下。
                    fab_layer := View {
                        width: Fill height: Fill
                        flow: Down
                        align: Align{x: 1.0, y: 1.0}
                        padding: Inset{right: 2.0, bottom: 2.0}
                        fab := OuyuFab {
                            draw_icon +: { svg: crate_resource("self:resources/icons/plus.svg") }
                        }
                    }
                }

                // ---- 右列解释栏（窗口窄于 ~900px 时隐藏）----
                aside := ScrollYView {
                    width: 300 height: Fill
                    flow: Down
                    spacing: 14.0

                    // 当页上下文：这一栏原来五页都只有静态说明，
                    // 看久了就变背景板。顶上这张卡跟着当前页和当前数据走，
                    // 但仍然只说「你自己的那一份」——不出现别人的身份、
                    // 人数、距离。
                    aside_ctx := OuyuCard {
                        width: Fill height: Fit
                        flow: Down
                        padding: 16.0
                        spacing: 10.0
                        ax_title := Label {
                            width: Fill
                            text: ""
                            draw_text +: {
                                wrap: Words
                                color: #ffca91
                                text_style +: { font_size: 14.0 line_spacing: 1.35 }
                            }
                        }
                            ax_r0 := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                ax_k := Label {
                                    width: 88
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                                ax_v := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                            ax_r1 := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                ax_k := Label {
                                    width: 88
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                                ax_v := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                            ax_r2 := View {
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 8.0
                                ax_k := Label {
                                    width: 88
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #a4b2c9
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                                ax_v := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: #e7edf8
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                            }
                        ax_tip := Label {
                            width: Fill
                            text: ""
                            draw_text +: {
                                wrap: Words
                                color: #x7b8aa3
                                text_style +: { font_size: 12.5 line_spacing: 1.35 }
                            }
                        }
                    }
                    aside_discover := View {
                        width: Fill height: Fit
                        flow: Down
                        spacing: 14.0
                        ad1 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            ad1_t := Label {
                                text: "偶遇不是找人雷达"
                                draw_text +: {
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            ad1_b := Label {
                                width: Fill
                                text: "看不到谁发布了行程；别人也看不到你的头像、姓名和位置。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                        ad2 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            ad2_t := Label {
                                width: Fill
                                text: "从可能，到真的相遇"
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            ad2_b := Label {
                                width: Fill
                                text: "1 发布粗区域与时段\n2 正常生活，线下认出彼此\n3 双方互认，可选相遇礼"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                        ad3 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            ad3_t := Label {
                                text: "你始终可以退出"
                                draw_text +: {
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            ad3_b := Label {
                                width: Fill
                                text: "发布随时可撤回。宁可少提示，也不披露某个熟人。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                    }
                    aside_meet := View {
                        visible: false
                        width: Fill height: Fit
                        flow: Down
                        spacing: 14.0
                        am1 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            am1_t := Label {
                                width: Fill
                                text: "定位只用在领奖验证"
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            am1_b := Label {
                                width: Fill
                                text: "不持续定位，不向任何人展示坐标。拒绝也能记回忆。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                        am2 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            am2_t := Label {
                                width: Fill
                                text: "每次相遇，重新选择"
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            am2_b := Label {
                                width: Fill
                                text: "保存、隐藏或不保存只影响这一次。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                    }
                    aside_contacts := View {
                        visible: false
                        width: Fill height: Fit
                        flow: Down
                        spacing: 14.0
                        ac1 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            ac1_t := Label {
                                width: Fill
                                text: "记不记，留到每次相遇"
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            ac1_b := Label {
                                width: Fill
                                text: "每次确认时选保存、隐藏或不保存，不设永久策略。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                        ac2 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            ac2_t := Label {
                                width: Fill
                                text: "删除联系人，不必删回忆"
                                draw_text +: {
                                    wrap: Words
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            ac2_b := Label {
                                width: Fill
                                text: "删除时可选是否连回忆一起删，默认保留。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                    }
                    aside_memories := View {
                        visible: false
                        width: Fill height: Fit
                        flow: Down
                        spacing: 14.0
                        ame1 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            ame1_t := Label {
                                text: "默认不记地点"
                                draw_text +: {
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            ame1_b := Label {
                                width: Fill
                                text: "只记和谁、哪一天，不记时刻、位置或路线。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                        ame2 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            ame2_t := Label {
                                text: "删除只影响这一份"
                                draw_text +: {
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            ame2_b := Label {
                                width: Fill
                                text: "删不掉对方自己记的那一份。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                    }
                    aside_achieve := View {
                        visible: false
                        width: Fill height: Fit
                        flow: Down
                        spacing: 14.0
                        aac1 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            aac1_t := Label {
                                text: "成就不是任务"
                                draw_text +: {
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            aac1_b := Label {
                                width: Fill
                                text: "没有签到、排行榜和惩罚。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                        aac2 := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 16.0
                            spacing: 8.0
                            aac2_t := Label {
                                text: "分享时留住边界"
                                draw_text +: {
                                    color: #e7edf8
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            aac2_b := Label {
                                width: Fill
                                text: "分享卡只含汇总次数和文案，不带联系人、地点或日期。"
                                draw_text +: {
                                    wrap: Words
                                    color: #a4b2c9
                                    text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                }
                            }
                        }
                    }
                }
            }
            // ---- 手机模式底部导航（宽屏隐藏，窄屏代替左侧栏）----
            tabbar := RoundedView {
                visible: false
                width: Fill height: 66
                flow: Right
                align: Align{x: 0.5, y: 0.5}
                padding: Inset{left: 6.0, right: 6.0, top: 4.0, bottom: 4.0}
                spacing: 4.0
                draw_bg +: {
                    color: #x0d1626
                    border_color: #x1d2b42
                    border_size: 1.0
                    border_radius: 0.0
                }
                tab_discover := OuyuNavTab { text: "发现" draw_icon +: { svg: crate_resource("self:resources/icons/nav-discover.svg") } }
                tab_meet := OuyuNavTab { text: "相遇" draw_icon +: { svg: crate_resource("self:resources/icons/nav-meet.svg") } }
                tab_contacts := OuyuNavTab { text: "熟人" draw_icon +: { svg: crate_resource("self:resources/icons/nav-contacts.svg") } }
                tab_memories := OuyuNavTab { text: "回忆" draw_icon +: { svg: crate_resource("self:resources/icons/nav-memories.svg") } }
                tab_achieve := OuyuNavTab { text: "我" draw_icon +: { svg: crate_resource("self:resources/icons/nav-me.svg") } }
            }
        }
    }
}

/// 开场三屏。顺序不能改 —— 第一句先把「这不是找人 app」说掉，
/// 后面两句才有意义（design/02-redesign-plan.md 七.1）。
const INTRO: [(&str, &str); 3] = [
    (
        "这不是一个约人的应用",
        "看不到谁在哪、谁在线。地图上没有人，只有一圈匿名的光，它只说明「这一带今天可能有几个熟人路过」。",
    ),
    (
        "你发布的只有一行",
        "「今天下午 · 三里屯一带 · 随意走走」，别人看到的就是这一行。没有名字、头像、时间点和路线，到期自动退出。",
    ),
    (
        "定位只用那一下",
        "只有你们线下认出彼此、一起确认那一刻才定位一次，用来核对是否同地。坐标不存、不传、不写日志。",
    ),
];

/// 第 3 屏底部的隐私要点。这四句是整个应用的边界，改之前先读
/// ouyu/design/02-features.md 的 B / F / G / H 四节。
const INTRO_POINTS: [&str; 4] = [
    "不显示身份、人数、距离和实时位置",
    "回忆只存在这台机器上，随时能导出和清空",
    "通知只有两类，而且默认都是关的",
    "隐藏的回忆不进搜索、不进提醒、不进统计",
];

const INTRO_DOTS: [LiveId; 3] = [live_id!(in_d0), live_id!(in_d1), live_id!(in_d2)];
const INTRO_ICONS: [LiveId; 3] = [live_id!(in_ic0), live_id!(in_ic1), live_id!(in_ic2)];
const INTRO_POINT_ROWS: [LiveId; 4] =
    [live_id!(ip0), live_id!(ip1), live_id!(ip2), live_id!(ip3)];

const TABS: [LiveId; 5] = [
    live_id!(tab_discover),
    live_id!(tab_meet),
    live_id!(tab_contacts),
    live_id!(tab_memories),
    live_id!(tab_achieve),
];
/// 手机形态顶栏标题：与底部导航保持一致。
const TAB_TITLES: [&str; 5] = ["发现", "相遇", "熟人", "回忆", "我"];
const PAGES: [LiveId; 5] = [
    live_id!(page_discover),
    live_id!(page_meet),
    live_id!(page_contacts),
    live_id!(page_memories),
    live_id!(page_achieve),
];
/// 上下文面板那三行。
const AX_ROWS: [LiveId; 3] = [live_id!(ax_r0), live_id!(ax_r1), live_id!(ax_r2)];
const ASIDES: [LiveId; 5] = [
    live_id!(aside_discover),
    live_id!(aside_meet),
    live_id!(aside_contacts),
    live_id!(aside_memories),
    live_id!(aside_achieve),
];
/// 发现页时间分段（今天 / 明天 / 本周）。
const SEG_DAYS: [LiveId; 3] = [live_id!(sg0), live_id!(sg1), live_id!(sg2)];
/// 发现页一周日期条。
const DAY_CELLS: [LiveId; DAY_SPAN] = [
    live_id!(d0),
    live_id!(d1),
    live_id!(d2),
    live_id!(d3),
    live_id!(d4),
    live_id!(d5),
    live_id!(d6),
];
/// 「最可能遇见的地方」的行（达到匿名阈值的片区）。
const RANK_ROWS: [LiveId; 6] = [
    live_id!(rk0),
    live_id!(rk1),
    live_id!(rk2),
    live_id!(rk3),
    live_id!(rk4),
    live_id!(rk5),
];
/// 阈值不足时的普通建议行。
const MORE_ROWS: [LiveId; 3] = [live_id!(rm0), live_id!(rm1), live_id!(rm2)];
/// 发布向导第 1 步：一周七天。
const PUB_DAY_CHIPS: [LiveId; DAY_SPAN] = [
    live_id!(pd0),
    live_id!(pd1),
    live_id!(pd2),
    live_id!(pd3),
    live_id!(pd4),
    live_id!(pd5),
    live_id!(pd6),
];
const PUB_SLOT_CHIPS: [LiveId; 3] = [live_id!(ps0), live_id!(ps1), live_id!(ps2)];
const PUB_INTENT_CHIPS: [LiveId; 3] = [live_id!(pi0), live_id!(pi1), live_id!(pi2)];
/// 区域选择器：类型筛选（0 = 全部，其余对应 AreaKind::ALL）。
const KIND_CHIPS: [LiveId; 9] = [
    live_id!(pk0),
    live_id!(pk1),
    live_id!(pk2),
    live_id!(pk3),
    live_id!(pk4),
    live_id!(pk5),
    live_id!(pk6),
    live_id!(pk7),
    live_id!(pk8),
];
/// 区域选择器：行政区筛选（0 = 全部，其余对应 areas::districts()）。
const DIST_CHIPS: [LiveId; 17] = [
    live_id!(pg0),
    live_id!(pg1),
    live_id!(pg2),
    live_id!(pg3),
    live_id!(pg4),
    live_id!(pg5),
    live_id!(pg6),
    live_id!(pg7),
    live_id!(pg8),
    live_id!(pg9),
    live_id!(pg10),
    live_id!(pg11),
    live_id!(pg12),
    live_id!(pg13),
    live_id!(pg14),
    live_id!(pg15),
    live_id!(pg16),
];
/// 区域选择器：一屏最多铺 18 行，剩下的靠搜索收敛。
const PICK_ROWS: [LiveId; 18] = [
    live_id!(pa0),
    live_id!(pa1),
    live_id!(pa2),
    live_id!(pa3),
    live_id!(pa4),
    live_id!(pa5),
    live_id!(pa6),
    live_id!(pa7),
    live_id!(pa8),
    live_id!(pa9),
    live_id!(pa10),
    live_id!(pa11),
    live_id!(pa12),
    live_id!(pa13),
    live_id!(pa14),
    live_id!(pa15),
    live_id!(pa16),
    live_id!(pa17),
];
/// 区域选择器：最近去过（本机，最多 5 条）。
const RECENT_ROWS: [LiveId; 5] = [
    live_id!(pr0),
    live_id!(pr1),
    live_id!(pr2),
    live_id!(pr3),
    live_id!(pr4),
];
const ECHO_CHIPS: [LiveId; 3] = [live_id!(echo0), live_id!(echo1), live_id!(echo2)];
/// 券包三个分区各留四个位置。演示数据不会更多；真实版本这里要换成列表控件。
const WA_ROWS: [LiveId; 4] = [live_id!(wa0), live_id!(wa1), live_id!(wa2), live_id!(wa3)];
const WU_ROWS: [LiveId; 4] = [live_id!(wu0), live_id!(wu1), live_id!(wu2), live_id!(wu3)];
const WG_ROWS: [LiveId; 4] = [live_id!(wg0), live_id!(wg1), live_id!(wg2), live_id!(wg3)];

const CHOICE_CHIPS: [LiveId; 3] = [live_id!(ch_save), live_id!(ch_hidden), live_id!(ch_skip)];
const MEET_ROWS: [LiveId; 6] = [
    live_id!(mr0),
    live_id!(mr1),
    live_id!(mr2),
    live_id!(mr3),
    live_id!(mr4),
    live_id!(mr5),
];
const SHOP_ROWS: [LiveId; 3] = [live_id!(sv0), live_id!(sv1), live_id!(sv2)];

/// 开发者选项里「替对方按一下」的五个动作。
#[derive(Clone, Copy, PartialEq)]
enum DevAct {
    Confirm,
    Mismatch,
    NotSamePlace,
    NoStock,
    Expire,
}
const CONTACT_ROWS: [LiveId; 6] = [
    live_id!(ct0),
    live_id!(ct1),
    live_id!(ct2),
    live_id!(ct3),
    live_id!(ct4),
    live_id!(ct5),
];
const MEM_ROWS: [LiveId; 8] = [
    live_id!(mem0),
    live_id!(mem1),
    live_id!(mem2),
    live_id!(mem3),
    live_id!(mem4),
    live_id!(mem5),
    live_id!(mem6),
    live_id!(mem7),
];
const HID_ROWS: [LiveId; 8] = [
    live_id!(hid0),
    live_id!(hid1),
    live_id!(hid2),
    live_id!(hid3),
    live_id!(hid4),
    live_id!(hid5),
    live_id!(hid6),
    live_id!(hid7),
];
/// 每个熟人上面那一格字母索引。同一个字母只在第一位显示。
const CONTACT_LETTERS: [LiveId; 6] = [
    live_id!(cl0),
    live_id!(cl1),
    live_id!(cl2),
    live_id!(cl3),
    live_id!(cl4),
    live_id!(cl5),
];

/// 每条回忆上面那个月份头。只有当月第一条才显示 —— 后面的留空隐藏。
const MEM_HEADS: [LiveId; 8] = [
    live_id!(mh0),
    live_id!(mh1),
    live_id!(mh2),
    live_id!(mh3),
    live_id!(mh4),
    live_id!(mh5),
    live_id!(mh6),
    live_id!(mh7),
];
const HID_HEADS: [LiveId; 8] = [
    live_id!(hh0),
    live_id!(hh1),
    live_id!(hh2),
    live_id!(hh3),
    live_id!(hh4),
    live_id!(hh5),
    live_id!(hh6),
    live_id!(hh7),
];

const FILT_CHIPS: [LiveId; 6] = [
    live_id!(filt0),
    live_id!(filt1),
    live_id!(filt2),
    live_id!(filt3),
    live_id!(filt4),
    live_id!(filt5),
];

/// 成就页：4 / 8 周切换 chips。
const WK_CHIPS: [LiveId; 2] = [live_id!(wk4), live_id!(wk8)];
/// 成就页：三个里程碑卡片。
const MS_ROWS: [LiveId; 3] = [live_id!(ms0), live_id!(ms1), live_id!(ms2)];
/// 成就页：「查看每周次数」数据表的 8 行。
const WK_ROWS: [LiveId; 8] = [
    live_id!(wk_r0),
    live_id!(wk_r1),
    live_id!(wk_r2),
    live_id!(wk_r3),
    live_id!(wk_r4),
    live_id!(wk_r5),
    live_id!(wk_r6),
    live_id!(wk_r7),
];
/// 分享页：暖杏 / 夜蓝样式 chips。
const STYLE_CHIPS: [LiveId; 2] = [live_id!(sh_warm), live_id!(sh_night)];

/// 带 `Fill` 子项、手机形态下需要换行的行（换行后第二行放操作按钮）。
const PHONE_WRAP_ROWS: [LiveId; 10] = [
    live_id!(rk_head),
    live_id!(ct_head),
    live_id!(ct_add),
    live_id!(ct_merge_bar),
    live_id!(vcf_row),
    live_id!(dir_head),
    live_id!(curve_head),
    live_id!(ms_head),
    live_id!(sp_head),
    live_id!(ck_head),
];

/// 每页的大标题：手机形态下统一收小一号。
const PAGE_TITLES: [LiveId; 7] = [
    live_id!(hero_title),
    live_id!(pw_title),
    live_id!(mp_title),
    live_id!(ct_title),
    live_id!(mm_title),
    live_id!(ac_title),
    live_id!(sp_title),
];

/// 界面模式（05 节的响应式规则，在宿主里也要能手动切换）：
/// `Auto` 跟随可用宽度；`Phone` 锁定窄屏单列，窗口够宽时收进居中的手机框方便预览；
/// `Wide` 锁定侧栏布局。宿主本身的手机 / 桌面切换在样式菜单里，两者互不冲突。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum UiMode {
    #[default]
    Auto,
    Phone,
    Wide,
}

impl UiMode {
    fn next(self) -> Self {
        match self {
            Self::Auto => Self::Phone,
            Self::Phone => Self::Wide,
            Self::Wide => Self::Auto,
        }
    }
    /// 侧栏按钮上的完整文案。
    fn long_label(self) -> &'static str {
        match self {
            Self::Auto => "自动",
            Self::Phone => "手机模式",
            Self::Wide => "大屏模式",
        }
    }
    /// 手机顶栏上的短文案。
    fn short_label(self) -> &'static str {
        match self {
            Self::Auto => "自动",
            Self::Phone => "手机",
            Self::Wide => "大屏",
        }
    }
}

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
    /// 宽 surface 上的手机预览框（居中一条手机宽度的列）。
    framed: bool,
    /// 右列解释栏是否还有位置。
    aside: bool,
    /// 横屏手机这类矮 surface：压缩街区插图与曲线高度。
    short: bool,
    /// 手机框的尺寸。
    frame: (f64, f64),
    /// 手机框左右留白（居中用内边距实现，避免 align 影响底色绘制）。
    pad: (f64, f64),
}

/// 开场三屏的正文列宽。
const INTRO_COL: f64 = 620.0;
/// 手机形态的上限宽度：低于此值走底部导航单列。
const PHONE_MAX: f64 = 720.0;
/// 桌面形态（侧栏 + 右列）的下限宽度。
const DESKTOP_MIN: f64 = 1060.0;
/// 右列解释栏至少需要的宽度。
const ASIDE_MIN: f64 = 900.0;
/// 手机预览框的宽 / 最大高（对齐宿主 mobile.rs 的 412x892 手机 surface）。
const FRAME_W: f64 = 412.0;
const FRAME_H: f64 = 880.0;

/// 可用 surface 尺寸 + 界面模式 → 布局形态。宿主把应用放进多大的 tile，
/// 这里就按多大排版：窗口尺寸不代表 tile 尺寸，所以只看自己拿到的 turtle。
fn shaping_for(mode: UiMode, size: Vec2d) -> Shaping {
    let (w, h) = (size.x, size.y);
    let auto = if w < PHONE_MAX {
        Shape::Phone
    } else if w < DESKTOP_MIN {
        Shape::Tablet
    } else {
        Shape::Desktop
    };
    let (shape, framed) = match mode {
        UiMode::Auto => (auto, false),
        UiMode::Phone => (Shape::Phone, w >= FRAME_W + 140.0),
        // 「大屏」最宽也只能给到当前宽度撑得住的那一档，免得锁死成不可用的布局。
        UiMode::Wide => (if auto == Shape::Phone { Shape::Tablet } else { auto }, false),
    };
    let inner_w = if framed { FRAME_W } else { w };
    let inner_h = if framed { (h - 24.0).clamp(360.0, FRAME_H) } else { h };
    Shaping {
        shape,
        framed,
        aside: shape == Shape::Desktop && inner_w >= ASIDE_MIN,
        short: inner_h < 560.0,
        frame: (FRAME_W, inner_h),
        pad: if framed {
            (((w - FRAME_W) * 0.5).max(0.0), ((h - inner_h) * 0.5).max(0.0))
        } else {
            (0.0, 0.0)
        },
    }
}

/// 「我」页进去的两张覆盖页。它们盖住 Tab 页，但不是第六个 Tab ——
/// 券包和设置是从「我」里进去的，退出来还得回到「我」。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sheet {
    Wallet,
    Settings,
}

/// 列表页的四个态（docs/02 七.5）。「空」不在这里 —— 空是数据说了算的，
/// 另外三个是加载和网络说了算的，混在一个枚举里会让「空」被误当成一种故障。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ListState {
    #[default]
    Ready,
    Loading,
    Failed,
    Offline,
}

#[derive(Script, ScriptHook, Widget)]
pub struct OuyuView {
    #[deref]
    view: View,
    /// Tab 切换淡入用的底色覆盖层（alpha 随时间衰减）。
    #[redraw]
    #[live]
    draw_fade: DrawColor,
    #[rust]
    state: OuyuState,
    #[rust]
    initialized: bool,
    /// Tab 淡入动画: 起始时间与当前不透明度。
    #[rust]
    fade_start: Option<f64>,
    #[rust]
    fade_alpha: f32,
    #[rust]
    next_frame: NextFrame,

    // ---- 发现页 ----
    /// 草图场景：稀疏 = 匿名条件未满足，只给城市小签（仅演示条件）。
    #[rust]
    sparse: bool,
    /// 发现页选中的日期（相对今天 0..=6）。
    #[rust]
    day_sel: usize,
    /// 时间分段：0 今天 / 1 明天 / 2 本周。
    #[rust]
    day_seg: usize,
    /// 发布向导当前步骤（0 时间 / 1 片区 / 2 意愿）；None = 没在发布。
    #[rust]
    wizard: Option<usize>,
    /// 发布草稿（day 偏移, slot, 片区 id, intent）；退出向导即丢弃。
    #[rust]
    draft: (usize, usize, u16, usize),
    /// 每个片区行当前铺的是哪个片区 —— 点一行时要知道点中了谁。
    #[rust]
    row_areas: Vec<(LiveId, u16)>,
    /// 区域选择器：搜索词、类型筛选、行政区筛选、筛选面板是否展开。
    #[rust]
    pick_query: String,
    #[rust]
    pick_kind: usize,
    #[rust]
    pick_dist: usize,
    #[rust]
    pick_filters: bool,

    // ---- 相遇页 ----
    /// 选中的联系人（contacts 下标）。
    #[rust]
    meet_sel: usize,
    /// 本次回忆选择（默认保存）。
    #[rust]
    choice: MemoryChoice,
    /// 进行中的现场会话（点「确认相遇」后）。
    #[rust]
    session: Option<RecogSession>,
    /// 等待态的每秒心跳。只在 Waiting 期间跑着。
    #[rust]
    tick: Timer,
    /// 等待态里「对方没有偶遇？」是否展开。
    #[rust]
    code_open: bool,

    // ---- 熟人页 ----
    #[rust]
    confirm_row: Option<usize>,
    #[rust]
    confirm_also: bool,

    // ---- 回忆页 ----
    #[rust]
    memory_filter: Option<String>,
    #[rust]
    filter_options: Vec<String>,
    #[rust]
    mem_row_ids: Vec<usize>,
    #[rust]
    hid_row_ids: Vec<usize>,

    // ---- 成就页 / 分享卡 ----
    /// 曲线窗口：4 / 8 周（默认 8）。
    #[rust]
    weeks: usize,
    /// 「查看每周次数」数据表展开态。
    #[rust]
    weekly_open: bool,
    /// 分享卡预览页打开（覆盖在成就页之上，不是侧栏 Tab）。
    #[rust]
    share_open: bool,
    #[rust]
    share_style: ShareStyle,
    /// 「包含每周曲线」开关（默认关）。
    #[rust]
    share_curve: bool,

    // ---- 「我」页的覆盖页 ----
    /// 打开的覆盖页（我的券 / 设置），None 表示在 Tab 页上。
    #[rust]
    sheet: Option<Sheet>,
    /// 「清除本机数据」的二次确认展开态。这一步不可撤销，所以不走 toast。
    #[rust]
    clear_armed: bool,
    /// 导出后显示的那行路径。
    #[rust]
    export_note: Option<String>,

    // ---- 删除撤销 ----
    /// 5 秒内还能放回去的那份快照。
    #[rust]
    undo: Option<UndoSnapshot>,
    /// 撤销窗口的计时器。到点就把快照丢掉、把 toast 收起来。
    #[rust]
    undo_timer: Timer,
    /// 通知的巡检计时器。一分钟看一眼够了 —— 两类通知的粒度都是分钟以上。
    #[rust]
    notice_poll: Timer,

    /// 列表四态。只能从开发者选项改 —— 这份草图没有真的加载和真的断网。
    #[rust]
    list_state: ListState,

    // ---- 熟人页 ----
    /// 正在往别人身上并的那一位（contact id）。None 表示不在合并模式。
    #[rust]
    merge_from: Option<usize>,
    /// 手动添加失败时那一句红字。
    #[rust]
    add_error: Option<AddContactError>,
    /// 熟人按字母排过之后的顺序（contact id）。
    ///
    /// 行的下标从此指这张表，不再直接指 `state.contacts`——
    /// 否则排序一变，点「删除」删掉的就是另一个人。
    #[rust]
    contact_order: Vec<usize>,

    // ---- 回忆页 ----
    /// 搜索框里的字。空串表示没在搜。
    #[rust]
    mem_query: String,
    /// 打开的那一条回忆详情（encounter id）。
    #[rust]
    mem_detail: Option<usize>,

    /// 上一次量到的窗口尺寸。`Shaping` 里只有档位没有宽度，
    /// 而开场三屏要按真实宽度算左右留白。
    #[rust]
    last_size: Vec2d,

    // ---- 开场三屏 ----
    /// 正停在第几屏；None 表示不在引导里。
    #[rust]
    intro: Option<usize>,

    // ---- 通知 ----
    /// 正显示的那一条通知。同时只显示一条 —— 两条一起弹就成了信息流。
    #[rust]
    notice: Option<Notice>,
    /// 这次运行里已经发过的通知种类，用来防重复。
    ///
    /// 只活在内存里：重启之后重新算一次没有坏处（该提醒的还是该提醒），
    /// 而为了防重就把「提醒过没有」写进 state.json，反倒是多存了一份东西。
    #[rust]
    notice_sent: Vec<NoticeKind>,

    /// 界面模式：自动 / 手机 / 大屏，由侧栏与手机顶栏的按钮循环切换。
    #[rust]
    ui_mode: UiMode,
    /// 上次已套用的布局形态；相同就跳过，避免每帧重排。
    #[rust]
    shaping: Option<Shaping>,
}

impl OuyuView {
    fn set_tab(&mut self, cx: &mut Cx, i: usize) {
        // 离开相遇页时：已经出结果的按本次选择写入回忆（只写一次）。
        // 进行中的会话留着 —— 10 分钟的窗口不该因为去别的页看一眼就作废。
        if self.state.tab == 1 && i != 1 {
            let done = self.session.as_ref().map(|s| s.stage.is_result()) == Some(true);
            if done {
                if let Some(mut s) = self.session.take() {
                    self.state.write_session_memory(&mut s);
                }
                self.refresh_contacts(cx);
                self.refresh_memories(cx);
            }
            self.code_open = false;
        }
        // 切 Tab 即离开分享卡预览和覆盖页（它们都不是 Tab）。
        // 发布向导也一样离开，草稿丢掉 —— 否则底部导航高亮在「我」，
        // 屏幕上却还摆着发布向导的第 1 步。
        self.share_open = false;
        self.sheet = None;
        self.clear_armed = false;
        self.wizard = None;
        self.mem_detail = None;
        self.merge_from = None;
        self.add_error = None;
        let page_changed = self.state.tab != i;
        self.state.tab = i;
        for (j, id) in TABS.iter().enumerate() {
            self.view
                .check_box(cx, &[live_id!(sidebar), *id])
                .set_active(cx, j == i, Animate::Yes);
            // 手机形态的底部导航是同一组 Tab 的第二份控件。
            self.view
                .check_box(cx, &[live_id!(tabbar), *id])
                .set_active(cx, j == i, Animate::Yes);
            // DrawSvg 没有 active 通道（CheckBox 的 animator 只驱动 draw_bg /
            // draw_text），所以图标的选中色在这里逐个 apply。
            for root in [live_id!(sidebar), live_id!(tabbar)] {
                let mut tab = self.view.widget(cx, &[root, *id]);
                if j == i {
                    script_apply_eval!(cx, tab, { draw_icon +: { color: #ffca91 } });
                } else {
                    script_apply_eval!(cx, tab, { draw_icon +: { color: #a4b2c9 } });
                }
            }
        }
        self.view
            .label(cx, ids!(shell.topbar.tb_title))
            .set_text(cx, TAB_TITLES[i]);
        self.update_page_visibility(cx);
        for (j, id) in ASIDES.iter().enumerate() {
            self.view.widget(cx, &[*id]).set_visible(cx, j == i);
        }
        // 页面切换（或首次进入）时做一次 150ms 淡入。
        if page_changed || !self.initialized {
            self.fade_start = None;
            self.fade_alpha = 1.0;
            self.next_frame = cx.new_next_frame();
            self.redraw(cx);
        }
        match i {
            0 => self.refresh_discover(cx),
            1 => self.refresh_meet(cx),
            2 => self.refresh_contacts(cx),
            3 => self.refresh_memories(cx),
            4 => self.refresh_achievements(cx),
            _ => {}
        }
    }

    /// 分享卡预览 / 发布向导 / 券包 / 设置 / 开场三屏打开时盖住所有 Tab 页；
    /// 否则只显示当前 Tab 页。
    fn update_page_visibility(&mut self, cx: &mut Cx) {
        let wizard = self.wizard.is_some();
        let intro = self.intro.is_some();
        let detail = self.mem_detail.is_some();
        let overlay = self.share_open || wizard || self.sheet.is_some() || intro || detail;
        self.view.widget(cx, ids!(page_intro)).set_visible(cx, intro);
        self.view
            .widget(cx, ids!(page_memdetail))
            .set_visible(cx, detail && !intro);
        for (j, id) in PAGES.iter().enumerate() {
            self.view
                .widget(cx, &[*id])
                .set_visible(cx, !overlay && j == self.state.tab);
        }
        self.view
            .widget(cx, ids!(page_share))
            .set_visible(cx, self.share_open);
        self.view
            .widget(cx, ids!(page_wallet))
            .set_visible(cx, self.sheet == Some(Sheet::Wallet));
        self.view
            .widget(cx, ids!(page_settings))
            .set_visible(cx, self.sheet == Some(Sheet::Settings));
        self.view
            .widget(cx, ids!(page_publish))
            .set_visible(cx, wizard && !self.share_open && self.sheet.is_none() && !intro);
        // 悬浮发布按钮只属于发现页。
        self.view
            .widget(cx, ids!(fab_layer))
            .set_visible(cx, !overlay && self.state.tab == 0);
    }

    /// 让一组芯片互斥选中（CheckBox 本身是可再点关的, 这里强制单选）。
    fn set_chip_group(&mut self, cx: &mut Cx, parent: &[LiveId], chips: &[LiveId], active: usize) {
        for (j, id) in chips.iter().enumerate() {
            let mut path = parent.to_vec();
            path.push(*id);
            self.view
                .check_box(cx, &path)
                .set_active(cx, j == active, Animate::Yes);
        }
    }

    fn refresh_all(&mut self, cx: &mut Cx) {
        self.set_chip_group(
            cx,
            &[live_id!(page_meet), live_id!(meet_result), live_id!(choice_card), live_id!(choice_row)],
            &CHOICE_CHIPS,
            choice_index(self.choice),
        );
        self.set_tab(cx, self.state.tab);
        self.refresh_discover(cx);
        self.refresh_publish(cx);
        self.refresh_meet(cx);
        self.refresh_contacts(cx);
        self.refresh_memories(cx);
        self.refresh_wallet(cx);
        self.refresh_settings(cx);
        self.refresh_mode_buttons(cx);
    }

    // ---- 开场三屏 ----

    /// 进引导。`step` 是从第几屏开始（重看时也从 0 开始）。
    fn open_intro(&mut self, cx: &mut Cx, step: usize) {
        self.intro = Some(step.min(INTRO.len() - 1));
        self.sheet = None;
        self.share_open = false;
        self.wizard = None;
        self.refresh_intro(cx);
        self.update_page_visibility(cx);
        self.reshape(cx);
    }

    /// 出引导。看完和跳过都走这里 —— 跳过的人也算「知道入口在哪」了，
    /// 所以一样记上 `onboarded`，重看的入口在「我 → 关于偶遇」里留着。
    fn close_intro(&mut self, cx: &mut Cx) {
        self.intro = None;
        self.state.settings.onboarded = true;
        self.state.save();
        self.update_page_visibility(cx);
        self.refresh_settings(cx);
        self.reshape(cx);
    }

    fn refresh_intro(&mut self, cx: &mut Cx) {
        let Some(step) = self.intro else { return };
        let (title, body) = INTRO[step];
        for (i, id) in INTRO_ICONS.iter().enumerate() {
            self.view
                .widget(cx, &[live_id!(page_intro), live_id!(in_mid), live_id!(in_icons), *id])
                .set_visible(cx, i == step);
        }
        self.view
            .widget(cx, ids!(page_intro.in_mid.in_title))
            .set_text(cx, title);
        self.view
            .widget(cx, ids!(page_intro.in_mid.in_body))
            .set_text(cx, body);
        // 进度点：走到哪一颗亮哪一颗。
        for (i, id) in INTRO_DOTS.iter().enumerate() {
            let mut dot = self.view.widget(cx, &[live_id!(page_intro), live_id!(in_top), *id]);
            if i == step {
                script_apply_eval!(cx, dot, { draw_bg +: { color: #82b5ff } });
            } else {
                script_apply_eval!(cx, dot, { draw_bg +: { color: #x26364c } });
            }
        }
        let last = step + 1 == INTRO.len();
        self.view
            .widget(cx, ids!(page_intro.in_mid.in_points))
            .set_visible(cx, last);
        for (i, id) in INTRO_POINT_ROWS.iter().enumerate() {
            self.view
                .widget(cx, &[live_id!(page_intro), live_id!(in_mid), live_id!(in_points), *id])
                .set_text(cx, &format!("· {}", INTRO_POINTS[i]));
        }
        self.view
            .widget(cx, ids!(page_intro.in_bar.in_back))
            .set_visible(cx, step > 0);
        self.view
            .widget(cx, ids!(page_intro.in_top.in_skip))
            .set_visible(cx, !last);
        self.view
            .widget(cx, ids!(page_intro.in_bar.in_next))
            .set_text(cx, if last { "知道了，开始" } else { "下一步" });
    }

    // ---- 通知 ----

    /// 看看此刻有没有该发的通知，有就弹一条。
    ///
    /// 判断全在 `due_notices` 里（纯函数、有单测）；这里只管发过的不再发。
    fn poll_notices(&mut self, cx: &mut Cx) {
        if self.notice.is_some() || self.intro.is_some() {
            return;
        }
        let now_min = minutes_of_day();
        let due = due_notices(&self.state, today_days(), now_min);
        let Some(n) = due.into_iter().find(|n| !self.notice_sent.contains(&n.kind)) else {
            return;
        };
        self.notice_sent.push(n.kind);
        self.show_notice(cx, n);
    }

    /// 把时钟挪到该响的那一刻，问一次真实逻辑「现在发什么」。
    ///
    /// 开关在这里临时打开又放回去 —— 演示不该顺手把用户的通知开关改掉。
    fn demo_notice(&mut self, kind: NoticeKind) -> Option<Notice> {
        let today = today_days();
        let saved = (self.state.settings.notify_publish, self.state.settings.notify_reward);
        self.state.settings.notify_publish = true;
        self.state.settings.notify_reward = true;
        let out = match kind {
            NoticeKind::PublishExpiring => {
                // 挪到这条去向所在时段结束前 15 分钟。
                let slot = self.state.publish.as_ref().map(|p| p.slot.min(2)).unwrap_or(1);
                let end = [12 * 60u32, 18 * 60, 22 * 60][slot];
                due_notices(&self.state, today, end - 15)
            }
            NoticeKind::RewardExpiring => {
                // 挪到券只剩一天的那天。
                let exp = self
                    .state
                    .wallet
                    .iter()
                    .filter(|r| r.state(today) == RewardState::Available)
                    .filter_map(|r| r.expires_on)
                    .min();
                match exp {
                    Some(e) => due_notices(&self.state, e - 1, 9 * 60),
                    None => Vec::new(),
                }
            }
        };
        self.state.settings.notify_publish = saved.0;
        self.state.settings.notify_reward = saved.1;
        out.into_iter().find(|n| n.kind == kind)
    }

    /// 一条没有撤销按钮的提示（撤销那条走 `offer_undo`）。
    fn toast(&mut self, cx: &mut Cx, text: &str) {
        self.view
            .widget(cx, ids!(toast_layer.toast.to_text))
            .set_text(cx, text);
        self.view
            .widget(cx, ids!(toast_layer.toast.to_undo))
            .set_visible(cx, false);
        self.view.widget(cx, ids!(toast_layer.toast)).set_visible(cx, true);
        self.undo = None;
        if !self.undo_timer.is_empty() {
            cx.stop_timer(self.undo_timer);
        }
        self.undo_timer = cx.start_timeout(4.0);
        self.redraw(cx);
    }

    fn show_notice(&mut self, cx: &mut Cx, n: Notice) {
        self.view
            .widget(cx, ids!(notice_layer.nt_card.nt_col.nt_title))
            .set_text(cx, n.kind.title());
        self.view
            .widget(cx, ids!(notice_layer.nt_card.nt_col.nt_text))
            .set_text(cx, &n.text);
        self.view.widget(cx, ids!(notice_layer)).set_visible(cx, true);
        self.notice = Some(n);
        self.redraw(cx);
    }

    fn close_notice(&mut self, cx: &mut Cx) {
        self.notice = None;
        self.view.widget(cx, ids!(notice_layer)).set_visible(cx, false);
        self.redraw(cx);
    }

    /// 侧栏 / 手机顶栏上的模式按钮文案。
    fn refresh_mode_buttons(&mut self, cx: &mut Cx) {
        let mode = self.ui_mode;
        self.view
            .button(cx, ids!(sidebar.sb_mode))
            .set_text(cx, mode.long_label());
        self.view
            .button(cx, ids!(shell.topbar.tb_mode))
            .set_text(cx, mode.short_label());
    }

    /// 按当前 surface 尺寸重排。托管在 tile 里时窗口尺寸没有意义，
    /// 所以只看 draw_walk 拿到的 turtle 矩形（见 Widget::draw_walk）。
    fn update_responsive(&mut self, cx: &mut Cx, size: Vec2d) {
        if size.x < 2.0 || size.y < 2.0 {
            return;
        }
        self.last_size = size;
        let want = shaping_for(self.ui_mode, size);
        // 开场三屏的留白按像素宽度走，档位没变也得重算一遍。
        if self.shaping == Some(want) && self.intro.is_none() {
            return;
        }
        self.shaping = Some(want);
        self.apply_shaping(cx, want);
    }

    /// 按当前形态重跑一次 `apply_shaping`。
    ///
    /// 进出开场三屏时用：形态没变（还是那个宽度），但「要不要给导航」变了，
    /// 而 `update_responsive` 看形态没变就直接返回了。
    fn reshape(&mut self, cx: &mut Cx) {
        if let Some(s) = self.shaping {
            self.apply_shaping(cx, s);
        }
    }

    /// 让一组行在手机形态下换行：带 `Fill` 子项的行会把后面的按钮挤到第二行。
    fn set_row_wrap(&mut self, cx: &mut Cx, path: &[LiveId], wrap: bool) {
        if let Some(mut view) = self.view.view(cx, path).borrow_mut() {
            view.layout.flow = Flow::Right { row_align: RowAlign::Top, wrap };
            view.layout.wrap_spacing = 8.0;
        }
    }

    /// 套用一次布局形态：导航位置、右列、内边距、换行、插图与标题尺寸。
    fn apply_shaping(&mut self, cx: &mut Cx, s: Shaping) {
        let phone = s.shape == Shape::Phone;
        // 开场三屏里连导航都不给：这三句话不该能被一脚跨过去。
        // 判断放在这里而不是 update_page_visibility —— 那边藏完，
        // 紧跟着的这一次重排又会把它们放回来。
        let intro = self.intro.is_some();

        // 手机：左侧栏收起，导航去底部，标题进顶栏。
        self.view.widget(cx, ids!(sidebar)).set_visible(cx, !phone && !intro);
        // 矮 tile 里侧栏底部塞不下说明文字，只留模式按钮。
        self.view
            .widget(cx, ids!(sidebar.sb_mode_label))
            .set_visible(cx, !s.short);
        self.view
            .widget(cx, ids!(sidebar.sb_note))
            .set_visible(cx, !s.short);
        self.view.widget(cx, ids!(shell.topbar)).set_visible(cx, phone && !intro);
        self.view.widget(cx, ids!(shell.tabbar)).set_visible(cx, phone && !intro);
        self.view.widget(cx, ids!(main.aside)).set_visible(cx, s.aside && !intro);

        // 开场三屏：宽屏上把整页收进一条 620px 的列。三句话是要被读完的，
        // 一行铺满 1280px 读起来会跳行。
        if let Some(mut v) = self.view.view(cx, ids!(page_intro)).borrow_mut() {
            let side = ((self.last_size.x - INTRO_COL) * 0.5).max(0.0);
            v.layout.padding = Inset { left: side, right: side, top: 0.0, bottom: 0.0 };
        }

        // 手机模式套在宽窗口上时，把整个应用收进居中的手机框做预览。
        // 用 padding 而不是 align 居中：align 会让根视图的底色跟着子项收缩。
        self.view.layout.padding = Inset {
            left: s.pad.0,
            right: s.pad.0,
            top: s.pad.1,
            bottom: s.pad.1,
        };
        let (fw, fh) = s.frame;
        if let Some(mut shell) = self.view.view(cx, ids!(shell)).borrow_mut() {
            shell.walk.width = if s.framed { Size::Fixed(fw) } else { Size::fill() };
            shell.walk.height = if s.framed { Size::Fixed(fh) } else { Size::fill() };
        }
        let (border, radius) = if s.framed { (1.0, 26.0) } else { (0.0, 0.0) };
        let mut shell = self.view.widget(cx, ids!(shell));
        script_apply_eval!(cx, shell, {
            draw_bg +: {
                color: #x0b1220
                border_size: #(border)
                border_radius: #(radius)
                border_color: #x3a4f74
            }
        });

        if let Some(mut main) = self.view.view(cx, ids!(main)).borrow_mut() {
            main.layout.padding = if phone {
                Inset { left: 16.0, right: 16.0, top: 18.0, bottom: 10.0 }
            } else {
                Inset { left: 20.0, right: 20.0, top: 20.0, bottom: 16.0 }
            };
            main.layout.spacing = if s.aside { 16.0 } else { 0.0 };
        }

        // 带 Fill 子项的行：手机上换行，宽屏保持单行。
        for id in PHONE_WRAP_ROWS {
            self.set_row_wrap(cx, &[id], phone);
        }
        for row in CONTACT_ROWS {
            self.set_row_wrap(cx, &[row, live_id!(c_main)], phone);
            self.set_row_wrap(cx, &[row, live_id!(c_confirm)], phone);
        }
        for row in MEM_ROWS.into_iter().chain(HID_ROWS) {
            self.set_row_wrap(cx, &[row], phone);
        }

        // 三张里程碑卡 / 分享页左右两栏：手机上改成竖排。
        for (path, spacing) in [(ids!(ms_row), 12.0), (ids!(sp_row), 14.0)] {
            if let Some(mut view) = self.view.view(cx, path).borrow_mut() {
                view.layout.flow = if phone {
                    Flow::Down
                } else {
                    Flow::Right { row_align: RowAlign::Top, wrap: false }
                };
                view.layout.spacing = spacing;
            }
        }
        if let Some(mut right) = self.view.view(cx, ids!(sp_row.sp_right)).borrow_mut() {
            right.walk.width = if phone { Size::fill() } else { Size::Fixed(300.0) };
        }

        // 成就曲线：手机降高，横屏手机这类矮 surface 再降一档。
        let chart_h = if s.short { 130.0 } else if phone { 160.0 } else { 200.0 };
        let mut chart = self.view.widget(cx, ids!(curve_chart));
        script_apply_eval!(cx, chart, { height: #(chart_h) });

        // 页面大标题：手机上收一号，免得两行标题占满第一屏。
        let title = if phone { 22.0 } else { 24.0 };
        for id in PAGE_TITLES {
            let mut label = self.view.widget(cx, &[id]);
            script_apply_eval!(cx, label, { draw_text.text_style.font_size: #(title) });
        }

        self.redraw(cx);
    }

    /// AI 工具应答（02 H 节）：只取匿名快照——结构上没有姓名 / 人数 / 联系方式，
    /// 隐藏回忆不进 AI；发布、互认、领奖仍由本人在界面操作。
    pub fn ai_answer(&self, call: &ServiceCall) -> ToolResult {
        let snap = ai::OpportunitySnapshot::from_state(&self.state, !self.sparse);
        ai::answer(&snap, call)
    }

    // ---- 发现页 ----

    /// 一行片区：名字 + 「行政区 · 类型」 + 机会分档。分档是这一行唯一
    /// 对外的强度信息，永远没有人数、身份或距离（02 B 节）。
    fn fill_area_row(
        &mut self,
        cx: &mut Cx,
        row: LiveId,
        area_id: u16,
        level: OppLevel,
        best_slot: Option<usize>,
    ) {
        let Some(a) = areas::area(area_id) else {
            self.view.widget(cx, &[row]).set_visible(cx, false);
            return;
        };
        self.view.widget(cx, &[row]).set_visible(cx, true);
        self.row_areas.retain(|(r, _)| *r != row);
        self.row_areas.push((row, area_id));
        self.view.label(cx, &[row, live_id!(ar_name)]).set_text(cx, a.name);
        // 时段只给到上午 / 下午 / 晚间这一粒度，不给具体钟点（文档 02 A-4）。
        let sub = match best_slot.filter(|_| level.shown()) {
            Some(s) => format!("{} · {} · {}最集中", a.district, a.kind.label(), SLOTS[s]),
            None => format!("{} · {}", a.district, a.kind.label()),
        };
        self.view.label(cx, &[row, live_id!(ar_sub)]).set_text(cx, &sub);
        let lv = self.view.label(cx, &[row, live_id!(ar_level)]);
        lv.set_text(cx, if level.shown() { level.label() } else { "" });
        // DrawSvg / DrawText 的颜色没有状态通道，逐行 apply。
        let mut lvw = self.view.widget(cx, &[row, live_id!(ar_level)]);
        let mut dot = self.view.widget(cx, &[row, live_id!(ar_dot)]);
        match level {
            OppLevel::Likely => {
                script_apply_eval!(cx, lvw, { draw_text +: { color: #ffca91 } });
                script_apply_eval!(cx, dot, { draw_icon +: { color: #ffca91 } });
            }
            OppLevel::Possible => {
                script_apply_eval!(cx, lvw, { draw_text +: { color: #82b5ff } });
                script_apply_eval!(cx, dot, { draw_icon +: { color: #82b5ff } });
            }
            OppLevel::Few => {
                script_apply_eval!(cx, lvw, { draw_text +: { color: #a4b2c9 } });
                script_apply_eval!(cx, dot, { draw_icon +: { color: #a4b2c9 } });
            }
            OppLevel::BelowThreshold => {
                script_apply_eval!(cx, dot, { draw_icon +: { color: #x3c4c6b } });
            }
        }
    }

    /// 某一行当前铺的片区。行被隐藏时点不到，所以不必清理旧记录。
    fn area_of_row(&self, row: LiveId) -> Option<u16> {
        self.row_areas
            .iter()
            .find(|(r, _)| *r == row)
            .map(|(_, a)| *a)
    }

    fn refresh_discover(&mut self, cx: &mut Cx) {
        let today = today_days();

        // ---- 时间选择 ----
        for (j, id) in SEG_DAYS.iter().enumerate() {
            self.view
                .check_box(
                    cx,
                    &[live_id!(page_discover), live_id!(time_card), live_id!(seg_track), *id],
                )
                .set_active(cx, j == self.day_seg, Animate::Yes);
        }
        // 日期条的强度点：当天达到阈值的片区越多越亮。只有亮度，没有数字。
        let intensity = week_intensity_at(today);
        for (j, id) in DAY_CELLS.iter().enumerate() {
            let path = [
                live_id!(page_discover),
                live_id!(time_card),
                live_id!(day_strip),
                *id,
            ];
            let cb = self.view.check_box(cx, &path);
            cb.set_text(day_label_at(today, j));
            cb.set_active(cx, j == self.day_sel, Animate::Yes);
            let mut w = self.view.widget(cx, &path);
            match intensity[j] {
                0 => script_apply_eval!(cx, w, { draw_icon +: { color: #x2b3a52 } }),
                1..=14 => script_apply_eval!(cx, w, { draw_icon +: { color: #x5b6f92 } }),
                15..=24 => script_apply_eval!(cx, w, { draw_icon +: { color: #82b5ff } }),
                _ => script_apply_eval!(cx, w, { draw_icon +: { color: #ffca91 } }),
            }
        }

        // ---- 排行 ----
        //
        // sparse 是开发者选项里的草图场景：强制走「阈值不足」那条分支，
        // 方便检查空态，不代表附近真实人数。
        let ranking = opportunity_ranking_at(today, self.day_sel);
        let top: Vec<AreaOpportunity> = if self.sparse {
            Vec::new()
        } else {
            ranking
                .iter()
                .copied()
                .filter(|o| o.level.shown())
                .take(RANK_ROWS.len())
                .collect()
        };
        for (j, id) in RANK_ROWS.iter().enumerate() {
            match top.get(j) {
                Some(o) => self.fill_area_row(cx, *id, o.area, o.level, o.best_slot),
                None => self.view.widget(cx, &[*id]).set_visible(cx, false),
            }
        }
        let has_top = !top.is_empty();
        self.view
            .widget(cx, ids!(page_discover.rank_card.rk_list))
            .set_visible(cx, has_top);
        self.view
            .widget(cx, ids!(page_discover.rank_card.rk_empty))
            .set_visible(cx, !has_top);
        self.view
            .widget(cx, ids!(page_discover.rank_card.sign_card))
            .set_visible(cx, !has_top);
        if !has_top {
            let sign = SIGNS[(today as usize + self.day_sel) % SIGNS.len()];
            self.view
                .label(cx, ids!(page_discover.rank_card.sign_card.sign_text))
                .set_text(cx, &format!("今天的小签：{sign}"));
        }
        self.view
            .label(cx, ids!(page_discover.rank_card.rk_note))
            .set_text(
                cx,
                if has_top {
                    "只分档，不给人数、身份或距离。"
                } else {
                    "还不满足匿名保护条件，暂不显示熟人机会。"
                },
            );
        // 阈值不足的片区里挑三个当作普通建议：这是城市建议，不是熟人机会，
        // 所以行内不显示任何分档字样。
        let more: Vec<AreaOpportunity> = ranking
            .iter()
            .copied()
            .filter(|o| !o.level.shown())
            .take(MORE_ROWS.len())
            .collect();
        for (j, id) in MORE_ROWS.iter().enumerate() {
            match more.get(j) {
                Some(o) => self.fill_area_row(cx, *id, o.area, OppLevel::BelowThreshold, None),
                None => self.view.widget(cx, &[*id]).set_visible(cx, false),
            }
        }

        // ---- 我的去向 ----
        let published = self.state.publish.is_some();
        let (title, sub) = match &self.state.publish {
            Some(p) => (p.text(), "别人只会看到这一行。".to_string()),
            None => (
                "还没写你的去向".to_string(),
                "写了才会进入别人的匿名机会。".to_string(),
            ),
        };
        self.view
            .label(cx, ids!(page_discover.mine_card.mn_col.mn_title))
            .set_text(cx, &title);
        self.view
            .label(cx, ids!(page_discover.mine_card.mn_col.mn_sub))
            .set_text(cx, &sub);
        self.view
            .widget(cx, ids!(page_discover.mine_card.mn_go))
            .set_visible(cx, !published);
        self.view
            .widget(cx, ids!(page_discover.mine_card.mn_edit))
            .set_visible(cx, published);
        self.view
            .widget(cx, ids!(page_discover.mine_card.mn_withdraw))
            .set_visible(cx, published);

        // ---- 开发者：草图场景开关（已收进「我」页的开发者选项）----
        self.view.button(cx, ids!(page_settings.dev_card.dv_sparse)).set_text(
            cx,
            if self.sparse {
                "稀疏场景（点击恢复）"
            } else {
                "切换稀疏场景"
            },
        );

        // ---- 回声 ----
        let echo = self.state.echo;
        for (i, id) in ECHO_CHIPS.iter().enumerate() {
            let path = [
                live_id!(page_discover),
                live_id!(echo_card),
                live_id!(echo_row),
                *id,
            ];
            self.view
                .check_box(cx, &path)
                .set_active(cx, echo == Some(i), Animate::Yes);
        }
        let status = self.view.widget(cx, ids!(page_discover.echo_card.echo_status));
        match echo {
            Some(i) => {
                status.set_visible(cx, true);
                status.set_text(cx, &format!("已留下「{}」· 随行程到期", ECHOES[i]));
            }
            None => status.set_visible(cx, false),
        }
        self.refresh_aside(cx);
    }

    // ---- 发布向导 ----

    /// 当前草稿对应的一行预览文案。
    fn draft_publish(&self) -> Publish {
        Publish {
            day: self.draft.0,
            slot: self.draft.1,
            area: self.draft.2,
            intent: self.draft.3,
            status: PublishStatus::Draft,
        }
    }

    /// 当前筛选 + 搜索下的片区列表。
    fn picker_hits(&self) -> Vec<&'static areas::Area> {
        let kind = (self.pick_kind > 0).then(|| areas::AreaKind::ALL[self.pick_kind - 1]);
        let districts = areas::districts();
        let dist = (self.pick_dist > 0)
            .then(|| districts.get(self.pick_dist - 1).copied())
            .flatten();
        areas::search(&self.pick_query, kind)
            .into_iter()
            .filter(|a| dist.is_none_or(|d| a.district == d))
            .collect()
    }

    fn refresh_publish(&mut self, cx: &mut Cx) {
        let Some(step) = self.wizard else {
            return;
        };
        let today = today_days();

        for (j, id) in [live_id!(pw_s1), live_id!(pw_s2), live_id!(pw_s3)]
            .iter()
            .enumerate()
        {
            self.view
                .widget(cx, &[live_id!(page_publish), *id])
                .set_visible(cx, j == step);
        }
        self.view
            .label(cx, ids!(page_publish.pw_top.pw_step))
            .set_text(cx, &format!("{} / 3", step + 1));
        self.view
            .widget(cx, ids!(page_publish.pw_bar.pw_prev))
            .set_visible(cx, step > 0);
        self.view
            .button(cx, ids!(page_publish.pw_bar.pw_next))
            .set_text(cx, if step == 2 { "确认发布" } else { "下一步" });
        // 第 2 步点中某一行会直接往前走，但「下一步」不能收起来 ——
        // 改行程的人常常只改时间，片区照旧，没有按钮就卡在这一步了。
        // 第 2 步副标题改成提示当前选的是哪一个。
        let sub = if step == 1 {
            format!(
                "已选「{}」，可点别处更换。",
                areas::area_name(self.draft.2)
            )
        } else {
            "别人只看到一行模糊文字，没有昵称、头像和位置。".to_string()
        };
        self.view
            .label(cx, ids!(page_publish.pw_sub))
            .set_text(cx, &sub);

        // ---- 第 1 步：时间 ----
        for (j, id) in PUB_DAY_CHIPS.iter().enumerate() {
            let path = [
                live_id!(page_publish),
                live_id!(pw_s1),
                live_id!(s1_card),
                live_id!(s1_days),
                *id,
            ];
            self.view.check_box(cx, &path).set_text(day_label_at(today, j));
            self.view
                .check_box(cx, &path)
                .set_active(cx, j == self.draft.0, Animate::Yes);
        }
        for (j, id) in PUB_SLOT_CHIPS.iter().enumerate() {
            self.view
                .check_box(
                    cx,
                    &[
                        live_id!(page_publish),
                        live_id!(pw_s1),
                        live_id!(s1_card),
                        live_id!(s1_slots),
                        *id,
                    ],
                )
                .set_active(cx, j == self.draft.1, Animate::Yes);
        }

        // ---- 第 2 步：片区 ----
        self.view
            .widget(cx, ids!(page_publish.pw_s2.s2_filters))
            .set_visible(cx, self.pick_filters);
        self.view
            .button(cx, ids!(page_publish.pw_s2.s2_bar.s2_filter))
            .set_text(
                cx,
                match (self.pick_kind, self.pick_dist) {
                    (0, 0) => "筛选",
                    _ => "筛选 · 已启用",
                },
            );
        for (j, id) in KIND_CHIPS.iter().enumerate() {
            self.view
                .check_box(
                    cx,
                    &[
                        live_id!(page_publish),
                        live_id!(pw_s2),
                        live_id!(s2_filters),
                        live_id!(s2_kinds),
                        *id,
                    ],
                )
                .set_active(cx, j == self.pick_kind, Animate::Yes);
        }
        for (j, id) in DIST_CHIPS.iter().enumerate() {
            self.view
                .check_box(
                    cx,
                    &[
                        live_id!(page_publish),
                        live_id!(pw_s2),
                        live_id!(s2_filters),
                        live_id!(s2_dists),
                        *id,
                    ],
                )
                .set_active(cx, j == self.pick_dist, Animate::Yes);
        }
        // 最近去过：只在本机，不搜索也能一步选回常去的地方。
        let recent = self.state.recent_areas.clone();
        for (j, id) in RECENT_ROWS.iter().enumerate() {
            match recent.get(j) {
                Some(a) => {
                    let o = area_opportunity_at(today, self.draft.0, *a);
                    self.fill_area_row(cx, *id, *a, o.level, o.best_slot);
                }
                None => self.view.widget(cx, &[*id]).set_visible(cx, false),
            }
        }
        let has_recent = !recent.is_empty() && self.pick_query.trim().is_empty();
        for id in [live_id!(s2_recent_head), live_id!(s2_recent)] {
            self.view
                .widget(cx, &[live_id!(page_publish), live_id!(pw_s2), id])
                .set_visible(cx, has_recent);
        }
        let hits: Vec<AreaOpportunity> = self
            .picker_hits()
            .into_iter()
            .take(PICK_ROWS.len())
            .map(|a| area_opportunity_at(today, self.draft.0, a.id))
            .collect();
        let total = self.picker_hits().len();
        for (j, id) in PICK_ROWS.iter().enumerate() {
            match hits.get(j) {
                Some(o) => self.fill_area_row(cx, *id, o.area, o.level, o.best_slot),
                None => self.view.widget(cx, &[*id]).set_visible(cx, false),
            }
        }
        self.view
            .widget(cx, ids!(page_publish.pw_s2.s2_list))
            .set_visible(cx, total > 0);
        self.view
            .widget(cx, ids!(page_publish.pw_s2.s2_list_head))
            .set_visible(cx, total > 0);
        self.view
            .widget(cx, ids!(page_publish.pw_s2.s2_empty))
            .set_visible(cx, total == 0);
        let more = total.saturating_sub(PICK_ROWS.len());
        let more_text = if more > 0 {
            format!("还有 {more} 个片区，可搜名字或拼音首字母。")
        } else {
            String::new()
        };
        self.view
            .label(cx, ids!(page_publish.pw_s2.s2_more))
            .set_text(cx, &more_text);
        self.view
            .widget(cx, ids!(page_publish.pw_s2.s2_more))
            .set_visible(cx, more > 0);
        self.view
            .label(cx, ids!(page_publish.pw_s2.s2_list_head))
            .set_text(
                cx,
                if !self.pick_query.trim().is_empty() {
                    "搜索结果"
                } else if self.pick_kind != 0 || self.pick_dist != 0 {
                    // 没搜字只开了筛选，叫「搜索结果」会让人以为自己搜过什么。
                    "筛选结果"
                } else {
                    "全部片区"
                },
            );

        // ---- 第 3 步：意愿与预览 ----
        for (j, id) in PUB_INTENT_CHIPS.iter().enumerate() {
            self.view
                .check_box(
                    cx,
                    &[
                        live_id!(page_publish),
                        live_id!(pw_s3),
                        live_id!(s3_card),
                        live_id!(s3_intents),
                        *id,
                    ],
                )
                .set_active(cx, j == self.draft.3, Animate::Yes);
        }
        let preview = self.draft_publish().text_at(today);
        self.view
            .label(cx, ids!(page_publish.pw_s3.s3_card2.s3_text))
            .set_text(cx, &preview);
        let editing = self.state.publish.is_some();
        self.view
            .label(cx, ids!(page_publish.pw_top.pw_title))
            .set_text(cx, if editing { "修改你的去向" } else { "写一下你的去向" });
    }

    /// 打开发布向导。`area` 给了就预选那个片区（从排行里点进来的情况）。
    fn open_wizard(&mut self, cx: &mut Cx, area: Option<u16>) {
        let base = self.state.publish.clone();
        self.draft = match &base {
            Some(p) => (p.day, p.slot, p.area, p.intent),
            None => (self.day_sel.min(DAY_SPAN - 1), 1, areas::AREAS[0].id, 0),
        };
        if let Some(a) = area {
            self.draft.2 = a;
        }
        self.pick_query.clear();
        self.pick_kind = 0;
        self.pick_dist = 0;
        self.pick_filters = false;
        self.view
            .text_input(cx, ids!(page_publish.pw_s2.s2_bar.s2_search))
            .set_text(cx, "");
        self.update_page_visibility(cx);
        self.goto_step(cx, 0);
    }

    /// 切到向导的另一步。必须顺手把页面滚回顶部 —— 第 2 步的片区列表
    /// 很长，人往下滚到底再点「上一步」，第 1 步就成了一屏空白。
    fn goto_step(&mut self, cx: &mut Cx, step: usize) {
        self.wizard = Some(step);
        self.view
            .view(cx, ids!(page_publish.pw_scroll))
            .set_scroll_pos(cx, Vec2d::default());
        self.refresh_publish(cx);
        self.redraw(cx);
    }

    /// 关闭向导，草稿丢弃。
    fn close_wizard(&mut self, cx: &mut Cx) {
        self.wizard = None;
        self.update_page_visibility(cx);
        self.refresh_discover(cx);
        self.redraw(cx);
    }

    // ---- 相遇页 ----

    /// 只在等待态跑秒表。进/出等待态各调一次，别处不用管。
    fn sync_tick(&mut self, cx: &mut Cx) {
        let want = self.session.as_ref().map(|s| s.stage) == Some(RecogStage::Waiting);
        if want && self.tick.is_empty() {
            self.tick = cx.start_interval(1.0);
        } else if !want && !self.tick.is_empty() {
            cx.stop_timer(self.tick);
            self.tick = Timer::empty();
        }
    }

    /// 开发者选项里替对方按的那一下。没走到等待态就什么都不做。
    fn apply_dev_act(&mut self, cx: &mut Cx, act: DevAct) {
        let Some(s) = self.session.as_mut() else { return };
        let changed = match act {
            DevAct::Confirm => s.peer_confirm(),
            DevAct::Mismatch => {
                s.peer_mismatch();
                s.stage == RecogStage::Mismatch
            }
            DevAct::NotSamePlace => s.not_same_place(),
            DevAct::NoStock => s.no_stock(),
            DevAct::Expire => {
                if s.stage == RecogStage::Waiting {
                    s.elapsed = RECOG_WINDOW_SECS - 1;
                    s.tick()
                } else {
                    false
                }
            }
        };
        if changed {
            self.on_stage_changed(cx);
            // 按完就回相遇页 —— 否则结果出在一个人看不见的页面上。
            if self.state.tab != 1 {
                self.set_tab(cx, 1);
            }
        }
    }

    /// 会话状态刚变过：成功就出券，然后刷新页面并校准秒表。
    fn on_stage_changed(&mut self, cx: &mut Cx) {
        let stage = self.session.as_ref().map(|s| s.stage);
        // 确认成功即出券 —— 结果屏上没有「领取」这一步。手里还有一张没核销的
        // 就不再发（批次 4 的券包才谈多张并存）。
        let today = today_days();
        if stage == Some(RecogStage::Success) && !self.state.has_available_reward(today) {
            let seed = self.state.encounters.len() * 7 + self.state.contacts.len();
            self.state.issue_reward(today, seed);
        }
        // 结果屏上默认保存这次回忆；互认没成立的两条路不预设写入。
        if let Some(s) = self.session.as_mut() {
            if matches!(s.stage, RecogStage::Expired | RecogStage::Mismatch) {
                s.choice = MemoryChoice::Skip;
            }
        }
        self.code_open = false;
        self.sync_tick(cx);
        self.refresh_meet(cx);
    }

    fn refresh_meet(&mut self, cx: &mut Cx) {
        let n = self.state.contacts.len();
        if n > 0 && self.meet_sel >= n {
            self.meet_sel = 0;
        }
        self.refresh_meet_pick(cx, n);

        let stage = self.session.as_ref().map(|s| s.stage);
        self.view
            .widget(cx, ids!(page_meet.meet_pick))
            .set_visible(cx, stage.is_none());
        self.view
            .widget(cx, ids!(page_meet.meet_gate))
            .set_visible(cx, matches!(stage, Some(RecogStage::LocationGate | RecogStage::NoLocation)));
        self.view
            .widget(cx, ids!(page_meet.meet_wait))
            .set_visible(cx, stage == Some(RecogStage::Waiting));
        self.view
            .widget(cx, ids!(page_meet.meet_result))
            .set_visible(cx, stage.map(|s| s.is_result()) == Some(true));

        match stage {
            Some(RecogStage::LocationGate) | Some(RecogStage::NoLocation) => self.refresh_meet_gate(cx),
            Some(RecogStage::Waiting) => self.refresh_meet_wait(cx),
            Some(st) if st.is_result() => self.refresh_meet_result(cx, st),
            _ => {}
        }
        self.refresh_aside(cx);
    }

    /// ① 选人。
    fn refresh_meet_pick(&mut self, cx: &mut Cx, n: usize) {
        for i in 0..MEET_ROWS.len() {
            let base = [live_id!(page_meet), live_id!(meet_pick), live_id!(pick_card), MEET_ROWS[i]];
            if i >= n {
                self.view.widget(cx, &base).set_visible(cx, false);
                continue;
            }
            let (label, count) = {
                let c = &self.state.contacts[i];
                (c.label.clone(), self.state.meeting_count(c.id))
            };
            // 首字色块代替头像：够认人，又不是一张会泄露身份的图。
            let initial = label.chars().next().map(String::from).unwrap_or_default();
            self.view.widget(cx, &base).set_visible(cx, true);
            let body = |id: LiveId| [base[0], base[1], base[2], base[3], live_id!(ps_body), id];
            self.view
                .widget(cx, &[base[0], base[1], base[2], base[3], live_id!(ps_body), live_id!(ps_face), live_id!(ps_initial)])
                .set_text(cx, &initial);
            self.view
                .widget(cx, &[base[0], base[1], base[2], base[3], live_id!(ps_body), live_id!(ps_col), live_id!(ps_name)])
                .set_text(cx, &label);
            self.view
                .widget(cx, &[base[0], base[1], base[2], base[3], live_id!(ps_body), live_id!(ps_col), live_id!(ps_sub)])
                .set_text(cx, &format!("相遇 {} 次", count));
            self.view
                .widget(cx, &body(live_id!(ps_tick)))
                .set_visible(cx, i == self.meet_sel);
        }
        self.view
            .widget(cx, ids!(page_meet.meet_pick.pick_card.pc_empty))
            .set_visible(cx, n == 0);
        self.view
            .widget(cx, ids!(page_meet.meet_pick.pick_go))
            .set_visible(cx, n > 0);
        self.view
            .widget(cx, ids!(page_meet.meet_pick.pick_plain))
            .set_visible(cx, n > 0);
    }

    /// ② 定位门槛。拒绝之后仍停在这一屏 —— 会话始终没有建立。
    fn refresh_meet_gate(&mut self, cx: &mut Cx) {
        let denied = self.session.as_ref().map(|s| s.stage) == Some(RecogStage::NoLocation);
        self.view
            .widget(cx, ids!(page_meet.meet_gate.gate_card.gt_denied))
            .set_visible(cx, denied);
        self.view
            .widget(cx, ids!(page_meet.meet_gate.gate_card.gt_denied))
            .set_text(
                cx,
                "没有定位就无法确认同地，这次拿不到相遇礼。\n仍可只记一笔回忆。",
            );
        self.view
            .widget(cx, ids!(page_meet.meet_gate.gate_card.gt_alt))
            .set_visible(cx, denied);
        self.view
            .widget(cx, ids!(page_meet.meet_gate.gate_card.gt_row.gate_allow))
            .set_text(cx, if denied { "去开启定位" } else { "开启定位并确认" });
        self.view
            .widget(cx, ids!(page_meet.meet_gate.gate_card.gt_row.gate_deny))
            .set_visible(cx, !denied);
    }

    /// ③ 等待对方确认。这一屏上没有「会话码」「本人提交」「模拟」这些字眼。
    fn refresh_meet_wait(&mut self, cx: &mut Cx) {
        let Some(s) = self.session.as_ref() else { return };
        let (label, code, left, progress) =
            (s.label.clone(), s.code.clone(), s.remaining_secs(), s.progress());
        self.view
            .widget(cx, ids!(page_meet.meet_wait.wait_card.wt_title))
            .set_text(cx, &format!("等待 {} 确认", label));
        self.view
            .widget(cx, ids!(page_meet.meet_wait.wait_card.wt_count))
            .set_text(cx, &format!("还可确认 {}", countdown_label(left)));
        let mut ring = self.view.widget(cx, ids!(page_meet.meet_wait.wait_card.wt_ring));
        script_apply_eval!(cx, ring, { draw_bg +: { progress: #(progress) } });
        let open = self.code_open;
        self.view
            .widget(cx, ids!(page_meet.meet_wait.wait_card.wt_more))
            .set_text(cx, if open { "收起" } else { "对方没有偶遇？" });
        self.view
            .widget(cx, ids!(page_meet.meet_wait.wait_card.wt_fold))
            .set_visible(cx, open);
        if open {
            self.view
                .widget(cx, ids!(page_meet.meet_wait.wait_card.wt_fold.wf_code))
                .set_text(cx, &code);
            self.view
                .widget(cx, ids!(page_meet.meet_wait.wait_card.wt_fold.wf_url))
                .set_text(cx, &format!("ouyu.app/j/{}", code));
        }
    }

    /// ④ 结果。六条路径同一张屏，差别只在文案、有没有券、给不给重试。
    fn refresh_meet_result(&mut self, cx: &mut Cx, stage: RecogStage) {
        let (label, attempts, can_retry) = match self.session.as_ref() {
            Some(s) => (s.label.clone(), s.attempts, s.can_retry()),
            None => return,
        };
        // 「同地不成立」和「没有库存」必须分开说：无库存时谎称校验失败，
        // 等于让人以为自己没真的遇见。
        let (ok, title, sub) = match stage {
            RecogStage::Success => (
                true,
                format!("与 {} 的相遇已确认", label),
                "两边都确认了，位置也对得上。".to_string(),
            ),
            RecogStage::Ordinary => (
                true,
                format!("与 {} 的相遇已确认", label),
                "确认成立，但没能证明同地，这次没有相遇礼。"
                    .to_string(),
            ),
            RecogStage::NoStock => (
                true,
                format!("与 {} 的相遇已确认", label),
                "相遇成立，只是相遇礼没库存了（每对朋友 7 天一张）。"
                    .to_string(),
            ),
            RecogStage::Expired => (
                false,
                "这次没能确认".to_string(),
                format!("{} 没在 10 分钟内确认，可能只是没顾上。", label),
            ),
            _ => (
                false,
                "两边的信息还对不上".to_string(),
                if attempts >= 3 {
                    "3 次都没对上，当面核对后再发起。".to_string()
                } else {
                    format!("第 {} 次没对上，各自检查选的是谁。", attempts)
                },
            ),
        };
        self.view
            .widget(cx, ids!(page_meet.meet_result.res_card.rs_mark.rs_ok))
            .set_visible(cx, ok);
        self.view
            .widget(cx, ids!(page_meet.meet_result.res_card.rs_mark.rs_no))
            .set_visible(cx, !ok);
        self.view
            .widget(cx, ids!(page_meet.meet_result.res_card.rs_title))
            .set_text(cx, &title);
        self.view
            .widget(cx, ids!(page_meet.meet_result.res_card.rs_sub))
            .set_text(cx, &sub);

        // 券与店家只在成功那条路上出现。
        let success = stage == RecogStage::Success;
        self.view
            .widget(cx, ids!(page_meet.meet_result.coupon_card))
            .set_visible(cx, success);
        self.view
            .widget(cx, ids!(page_meet.meet_result.shops_card))
            .set_visible(cx, success);
        if success {
            self.refresh_coupon(cx);
        }

        // 互认成立才谈「这次回忆怎么留」。
        let settled = matches!(
            stage,
            RecogStage::Success | RecogStage::Ordinary | RecogStage::NoStock
        );
        self.view
            .widget(cx, ids!(page_meet.meet_result.choice_card))
            .set_visible(cx, settled);
        if settled {
            let note = match self.choice {
                MemoryChoice::Save => "只存「和谁 · 哪一天」，不记地点或精确时刻。",
                MemoryChoice::Hidden => "隐藏本次仍会保存，可在回忆页逐条恢复；不影响旧记录。",
                MemoryChoice::Skip => "不保存只影响这次，旧记录不受影响，也不累计次数。",
            };
            self.view
                .widget(cx, ids!(page_meet.meet_result.choice_card.ch_note))
                .set_text(cx, note);
            self.set_chip_group(
                cx,
                &[live_id!(page_meet), live_id!(meet_result), live_id!(choice_card), live_id!(choice_row)],
                &CHOICE_CHIPS,
                choice_index(self.choice),
            );
        }

        // 每条异常都得有出口：能重试的给重试，不能重试的至少能留一笔。
        let retry = stage == RecogStage::Expired || (stage == RecogStage::Mismatch && can_retry);
        self.view
            .widget(cx, ids!(page_meet.meet_result.res_row.rs_retry))
            .set_visible(cx, retry);
        self.view
            .widget(cx, ids!(page_meet.meet_result.res_row.rs_plain))
            .set_visible(cx, !settled);
    }

    fn refresh_coupon(&mut self, cx: &mut Cx) {
        let Some(r) = self.state.latest_reward().cloned() else { return };
        let base = [live_id!(page_meet), live_id!(meet_result), live_id!(coupon_card)];
        let put = |me: &mut Self, cx: &mut Cx, path: &[LiveId], text: &str| {
            me.view.widget(cx, path).set_text(cx, text);
        };
        put(self, cx, &[base[0], base[1], base[2], live_id!(ck_head), live_id!(ck_venue)], &r.venue);
        put(self, cx, &[base[0], base[1], base[2], live_id!(ck_offer)], &r.offer);
        put(
            self,
            cx,
            &[base[0], base[1], base[2], live_id!(ck_terms)],
            r.terms.as_deref().unwrap_or("以券面条款为准"),
        );
        put(
            self,
            cx,
            &[base[0], base[1], base[2], live_id!(ck_meta), live_id!(ck_token)],
            &format!("核销码 {}", r.token.as_deref().unwrap_or("—")),
        );
        put(
            self,
            cx,
            &[base[0], base[1], base[2], live_id!(ck_meta), live_id!(ck_expiry)],
            &r.expiry_label(),
        );
        put(
            self,
            cx,
            &[base[0], base[1], base[2], live_id!(ck_status)],
            if r.redeemed { "已核销（模拟）" } else { "到店出示核销码" },
        );
        self.view
            .widget(cx, &[base[0], base[1], base[2], live_id!(ck_row), live_id!(ck_redeem)])
            .set_visible(cx, !r.redeemed);

        for (i, m) in MERCHANTS.iter().enumerate() {
            let row = [live_id!(page_meet), live_id!(meet_result), live_id!(shops_card), SHOP_ROWS[i]];
            self.view
                .widget(cx, &[row[0], row[1], row[2], row[3], live_id!(sp_head), live_id!(sp_name)])
                .set_text(cx, m.name);
            self.view
                .widget(cx, &[row[0], row[1], row[2], row[3], live_id!(sp_head), live_id!(sp_walk)])
                .set_text(cx, m.walk);
            self.view
                .widget(cx, &[row[0], row[1], row[2], row[3], live_id!(sp_addr)])
                .set_text(cx, m.address);
            self.view
                .widget(cx, &[row[0], row[1], row[2], row[3], live_id!(sp_hours)])
                .set_text(cx, m.hours);
        }
    }

    /// 结束本次相遇：按本次选择写入回忆，回到发现页。
    fn finish_meet(&mut self, cx: &mut Cx) {
        if let Some(mut s) = self.session.take() {
            self.state.write_session_memory(&mut s);
        }
        self.code_open = false;
        self.sync_tick(cx);
        self.refresh_contacts(cx);
        self.refresh_memories(cx);
        self.set_tab(cx, 0);
    }

    // ---- 熟人页 ----

    fn refresh_contacts(&mut self, cx: &mut Cx) {
        let n = self.state.contacts.len();
        self.view
            .widget(cx, ids!(page_contacts.ct_card.ct_head.ct_head_text))
            .set_text(cx, &format!("本机熟人 · {} 位", n));

        // 按字母排一次。`sort_by_key` 是稳定的，同一个字母里保持加入的先后，
        // 刷新一次不会自己换位置。「#」（认不出姓的）排到最后。
        let mut order: Vec<(u8, char, usize)> = self
            .state
            .contacts
            .iter()
            .map(|c| {
                let k = alpha_key(&c.label);
                (if k == '#' { 1 } else { 0 }, k, c.id)
            })
            .collect();
        order.sort_by_key(|(tail, k, _)| (*tail, *k));
        self.contact_order = order.iter().map(|(_, _, id)| *id).collect();

        // 合并模式的说明条。
        let merging = self
            .merge_from
            .and_then(|id| self.state.contact(id))
            .map(|c| c.label.clone());
        self.view
            .widget(cx, ids!(page_contacts.ct_card.ct_merge_bar))
            .set_visible(cx, merging.is_some());
        if let Some(from) = &merging {
            let text = format!("把「{}」并到谁？点另一位的「并到这里」。", from);
            self.view
                .widget(cx, ids!(page_contacts.ct_card.ct_merge_bar.cm_text))
                .set_text(cx, &text);
        }
        // 添加失败的那一句红字。
        self.view
            .widget(cx, ids!(page_contacts.ct_card.ca_err))
            .set_visible(cx, self.add_error.is_some());
        if let Some(e) = self.add_error {
            self.view
                .widget(cx, ids!(page_contacts.ct_card.ca_err))
                .set_text(cx, e.text());
        }

        let show_list = self.apply_list_state(
            cx,
            ids!(page_contacts.ct_card.ct_empty),
            "还没有熟人",
            "从通讯录导入",
            n,
        );
        let mut last_letter = '\0';
        for i in 0..CONTACT_ROWS.len() {
            let base = [live_id!(page_contacts), live_id!(ct_card), CONTACT_ROWS[i]];
            let letter = [live_id!(page_contacts), live_id!(ct_card), CONTACT_LETTERS[i]];
            if i < n && show_list {
                let Some((id, label, count)) = self
                    .contact_order
                    .get(i)
                    .and_then(|id| self.state.contact(*id))
                    .map(|c| (c.id, c.label.clone()))
                    .map(|(id, label)| (id, label, self.state.meeting_count(id)))
                else {
                    self.view.widget(cx, &base).set_visible(cx, false);
                    self.view.widget(cx, &letter).set_visible(cx, false);
                    continue;
                };
                // 字母索引：和上一行不同字母时才立一个头。
                let key = alpha_key(&label);
                let show_letter = key != last_letter;
                last_letter = key;
                self.view.widget(cx, &letter).set_visible(cx, show_letter);
                if show_letter {
                    self.view
                        .widget(cx, &letter)
                        .set_text(cx, &key.to_string());
                }
                // 合并模式下，源那一行的按钮变「取消」，其余变「并到这里」。
                let merge_text = match self.merge_from {
                    Some(f) if f == id => "取消",
                    Some(_) => "并到这里",
                    None => "合并",
                };
                self.view
                    .widget(cx, &[base[0], base[1], base[2], live_id!(c_main), live_id!(c_merge)])
                    .set_text(cx, merge_text);
                self.view.widget(cx, &base).set_visible(cx, true);
                self.view
                    .widget(cx, &[base[0], base[1], base[2], live_id!(c_main)])
                    .set_visible(cx, self.confirm_row != Some(i));
                self.view
                    .widget(cx, &[base[0], base[1], base[2], live_id!(c_main), live_id!(c_name)])
                    .set_text(cx, &label);
                self.view
                    .widget(cx, &[base[0], base[1], base[2], live_id!(c_main), live_id!(c_count)])
                    .set_text(cx, &format!("相遇 {} 次", count));
                let confirming = self.confirm_row == Some(i);
                self.view
                    .widget(cx, &[base[0], base[1], base[2], live_id!(c_confirm)])
                    .set_visible(cx, confirming);
                if confirming {
                    let text = "从本机熟人列表移除，不改系统通讯录。默认保留旧回忆。";
                    self.view
                        .widget(cx, &[base[0], base[1], base[2], live_id!(c_confirm), live_id!(c_ctext)])
                        .set_text(cx, text);
                    let also_path = [
                        base[0],
                        base[1],
                        base[2],
                        live_id!(c_confirm),
                        live_id!(c_also),
                    ];
                    self.view.widget(cx, &also_path).set_visible(cx, true);
                    self.view
                        .check_box(cx, &also_path)
                        .set_active(cx, self.confirm_also, Animate::No);
                }
            } else {
                self.view.widget(cx, &base).set_visible(cx, false);
                self.view.widget(cx, &letter).set_visible(cx, false);
            }
        }
        self.view
            .widget(cx, ids!(page_contacts.dir_head.dir_label))
            .set_text(cx, &format!("通讯录 · {} 人", self.state.directory.len()));
        let dir = self.state.directory.join("  ");
        self.view
            .widget(cx, ids!(page_contacts.dir_list))
            .set_text(cx, &dir);
        self.refresh_aside(cx);
    }

    // ---- 桌面右栏 ----

    /// 当页上下文面板。宽屏才有这一栏，窄屏整栏不显示，所以这里不做
    /// 布局分支，只管填字。
    ///
    /// 三行一律是「你自己的那一份」：发布、券、回忆、熟人数目。
    /// 刻意不放任何跟别人有关的计数（谁在附近、几个人看到了你），
    /// 那是 03 节红线表里第一条。
    fn refresh_aside(&mut self, cx: &mut Cx) {
        let today = today_days();
        let (title, rows, tip): (&str, [(String, String); 3], &str) = match self.state.tab {
            0 => {
                let mine = match &self.state.publish {
                    Some(p) if p.status == PublishStatus::Published => p.text_at(today),
                    _ => "还没有发布".to_string(),
                };
                let looking = day_label_at(today, self.day_sel).to_string();
                let ranking = opportunity_ranking_at(today, self.day_sel);
                let best = ranking
                    .iter()
                    .find(|o| o.level.shown())
                    .map(|o| format!("{} · {}", areas::area_name(o.area), o.level.label()))
                    .unwrap_or_else(|| "还不够成局".to_string());
                (
                    "这一页在看什么",
                    [
                        ("你的去向".into(), mine),
                        ("正在看".into(), looking),
                        ("最靠前".into(), best),
                    ],
                    "档位按你的熟人算，不显示人数和是谁。",
                )
            }
            1 => {
                let people = self.state.contacts.len();
                let usable = self
                    .state
                    .rewards_in(today, RewardState::Available)
                    .len();
                let last = self
                    .state
                    .encounters
                    .iter()
                    .map(|e| e.date.clone())
                    .max()
                    .unwrap_or_else(|| "还没有".to_string());
                (
                    "互认前后",
                    [
                        ("可互认".into(), format!("{} 位", people)),
                        ("可用券".into(), format!("{} 张", usable)),
                        ("最近相遇".into(), last),
                    ],
                    "定位只在确认那一下用一次，不存不传。",
                )
            }
            2 => {
                let most = self
                    .state
                    .contacts
                    .iter()
                    .map(|c| (self.state.meeting_count(c.id), c.label.clone()))
                    .max_by_key(|(n, _)| *n)
                    .filter(|(n, _)| *n > 0)
                    .map(|(n, label)| format!("{} · {} 次", label, n))
                    .unwrap_or_else(|| "还没有".to_string());
                (
                    "这份名单",
                    [
                        ("本机熟人".into(), format!("{} 位", self.state.contacts.len())),
                        ("通讯录池".into(), format!("{} 人", self.state.directory.len())),
                        ("回忆最多".into(), most),
                    ],
                    "次数含隐藏回忆；成就页只算未隐藏的。",
                )
            }
            3 => {
                let hidden = self.state.encounters.iter().filter(|e| e.hidden).count();
                let shown = self.state.encounters.len() - hidden;
                let span = {
                    let mut ds: Vec<&str> =
                        self.state.encounters.iter().map(|e| e.date.as_str()).collect();
                    ds.sort_unstable();
                    match (ds.first(), ds.last()) {
                        (Some(a), Some(b)) => {
                            let (ha, hb) = (month_head(a), month_head(b));
                            if ha == hb { ha } else { format!("{} → {}", ha, hb) }
                        }
                        _ => "还没有".to_string(),
                    }
                };
                (
                    "这台机器上的回忆",
                    [
                        ("可见".into(), format!("{} 条", shown)),
                        ("已隐藏".into(), format!("{} 条", hidden)),
                        ("跨度".into(), span),
                    ],
                    "隐藏的不进搜索、提醒和统计。",
                )
            }
            _ => {
                let st = achievement_stats(&self.state.encounters, self.weeks, today);
                let this_week = st.weekly.last().map(|w| w.count).unwrap_or(0);
                (
                    "你的这一份",
                    [
                        ("记住的相遇".into(), format!("{} 次", st.remembered)),
                        ("相遇日".into(), format!("{} 天", st.days)),
                        ("本周".into(), format!("{} 次", this_week)),
                    ],
                    "只算未隐藏的回忆。分享卡不带名字、地点和日期。",
                )
            }
        };
        self.view
            .widget(cx, ids!(main.aside.aside_ctx.ax_title))
            .set_text(cx, title);
        for (id, (k, v)) in AX_ROWS.iter().zip(rows.iter()) {
            let base = [live_id!(main), live_id!(aside), live_id!(aside_ctx), *id];
            self.view
                .widget(cx, &[base[0], base[1], base[2], base[3], live_id!(ax_k)])
                .set_text(cx, k);
            self.view
                .widget(cx, &[base[0], base[1], base[2], base[3], live_id!(ax_v)])
                .set_text(cx, v);
        }
        self.view
            .widget(cx, ids!(main.aside.aside_ctx.ax_tip))
            .set_text(cx, tip);
    }

    /// 「导入 vCard」：读 <MAKEPAD_HOME>/ouyu/contacts.vcf, 名字并入通讯录池并落盘。
    fn import_vcard(&mut self, cx: &mut Cx) {
        let status = self.view.widget(cx, ids!(page_contacts.vcf_row.vcf_status));
        let Some(path) = OuyuState::contacts_vcf() else {
            status.set_text(cx, "未设置 MAKEPAD_HOME, 找不到状态目录");
            return;
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                status.set_text(cx, &format!("把 .vcf 放到 {} 再点我", path.display()));
                return;
            }
        };
        let added = self.state.merge_directory(parse_vcard(&text));
        self.state.save();
        status.set_text(cx, &format!("导入成功 {} 人", added));
        self.refresh_contacts(cx);
    }

    // ---- 回忆页 ----

    fn refresh_memories(&mut self, cx: &mut Cx) {
        // 筛选选项：本机联系人称呼 ∪ 回忆里的称呼快照（含已删除联系人）。
        let mut options: Vec<String> = Vec::new();
        for c in &self.state.contacts {
            if !options.contains(&c.label) {
                options.push(c.label.clone());
            }
        }
        for e in &self.state.encounters {
            if !options.contains(&e.label_snapshot) {
                options.push(e.label_snapshot.clone());
            }
        }
        options.truncate(FILT_CHIPS.len());
        if let Some(f) = &self.memory_filter {
            if !options.contains(f) {
                self.memory_filter = None;
            }
        }
        self.filter_options = options;
        let options = self.filter_options.clone();
        let filt_all_path = [
            live_id!(page_memories),
            live_id!(filt_row),
            live_id!(filt_all),
        ];
        self.view
            .check_box(cx, &filt_all_path)
            .set_active(cx, self.memory_filter.is_none(), Animate::Yes);
        for i in 0..FILT_CHIPS.len() {
            let path = [live_id!(page_memories), live_id!(filt_row), FILT_CHIPS[i]];
            match options.get(i) {
                Some(label) => {
                    self.view.widget(cx, &path).set_visible(cx, true);
                    self.view.widget(cx, &path).set_text(cx, label);
                    self.view
                        .check_box(cx, &path)
                        .set_active(cx, self.memory_filter.as_deref() == Some(label.as_str()), Animate::Yes);
                }
                None => self.view.widget(cx, &path).set_visible(cx, false),
            }
        }

        // 可见的那一段：搜索 + 按人筛选都走 `search_memories`（隐藏的进不来）。
        let filter = self.memory_filter.clone();
        self.mem_row_ids =
            search_memories(&self.state.encounters, &self.mem_query, filter.as_deref())
                .iter()
                .map(|e| e.id)
                .collect();
        // 已隐藏那一段永远不参与搜索 —— 搜索中干脆整段收起来，并说明为什么。
        let searching = !self.mem_query.trim().is_empty();
        self.hid_row_ids = if searching {
            Vec::new()
        } else {
            self.state
                .encounters
                .iter()
                .filter(|e| e.hidden)
                .filter(|e| filter.as_ref().is_none_or(|f| e.label_snapshot == *f))
                .map(|e| e.id)
                .collect()
        };

        self.fill_memory_rows(cx, &MEM_ROWS, &MEM_HEADS, true);
        self.fill_memory_rows(cx, &HID_ROWS, &HID_HEADS, false);

        let n = self.mem_row_ids.len();
        let empty_text = if searching { "没有搜到" } else { "这里暂时留白" };
        self.apply_list_state(cx, ids!(page_memories.mem_empty), empty_text, "", n);
        self.view
            .widget(cx, ids!(page_memories.mm_hidden_note))
            .set_visible(cx, searching);
        for id in [live_id!(sec_hid), live_id!(hid_empty), live_id!(mm_note)] {
            self.view
                .widget(cx, &[live_id!(page_memories), id])
                .set_visible(cx, !searching);
        }
        self.view
            .widget(cx, ids!(page_memories.hid_empty))
            .set_visible(cx, !searching && self.hid_row_ids.is_empty());
        self.refresh_memory_detail(cx);
        self.refresh_aside(cx);
    }

    /// 铺一段回忆行，顺带在每个月的第一条上面放一个月份头。
    ///
    /// 分组头是「行的一部分」而不是独立的一段：固定八个位置，行往下挪的时候
    /// 头也跟着挪，不用另算一套下标。
    fn fill_memory_rows(&mut self, cx: &mut Cx, rows: &[LiveId], heads: &[LiveId], visible: bool) {
        let ids = if visible {
            self.mem_row_ids.clone()
        } else {
            self.hid_row_ids.clone()
        };
        let mut last_month = String::new();
        for i in 0..rows.len() {
            let row = [live_id!(page_memories), rows[i]];
            let head = [live_id!(page_memories), heads[i]];
            let Some(e) = ids
                .get(i)
                .and_then(|id| self.state.encounters.iter().find(|e| e.id == *id))
            else {
                self.view.widget(cx, &row).set_visible(cx, false);
                self.view.widget(cx, &head).set_visible(cx, false);
                continue;
            };
            let (date, text) = (e.date.clone(), encounter_text(e));
            let month = month_head(&date);
            let show_head = month != last_month;
            last_month = month.clone();

            self.view.widget(cx, &head).set_visible(cx, show_head);
            if show_head {
                self.view.widget(cx, &head).set_text(cx, &month);
            }
            self.view.widget(cx, &row).set_visible(cx, true);
            self.view
                .widget(cx, &[row[0], row[1], live_id!(m_date)])
                .set_text(cx, &date);
            self.view
                .widget(cx, &[row[0], row[1], live_id!(m_text)])
                .set_text(cx, &text);
        }
    }

    /// 一条回忆的详情页。
    fn refresh_memory_detail(&mut self, cx: &mut Cx) {
        let Some(id) = self.mem_detail else { return };
        let Some(e) = self.state.encounters.iter().find(|e| e.id == id) else {
            // 这一条被删掉了（比如撤销窗口过完之后）：退回列表，不留一张空详情。
            self.mem_detail = None;
            self.update_page_visibility(cx);
            return;
        };
        let (label, date, note, hidden) =
            (e.label_snapshot.clone(), e.date.clone(), e.note.clone(), e.hidden);
        self.view
            .widget(cx, ids!(page_memdetail.md_title))
            .set_text(cx, &format!("和{}的那次", label));
        self.view.widget(cx, ids!(page_memdetail.md_date)).set_text(
            cx,
            &format!("{}{}", date, if hidden { " · 已隐藏" } else { "" }),
        );
        self.view
            .text_input(cx, ids!(page_memdetail.md_card.md_note))
            .set_text(cx, &note);
        self.view
            .widget(cx, ids!(page_memdetail.md_act.md_toggle))
            .set_text(cx, if hidden { "恢复这一条" } else { "隐藏这一条" });
    }

    // ---- 成就页 / 分享卡 ----

    fn refresh_achievements(&mut self, cx: &mut Cx) {
        let today = today_days();
        // 三个入口。券那一格直接显示「几张可用」，不用点进去才知道有没有。
        let avail = self.state.rewards_in(today, RewardState::Available).len();
        self.set_row(
            cx,
            ids!(page_achieve.hub_card.row_wallet),
            "我的券",
            "",
            &if avail > 0 {
                format!("{} 张可用", avail)
            } else if self.state.wallet.is_empty() {
                "还没有".to_string()
            } else {
                "暂无可用".to_string()
            },
        );
        self.set_row(
            cx,
            ids!(page_achieve.hub_card.row_settings),
            "设置",
            "",
            "定位 · 通知 · 数据",
        );
        self.set_row(
            cx,
            ids!(page_achieve.hub_card.row_about),
            "关于偶遇",
            "",
            "设计稿 v0.4",
        );
        let stats = achievement_stats(&self.state.encounters, self.weeks, today);
        // 摘要行。
        self.view
            .widget(cx, ids!(page_achieve.sum_card.sum_row.sum_col0.sum_n0))
            .set_text(cx, &stats.remembered.to_string());
        self.view
            .widget(cx, ids!(page_achieve.sum_card.sum_row.sum_col1.sum_n1))
            .set_text(cx, &stats.days.to_string());
        self.view
            .widget(cx, ids!(page_achieve.sum_card.sum_row.sum_col2.sum_n2))
            .set_text(cx, &stats.recent().to_string());
        self.view
            .widget(cx, ids!(page_achieve.sum_card.sum_row.sum_col2.sum_l2))
            .set_text(cx, &format!("近 {} 周相遇", self.weeks));
        // 曲线卡片：chips + 数据 + 留白。
        self.view
            .widget(cx, ids!(page_achieve.curve_card.curve_head.curve_title))
            .set_text(cx, "相遇频率曲线");
        self.set_chip_group(
            cx,
            &[
                live_id!(page_achieve),
                live_id!(curve_card),
                live_id!(curve_head),
            ],
            &WK_CHIPS,
            if self.weeks == 4 { 0 } else { 1 },
        );
        self.view
            .widget(cx, ids!(page_achieve.curve_card.curve_sub))
            .set_text(cx, &format!("最近 {} 周，记住 {} 次重逢", self.weeks, stats.recent()));
        let has_data = stats.recent() > 0;
        self.view
            .widget(cx, ids!(page_achieve.curve_card.curve_chart))
            .set_visible(cx, has_data);
        self.view
            .widget(cx, ids!(page_achieve.curve_card.curve_empty))
            .set_visible(cx, !has_data);
        let chart = self.view.widget(cx, ids!(page_achieve.curve_card.curve_chart));
        if let Some(mut c) = chart.borrow_mut::<OuyuChart>() {
            let first = stats.weekly.first().map(|w| w.start).unwrap_or(today);
            let last = stats.weekly.last().map(|w| w.start).unwrap_or(today);
            c.set_data(ChartData {
                counts: stats.weekly.iter().map(|w| w.count).collect(),
                first_label: fmt_md(first),
                last_label: fmt_md(last),
                last_note: "本周尚未结束".into(),
            });
        }
        chart.redraw(cx);
        // 可展开的「查看每周次数」数据表（不依赖 hover 读数）。
        self.view
            .widget(cx, ids!(page_achieve.curve_card.wk_toggle))
            .set_text(cx, if self.weekly_open { "收起" } else { "每周次数" });
        self.view
            .widget(cx, ids!(page_achieve.curve_card.wk_table))
            .set_visible(cx, self.weekly_open);
        for (i, id) in WK_ROWS.iter().enumerate() {
            let base = [
                live_id!(page_achieve),
                live_id!(curve_card),
                live_id!(wk_table),
                *id,
            ];
            match stats.weekly.get(i) {
                Some(w) if self.weekly_open => {
                    self.view.widget(cx, &base).set_visible(cx, true);
                    self.view
                        .widget(cx, &[base[0], base[1], base[2], base[3], live_id!(wk_d)])
                        .set_text(cx, &format!("{} 这一周", fmt_md(w.start)));
                    self.view
                        .widget(cx, &[base[0], base[1], base[2], base[3], live_id!(wk_c)])
                        .set_text(cx, &format!("{} 次", w.count));
                }
                _ => self.view.widget(cx, &base).set_visible(cx, false),
            }
        }
        // 里程碑：点亮 / 等自然发生。
        let ms = milestones(&stats);
        for (i, id) in MS_ROWS.iter().enumerate() {
            let base = [
                live_id!(page_achieve),
                live_id!(ms_card),
                live_id!(ms_row),
                *id,
            ];
            let m = &ms[i];
            self.view
                .widget(cx, &[base[0], base[1], base[2], base[3], live_id!(ms_icon)])
                .set_text(cx, if m.lit { "★" } else { "☆" });
            self.view
                .widget(cx, &[base[0], base[1], base[2], base[3], live_id!(ms_state)])
                .set_text(cx, if m.lit { "已点亮" } else { "等自然发生" });
        }
        if self.share_open {
            self.refresh_share(cx);
        }
        self.refresh_aside(cx);
    }

    /// 当前分享卡场景（预览与保存共用，保证所见即所得）。
    fn share_scene(&self) -> share::ShareCardScene {
        // 分享卡曲线固定近 8 周（06 节文案口径），与成就页的 4/8 切换无关。
        let stats = achievement_stats(&self.state.encounters, 8, today_days());
        let ms = milestones(&stats);
        let mut scene = share::scene_from_stats(&stats, self.share_style, &ms);
        if self.share_curve {
            scene.curve = Some(stats.weekly.iter().map(|w| w.count).collect());
        }
        scene
    }

    // ---- 列表四态 ----

    /// 把四态铺到一个 OuyuEmpty 上，返回列表本身该不该显示。
    ///
    /// `empty_text` / `empty_action` 是这一页自己的空态（每页的话都不一样）；
    /// 加载 / 出错 / 离线三个态全应用统一的图标、颜色和出口按钮 —— 出错的样子
    /// 要是每页都不同，人就没法从「见过一次」推出「这是怎么回事」。
    fn apply_list_state(
        &mut self,
        cx: &mut Cx,
        slot: &[LiveId],
        empty_text: &str,
        empty_action: &str,
        n: usize,
    ) -> bool {
        let st = self.list_state;
        let show_list = st == ListState::Ready && n > 0;
        let show_slot = !show_list;
        self.view.widget(cx, slot).set_visible(cx, show_slot);
        if !show_slot {
            return show_list;
        }
        let (text, sub, action) = match st {
            ListState::Ready => (empty_text, "", empty_action),
            ListState::Loading => ("正在加载…", "", ""),
            ListState::Failed => (
                "没能加载出来",
                "稍后再试一次。",
                "重试",
            ),
            ListState::Offline => (
                "现在连不上网",
                "本机数据照常可看；发布和互认要等网络。",
                "重试",
            ),
        };
        // 图标沿用这一页自己的那个（券 / 熟人 / 回忆），只换颜色 —— 运行时换不了
        // svg，而且一页一个图标本来就比四页一个通用图标好认。
        let mut icon_w = self.view.widget(cx, &join(slot, live_id!(em_icon)));
        match st {
            ListState::Ready => script_apply_eval!(cx, icon_w, { draw_icon +: { color: #x3c4c6b } }),
            ListState::Failed => script_apply_eval!(cx, icon_w, { draw_icon +: { color: #ff9eab } }),
            _ => script_apply_eval!(cx, icon_w, { draw_icon +: { color: #x5d6f8d } }),
        }
        self.view
            .widget(cx, &join(slot, live_id!(em_text)))
            .set_text(cx, text);
        self.view
            .widget(cx, &join(slot, live_id!(em_sub)))
            .set_visible(cx, !sub.is_empty());
        self.view
            .widget(cx, &join(slot, live_id!(em_sub)))
            .set_text(cx, sub);
        self.view
            .widget(cx, &join(slot, live_id!(em_action)))
            .set_visible(cx, !action.is_empty());
        self.view
            .widget(cx, &join(slot, live_id!(em_action)))
            .set_text(cx, action);
        show_list
    }

    // ---- 删除撤销 ----

    /// 删完之后给 5 秒后悔时间。到点或者再删一次，上一份快照就作废。
    fn offer_undo(&mut self, cx: &mut Cx, snap: UndoSnapshot) {
        if !self.undo_timer.is_empty() {
            cx.stop_timer(self.undo_timer);
        }
        self.view
            .widget(cx, ids!(toast_layer.toast.to_text))
            .set_text(cx, &snap.label);
        self.view
            .widget(cx, ids!(toast_layer.toast.to_undo))
            .set_visible(cx, true);
        self.view.widget(cx, ids!(toast_layer.toast)).set_visible(cx, true);
        self.undo = Some(snap);
        self.undo_timer = cx.start_timeout(5.0);
    }

    /// 收起 toast。`restore` 为真时把快照放回去。
    fn close_undo(&mut self, cx: &mut Cx, restore: bool) {
        if !self.undo_timer.is_empty() {
            cx.stop_timer(self.undo_timer);
            self.undo_timer = Timer::empty();
        }
        let snap = self.undo.take();
        self.view.widget(cx, ids!(toast_layer.toast)).set_visible(cx, false);
        if restore {
            if let Some(u) = snap {
                self.state.restore(u);
                self.refresh_contacts(cx);
                self.refresh_memories(cx);
                self.refresh_meet(cx);
                self.refresh_achievements(cx);
            }
        }
    }

    // ---- 我的券 ----

    fn refresh_wallet(&mut self, cx: &mut Cx) {
        let today = today_days();
        for (sec, rows, st) in [
            (live_id!(wl_avail), WA_ROWS, RewardState::Available),
            (live_id!(wl_used), WU_ROWS, RewardState::Redeemed),
            (live_id!(wl_gone), WG_ROWS, RewardState::Expired),
        ] {
            let items: Vec<RewardClaim> = self
                .state
                .rewards_in(today, st)
                .into_iter()
                .cloned()
                .take(rows.len())
                .collect();
            let show = !items.is_empty() && self.list_state == ListState::Ready;
            self.view
                .widget(cx, &[live_id!(page_wallet), sec])
                .set_visible(cx, show);
            for (i, id) in rows.iter().enumerate() {
                let base = [live_id!(page_wallet), sec, *id];
                match items.get(i) {
                    Some(r) => {
                        let r = r.clone();
                        self.view.widget(cx, &base).set_visible(cx, true);
                        self.fill_coupon(cx, &base, &r, today);
                    }
                    None => {
                        self.view.widget(cx, &base).set_visible(cx, false);
                    }
                }
            }
        }
        let n = self.state.wallet.len();
        self.apply_list_state(
            cx,
            ids!(page_wallet.wl_state),
            "还没有相遇礼",
            "",
            n,
        );
    }

    /// 填一张券。已核销 / 已过期的整张降下来，但不隐藏 —— 核销码留着，
    /// 人有时要拿它对账。
    fn fill_coupon(&mut self, cx: &mut Cx, base: &[LiveId], r: &RewardClaim, today: i64) {
        let st = r.state(today);
        let head = join(base, live_id!(cw_head));
        self.view
            .widget(cx, &join(&head, live_id!(cw_venue)))
            .set_text(cx, &r.venue);
        let state_text = match st {
            RewardState::Available => r
                .remaining_label(today)
                .unwrap_or_else(|| "可用".to_string()),
            other => other.label().to_string(),
        };
        self.view
            .widget(cx, &join(&head, live_id!(cw_state)))
            .set_text(cx, &state_text);
        self.view
            .widget(cx, &join(base, live_id!(cw_offer)))
            .set_text(cx, &r.offer);
        let meta = join(base, live_id!(cw_meta));
        self.view
            .widget(cx, &join(&meta, live_id!(cw_token)))
            .set_text(cx, &format!("核销码 {}", r.token.as_deref().unwrap_or("—")));
        self.view
            .widget(cx, &join(&meta, live_id!(cw_expiry)))
            .set_text(cx, &r.expiry_label());
        self.view
            .widget(cx, &join(base, live_id!(cw_terms)))
            .set_text(cx, r.terms.as_deref().unwrap_or("以券面条款为准"));
        self.view
            .widget(cx, &join(&join(base, live_id!(cw_row)), live_id!(cw_redeem)))
            .set_visible(cx, st == RewardState::Available);
        let mut card = self.view.widget(cx, base);
        if st == RewardState::Available {
            script_apply_eval!(cx, card, { draw_bg +: { color: #ffca91 } });
        } else {
            script_apply_eval!(cx, card, { draw_bg +: { color: #x8f7c66 } });
        }
    }

    // ---- 设置 ----

    fn refresh_settings(&mut self, cx: &mut Cx) {
        let granted = self.state.settings.location_granted;
        self.set_row(
            cx,
            ids!(page_settings.loc_card.row_loc),
            "相遇时使用定位",
            "只在互认那一下用一次，不留坐标",
            if granted { "已开启" } else { "未开启" },
        );
        self.set_switch(
            cx,
            ids!(page_settings.ntf_card.row_ntf_publish),
            "行程快到期时提醒我",
            "只提醒你自己的那一条，不提醒别人的",
            self.state.settings.notify_publish,
        );
        self.set_switch(
            cx,
            ids!(page_settings.ntf_card.row_ntf_reward),
            "券快过期时提醒我",
            "剩最后两天时提醒一次",
            self.state.settings.notify_reward,
        );
        let export = self
            .export_note
            .clone()
            .unwrap_or_else(|| "JSON".to_string());
        self.set_row(
            cx,
            ids!(page_settings.data_card.row_export),
            "导出本机数据",
            "写成一个文件放进应用目录",
            &export,
        );
        self.set_row(
            cx,
            ids!(page_settings.data_card.row_clear),
            "清除本机数据",
            "不可撤销，设置会留着",
            "",
        );
        let mut clear_name = self
            .view
            .widget(cx, ids!(page_settings.data_card.row_clear.st_body.st_col.st_name));
        script_apply_eval!(cx, clear_name, { draw_text +: { color: #ff9eab } });
        self.view
            .widget(cx, ids!(page_settings.data_card.clear_confirm))
            .set_visible(cx, self.clear_armed);
    }

    /// 写一条设置行（名称 / 说明 / 右侧当前值）。
    fn set_row(&mut self, cx: &mut Cx, row: &[LiveId], name: &str, sub: &str, value: &str) {
        let body = join(row, live_id!(st_body));
        let col = join(&body, live_id!(st_col));
        self.view
            .widget(cx, &join(&col, live_id!(st_name)))
            .set_text(cx, name);
        self.view
            .widget(cx, &join(&col, live_id!(st_sub)))
            .set_visible(cx, !sub.is_empty());
        self.view
            .widget(cx, &join(&col, live_id!(st_sub)))
            .set_text(cx, sub);
        self.view
            .widget(cx, &join(&body, live_id!(st_val)))
            .set_text(cx, value);
    }

    /// 写一条开关行。开关的样子只跟着传进来的 `on` 走。
    fn set_switch(&mut self, cx: &mut Cx, row: &[LiveId], name: &str, sub: &str, on: bool) {
        let body = join(row, live_id!(sw_body));
        let col = join(&body, live_id!(sw_col));
        self.view
            .widget(cx, &join(&col, live_id!(sw_name)))
            .set_text(cx, name);
        self.view
            .widget(cx, &join(&col, live_id!(sw_sub)))
            .set_text(cx, sub);
        let track = join(&body, live_id!(sw_track));
        let mut track_w = self.view.widget(cx, &track);
        if on {
            script_apply_eval!(cx, track_w, { draw_bg +: { color: #82b5ff } });
        } else {
            script_apply_eval!(cx, track_w, { draw_bg +: { color: #x24354f } });
        }
        self.view
            .widget(cx, &join(&track, live_id!(sw_off)))
            .set_visible(cx, !on);
        self.view
            .widget(cx, &join(&track, live_id!(sw_on)))
            .set_visible(cx, on);
    }

    fn refresh_share(&mut self, cx: &mut Cx) {
        let scene = self.share_scene();
        let card = self
            .view
            .widget(cx, ids!(page_share.sp_row.sp_left.sp_center.sh_card));
        if let Some(mut c) = card.borrow_mut::<OuyuShareCard>() {
            c.set_scene(scene);
        }
        card.redraw(cx);
        self.set_chip_group(
            cx,
            &[
                live_id!(page_share),
                live_id!(sp_row),
                live_id!(sp_right),
                live_id!(sp_style),
                live_id!(sps_row),
            ],
            &STYLE_CHIPS,
            match self.share_style {
                ShareStyle::Warm => 0,
                ShareStyle::Night => 1,
            },
        );
        self.view
            .check_box(
                cx,
                ids!(page_share.sp_row.sp_right.sp_what.sh_curve),
            )
            .set_active(cx, self.share_curve, Animate::Yes);
        self.view
            .widget(cx, ids!(page_share.sp_row.sp_right.sp_what.sh_curve_hint))
            .set_visible(cx, self.share_curve);
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // 侧栏 Tab / 手机底部导航（同一组 Tab 的两份控件）
        for i in 0..TABS.len() {
            let sidebar = self
                .view
                .check_box(cx, &[live_id!(sidebar), TABS[i]])
                .changed(actions)
                .is_some();
            let bottom = self
                .view
                .check_box(cx, &[live_id!(tabbar), TABS[i]])
                .changed(actions)
                .is_some();
            if sidebar || bottom {
                self.set_tab(cx, i);
            }
        }
        // 界面模式：自动 → 手机 → 大屏 → 自动。宿主整机的手机 / 桌面切换在样式菜单里。
        if self.view.button(cx, ids!(sidebar.sb_mode)).clicked(actions)
            || self.view.button(cx, ids!(shell.topbar.tb_mode)).clicked(actions)
        {
            self.ui_mode = self.ui_mode.next();
            self.shaping = None;
            self.refresh_mode_buttons(cx);
            self.redraw(cx);
        }
        // 开发者选项: 草图场景（阈值不足分支），不代表附近真实人数。
        if self.view.button(cx, ids!(page_settings.dev_card.dv_sparse)).clicked(actions) {
            self.sparse = !self.sparse;
            self.refresh_discover(cx);
        }
        // 开发者选项: 替对方这一侧按一下。会话没走到等待态时全部无效。
        for (id, act) in [
            (live_id!(dv_ok), DevAct::Confirm),
            (live_id!(dv_mismatch), DevAct::Mismatch),
            (live_id!(dv_far), DevAct::NotSamePlace),
            (live_id!(dv_stock), DevAct::NoStock),
            (live_id!(dv_expire), DevAct::Expire),
        ] {
            let path = [live_id!(page_settings), live_id!(dev_card), live_id!(dv_meet), id];
            if self.view.button(cx, &path).clicked(actions) {
                self.apply_dev_act(cx, act);
            }
        }
        // 发现页: 时间分段（今天 / 明天 / 本周）
        for (i, id) in SEG_DAYS.iter().enumerate() {
            let path = [
                live_id!(page_discover),
                live_id!(time_card),
                live_id!(seg_track),
                *id,
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.day_seg = i;
                // 今天 / 明天直接定到那一天；本周保留当前选择，靠日期条挑。
                if i < 2 {
                    self.day_sel = i;
                } else if self.day_sel < 2 {
                    self.day_sel = 2;
                }
                self.refresh_discover(cx);
            }
        }
        // 发现页: 一周日期条
        for (i, id) in DAY_CELLS.iter().enumerate() {
            let path = [
                live_id!(page_discover),
                live_id!(time_card),
                live_id!(day_strip),
                *id,
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.day_sel = i;
                self.day_seg = if i < 2 { i } else { 2 };
                self.refresh_discover(cx);
            }
        }
        // 发现页: 点排行里的一行 = 以这个片区为起点去发布
        for id in RANK_ROWS.iter().chain(MORE_ROWS.iter()) {
            if self
                .view
                .button(cx, &[*id, live_id!(ar_hit)])
                .clicked(actions)
            {
                let area = self.area_of_row(*id);
                self.open_wizard(cx, area);
            }
        }
        // 发现页: 发布 / 修改 / 撤回
        let open_new = self.view.button(cx, ids!(fab_layer.fab)).clicked(actions)
            || self
                .view
                .button(cx, ids!(page_discover.mine_card.mn_go))
                .clicked(actions);
        if open_new {
            self.open_wizard(cx, None);
        }
        if self
            .view
            .button(cx, ids!(page_discover.mine_card.mn_edit))
            .clicked(actions)
        {
            self.open_wizard(cx, None);
        }
        if self
            .view
            .button(cx, ids!(page_discover.mine_card.mn_withdraw))
            .clicked(actions)
        {
            self.state.withdraw();
            self.refresh_discover(cx);
        }
        // 发布向导: 退出（返回 / 放弃都丢草稿）
        if self
            .view
            .button(cx, ids!(page_publish.pw_top.pw_back))
            .clicked(actions)
            || self
                .view
                .button(cx, ids!(page_publish.pw_bar.pw_cancel))
                .clicked(actions)
        {
            self.close_wizard(cx);
        }
        // 发布向导: 第 1 步的日期 / 时段
        for (i, id) in PUB_DAY_CHIPS.iter().enumerate() {
            let path = [
                live_id!(page_publish),
                live_id!(pw_s1),
                live_id!(s1_card),
                live_id!(s1_days),
                *id,
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.draft.0 = i;
                self.refresh_publish(cx);
            }
        }
        for (i, id) in PUB_SLOT_CHIPS.iter().enumerate() {
            let path = [
                live_id!(page_publish),
                live_id!(pw_s1),
                live_id!(s1_card),
                live_id!(s1_slots),
                *id,
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.draft.1 = i;
                self.refresh_publish(cx);
            }
        }
        // 发布向导: 第 2 步的搜索与筛选
        if let Some(q) = self
            .view
            .text_input(cx, ids!(page_publish.pw_s2.s2_bar.s2_search))
            .changed(actions)
        {
            self.pick_query = q;
            self.refresh_publish(cx);
        }
        if self
            .view
            .button(cx, ids!(page_publish.pw_s2.s2_bar.s2_filter))
            .clicked(actions)
        {
            self.pick_filters = !self.pick_filters;
            self.refresh_publish(cx);
        }
        for (i, id) in KIND_CHIPS.iter().enumerate() {
            let path = [
                live_id!(page_publish),
                live_id!(pw_s2),
                live_id!(s2_filters),
                live_id!(s2_kinds),
                *id,
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.pick_kind = i;
                self.refresh_publish(cx);
            }
        }
        for (i, id) in DIST_CHIPS.iter().enumerate() {
            let path = [
                live_id!(page_publish),
                live_id!(pw_s2),
                live_id!(s2_filters),
                live_id!(s2_dists),
                *id,
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.pick_dist = i;
                self.refresh_publish(cx);
            }
        }
        // 发布向导: 选中一个片区就直接进第 3 步
        for id in PICK_ROWS.iter().chain(RECENT_ROWS.iter()) {
            if self
                .view
                .button(cx, &[*id, live_id!(ar_hit)])
                .clicked(actions)
            {
                if let Some(a) = self.area_of_row(*id) {
                    self.draft.2 = a;
                    self.goto_step(cx, 2);
                    self.refresh_publish(cx);
                    self.redraw(cx);
                }
            }
        }
        // 发布向导: 第 3 步的意愿
        for (i, id) in PUB_INTENT_CHIPS.iter().enumerate() {
            let path = [
                live_id!(page_publish),
                live_id!(pw_s3),
                live_id!(s3_card),
                live_id!(s3_intents),
                *id,
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.draft.3 = i;
                self.refresh_publish(cx);
            }
        }
        // 发布向导: 上一步 / 下一步 / 确认发布
        if self
            .view
            .button(cx, ids!(page_publish.pw_bar.pw_prev))
            .clicked(actions)
        {
            if let Some(step) = self.wizard {
                self.goto_step(cx, step.saturating_sub(1));
            }
        }
        if self
            .view
            .button(cx, ids!(page_publish.pw_bar.pw_next))
            .clicked(actions)
        {
            match self.wizard {
                Some(2) => {
                    self.state
                        .publish(self.draft.0, self.draft.1, self.draft.2, self.draft.3);
                    self.close_wizard(cx);
                }
                Some(step) => {
                    self.goto_step(cx, step + 1);
                }
                None => {}
            }
        }
        // 发现页: 回声（固定三选）
        for i in 0..ECHO_CHIPS.len() {
            let path = [live_id!(page_discover), live_id!(echo_card), live_id!(echo_row), ECHO_CHIPS[i]];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.state.set_echo(i);
                self.refresh_discover(cx);
            }
        }
        // 发现页: 我们碰到了 → 相遇页
        if self
            .view
            .button(cx, ids!(page_discover.met_card.met_go))
            .clicked(actions)
        {
            self.set_tab(cx, 1);
        }

        // 相遇页 ①: 选人
        for i in 0..MEET_ROWS.len() {
            let path = [
                live_id!(page_meet),
                live_id!(meet_pick),
                live_id!(pick_card),
                MEET_ROWS[i],
                live_id!(ps_hit),
            ];
            if self.view.button(cx, &path).clicked(actions) && i < self.state.contacts.len() {
                self.meet_sel = i;
                self.refresh_meet(cx);
            }
        }
        // 相遇页 ①: 本次不确认，只记一笔
        if self
            .view
            .button(cx, ids!(page_meet.meet_pick.pick_plain))
            .clicked(actions)
        {
            if let Some(c) = self.state.contacts.get(self.meet_sel) {
                let (id, label) = (c.id, c.label.clone());
                self.state.push_memory(id, &label, MemoryChoice::Save);
                self.view
                    .widget(cx, ids!(page_meet.meet_pick.pick_done))
                    .set_visible(cx, true);
                self.refresh_contacts(cx);
                self.refresh_memories(cx);
            }
        }
        // 相遇页 ①: 确认相遇 → 先过定位门槛，这时还没有会话。
        if self
            .view
            .button(cx, ids!(page_meet.meet_pick.pick_go))
            .clicked(actions)
        {
            if let Some(c) = self.state.contacts.get(self.meet_sel) {
                let seed = self.state.encounters.len() + self.state.contacts.len();
                let mut s = RecogSession::new(c.id, c.label.clone(), seed);
                s.choice = self.choice;
                self.session = Some(s);
                self.view
                    .widget(cx, ids!(page_meet.meet_pick.pick_done))
                    .set_visible(cx, false);
                self.refresh_meet(cx);
            }
        }
        // 相遇页 ②: 定位门槛。未授权就不建会话 —— 这是硬门槛。
        if self
            .view
            .button(cx, ids!(page_meet.meet_gate.gate_card.gt_row.gate_allow))
            .clicked(actions)
        {
            if let Some(s) = self.session.as_mut() {
                s.grant_location();
            }
            self.on_stage_changed(cx);
        }
        if self
            .view
            .button(cx, ids!(page_meet.meet_gate.gate_card.gt_row.gate_deny))
            .clicked(actions)
        {
            if let Some(s) = self.session.as_mut() {
                s.deny_location();
            }
            self.on_stage_changed(cx);
        }
        // 相遇页 ②: 拒绝定位后的出口 —— 只记一笔回忆。
        if self
            .view
            .button(cx, ids!(page_meet.meet_gate.gate_card.gt_alt.gate_plain))
            .clicked(actions)
        {
            if let Some(s) = self.session.as_mut() {
                s.choice = MemoryChoice::Save;
            }
            self.finish_meet(cx);
        }
        if self
            .view
            .button(cx, ids!(page_meet.meet_gate.gate_card.gate_cancel))
            .clicked(actions)
        {
            self.session = None;
            self.on_stage_changed(cx);
        }
        // 相遇页 ③: 对方没有偶遇？（折叠的短码与链接）
        if self
            .view
            .button(cx, ids!(page_meet.meet_wait.wait_card.wt_more))
            .clicked(actions)
        {
            self.code_open = !self.code_open;
            self.refresh_meet(cx);
        }
        if self
            .view
            .button(cx, ids!(page_meet.meet_wait.wait_card.wt_cancel))
            .clicked(actions)
        {
            self.session = None;
            self.on_stage_changed(cx);
        }
        // 相遇页 ④: 本次回忆怎么留
        for i in 0..CHOICE_CHIPS.len() {
            let path = [
                live_id!(page_meet),
                live_id!(meet_result),
                live_id!(choice_card),
                live_id!(choice_row),
                CHOICE_CHIPS[i],
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.choice = index_choice(i);
                if let Some(s) = self.session.as_mut() {
                    s.choice = self.choice;
                }
                self.refresh_meet(cx);
            }
        }
        // 相遇页 ④: 核销（成功即出券，没有「领取」这一步）
        if self
            .view
            .button(cx, ids!(page_meet.meet_result.coupon_card.ck_row.ck_redeem))
            .clicked(actions)
        {
            self.state.redeem_reward();
            self.refresh_meet(cx);
        }
        // 相遇页 ④: 再试一次 / 只记一笔 / 完成
        if self
            .view
            .button(cx, ids!(page_meet.meet_result.res_row.rs_retry))
            .clicked(actions)
        {
            if let Some(s) = self.session.as_mut() {
                if s.stage == RecogStage::Expired {
                    // 超时不算一次「信息不一致」，重新起表即可。
                    s.stage = RecogStage::Waiting;
                    s.elapsed = 0;
                } else {
                    s.retry();
                }
            }
            self.on_stage_changed(cx);
        }
        if self
            .view
            .button(cx, ids!(page_meet.meet_result.res_row.rs_plain))
            .clicked(actions)
        {
            if let Some(s) = self.session.as_mut() {
                s.choice = MemoryChoice::Save;
            }
            self.finish_meet(cx);
        }
        if self
            .view
            .button(cx, ids!(page_meet.meet_result.res_row.rs_finish))
            .clicked(actions)
        {
            self.finish_meet(cx);
        }

        // 熟人页: 手动添加
        if self
            .view
            .button(cx, ids!(page_contacts.ct_card.ct_add.ca_btn))
            .clicked(actions)
        {
            let input = self.view.text_input(cx, ids!(page_contacts.ct_card.ct_add.ca_input));
            let label = input.text();
            match self.state.add_contact(&label) {
                Ok(_) => {
                    self.add_error = None;
                    input.set_text(cx, "");
                    self.toast(cx, &format!("已添加「{}」", label.trim()));
                    self.refresh_contacts(cx);
                    self.refresh_meet(cx);
                }
                Err(e) => {
                    self.add_error = Some(e);
                    self.refresh_contacts(cx);
                }
            }
            self.redraw(cx);
        }
        // 熟人页: 退出合并模式
        if self
            .view
            .button(cx, ids!(page_contacts.ct_card.ct_merge_bar.cm_cancel))
            .clicked(actions)
        {
            self.merge_from = None;
            self.refresh_contacts(cx);
            self.redraw(cx);
        }

        // 熟人页: 行操作
        for i in 0..CONTACT_ROWS.len() {
            let base = [live_id!(page_contacts), live_id!(ct_card), CONTACT_ROWS[i]];
            let Some(row_id) = self.contact_order.get(i).copied() else {
                continue;
            };
            let main = [base[0], base[1], base[2], live_id!(c_main)];
            let confirm = [base[0], base[1], base[2], live_id!(c_confirm)];
            if self.view.button(cx, &[main[0], main[1], main[2], main[3], live_id!(c_view)]).clicked(actions) {
                if let Some(c) = self.state.contact(row_id) {
                    self.memory_filter = Some(c.label.clone());
                }
                self.set_tab(cx, 3);
            }
            // 合并重复：第一下选中要并走的那一位，第二下选目标。
            //
            // 不做「勾两个再点合并」，是因为这份名单最多六行，一步一确认
            // 比多选框好懂；方向也说死了 —— 被并走的那一位会消失。
            if self.view.button(cx, &[main[0], main[1], main[2], main[3], live_id!(c_merge)]).clicked(actions) {
                match self.merge_from {
                    None => self.merge_from = Some(row_id),
                    Some(f) if f == row_id => self.merge_from = None,
                    Some(f) => {
                        let from_label = self
                            .state
                            .contact(f)
                            .map(|c| c.label.clone())
                            .unwrap_or_default();
                        let into_label = self
                            .state
                            .contact(row_id)
                            .map(|c| c.label.clone())
                            .unwrap_or_default();
                        let n = self
                            .state
                            .encounters
                            .iter()
                            .filter(|e| e.contact_id == Some(f))
                            .count();
                        let snap = self.state.snapshot_for_undo(format!(
                            "已把「{}」并入「{}」，{} 条回忆一起转过去",
                            from_label, into_label, n
                        ));
                        if self.state.merge_contacts(f, row_id).is_some() {
                            self.merge_from = None;
                            self.offer_undo(cx, snap);
                            self.refresh_memories(cx);
                            self.refresh_meet(cx);
                            self.refresh_achievements(cx);
                        }
                    }
                }
                self.refresh_contacts(cx);
                self.redraw(cx);
            }
            // 删除某人的全部回忆：能撤销，就不必先问一遍。
            if self.view.button(cx, &[main[0], main[1], main[2], main[3], live_id!(c_delmem)]).clicked(actions) {
                let (id, label) = {
                    let Some(c) = self.state.contact(row_id) else { continue };
                    (c.id, c.label.clone())
                };
                let n = self
                    .state
                    .encounters
                    .iter()
                    .filter(|e| e.contact_id == Some(id))
                    .count();
                if n > 0 {
                    let snap = self.state.snapshot_for_undo(format!("已删除 {} 的 {} 条回忆", label, n));
                    self.state.delete_memories_of(id);
                    self.offer_undo(cx, snap);
                    self.refresh_contacts(cx);
                    self.refresh_memories(cx);
                    self.refresh_achievements(cx);
                }
            }
            // 删除联系人仍然先展开一行 —— 那一行不只是「确定吗」，它还带着
            // 「要不要连回忆一起删」这个真正的选择。
            if self.view.button(cx, &[main[0], main[1], main[2], main[3], live_id!(c_del)]).clicked(actions) {
                self.confirm_row = Some(i);
                self.confirm_also = false;
                self.refresh_contacts(cx);
            }
            if self.view.check_box(cx, &[confirm[0], confirm[1], confirm[2], confirm[3], live_id!(c_also)]).changed(actions).is_some() {
                self.confirm_also = !self.confirm_also;
                self.refresh_contacts(cx);
            }
            if self.view.button(cx, &[confirm[0], confirm[1], confirm[2], confirm[3], live_id!(c_yes)]).clicked(actions)
                && self.confirm_row == Some(i)
            {
                let (id, label) = {
                    let Some(c) = self.state.contact(row_id) else { continue };
                    (c.id, c.label.clone())
                };
                let snap = self.state.snapshot_for_undo(format!("已删除熟人 {}", label));
                self.state.delete_contact(id, self.confirm_also);
                self.offer_undo(cx, snap);
                if self.merge_from == Some(id) {
                    self.merge_from = None;
                }
                self.confirm_row = None;
                self.confirm_also = false;
                self.refresh_contacts(cx);
                self.refresh_memories(cx);
                self.refresh_meet(cx);
                self.refresh_achievements(cx);
            }
            if self.view.button(cx, &[confirm[0], confirm[1], confirm[2], confirm[3], live_id!(c_no)]).clicked(actions) {
                self.confirm_row = None;
                self.confirm_also = false;
                self.refresh_contacts(cx);
            }
        }
        // 熟人页: 导入 vCard（头部按钮与行内按钮同动作）
        if self
            .view
            .button(cx, ids!(page_contacts.ct_card.ct_head.ct_import))
            .clicked(actions)
            || self
                .view
                .button(cx, ids!(page_contacts.vcf_row.vcf_btn))
                .clicked(actions)
        {
            self.import_vcard(cx);
        }
        // 熟人页: 通讯录折叠
        if self
            .view
            .button(cx, ids!(page_contacts.dir_head.dir_btn))
            .clicked(actions)
        {
            let list = self.view.widget(cx, ids!(page_contacts.dir_list));
            let open = !list.visible();
            list.set_visible(cx, open);
            self.view
                .widget(cx, ids!(page_contacts.dir_head.dir_btn))
                .set_text(cx, if open { "收起" } else { "展开" });
        }

        // 回忆页: 筛选芯片
        if self
            .view
            .check_box(cx, ids!(page_memories.filt_row.filt_all))
            .changed(actions)
            .is_some()
        {
            self.memory_filter = None;
            self.refresh_memories(cx);
        }
        for i in 0..FILT_CHIPS.len() {
            let path = [live_id!(page_memories), live_id!(filt_row), FILT_CHIPS[i]];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                if let Some(label) = self.filter_options.get(i) {
                    self.memory_filter = Some(label.clone());
                }
                self.refresh_memories(cx);
            }
        }
        // 回忆页: 搜索
        if let Some(q) = self
            .view
            .text_input(cx, ids!(page_memories.mm_bar.mm_search))
            .changed(actions)
        {
            self.mem_query = q;
            self.refresh_memories(cx);
        }
        // 回忆页: 打开某一条的详情（可见段与已隐藏段是同一个按钮）
        for (rows, hidden_seg) in [(&MEM_ROWS, false), (&HID_ROWS, true)] {
            for i in 0..rows.len() {
                let base = [live_id!(page_memories), rows[i]];
                let ids = if hidden_seg { &self.hid_row_ids } else { &self.mem_row_ids };
                let Some(id) = ids.get(i).copied() else { continue };
                if self.view.button(cx, &[base[0], base[1], live_id!(m_open)]).clicked(actions) {
                    self.mem_detail = Some(id);
                    self.refresh_memory_detail(cx);
                    self.update_page_visibility(cx);
                    self.redraw(cx);
                }
            }
        }
        // 回忆详情: 返回
        if self.view.button(cx, ids!(page_memdetail.md_back)).clicked(actions) {
            self.mem_detail = None;
            self.update_page_visibility(cx);
            self.redraw(cx);
        }
        // 回忆详情: 保存备注
        if self
            .view
            .button(cx, ids!(page_memdetail.md_card.md_save))
            .clicked(actions)
        {
            if let Some(id) = self.mem_detail {
                let note = self
                    .view
                    .text_input(cx, ids!(page_memdetail.md_card.md_note))
                    .text();
                self.state.set_note(id, &note);
                self.toast(cx, "备注已保存");
                self.refresh_memories(cx);
            }
        }
        // 回忆详情: 隐藏 / 恢复
        if self
            .view
            .button(cx, ids!(page_memdetail.md_act.md_toggle))
            .clicked(actions)
        {
            if let Some(id) = self.mem_detail {
                let now = self
                    .state
                    .encounters
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.hidden)
                    .unwrap_or(false);
                self.state.set_hidden(id, !now);
                self.refresh_memories(cx);
                self.refresh_contacts(cx);
                self.refresh_achievements(cx);
                self.redraw(cx);
            }
        }
        // 回忆详情: 删除（5 秒可撤销，删完退回列表）
        if self
            .view
            .button(cx, ids!(page_memdetail.md_act.md_del))
            .clicked(actions)
        {
            if let Some(id) = self.mem_detail {
                let snap = self.state.snapshot_for_undo("已删除 1 条回忆");
                self.state.delete_encounter(id);
                self.mem_detail = None;
                self.offer_undo(cx, snap);
                self.update_page_visibility(cx);
                self.refresh_memories(cx);
                self.refresh_contacts(cx);
                self.refresh_achievements(cx);
                self.redraw(cx);
            }
        }

        // 成就页: 4 / 8 周切换
        for i in 0..WK_CHIPS.len() {
            let path = [
                live_id!(page_achieve),
                live_id!(curve_card),
                live_id!(curve_head),
                WK_CHIPS[i],
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.weeks = if i == 0 { 4 } else { 8 };
                self.set_chip_group(cx, &path[..3], &WK_CHIPS, i);
                self.refresh_achievements(cx);
            }
        }
        // 成就页: 查看 / 收起每周次数
        if self
            .view
            .button(cx, ids!(page_achieve.curve_card.wk_toggle))
            .clicked(actions)
        {
            self.weekly_open = !self.weekly_open;
            self.refresh_achievements(cx);
        }
        // 开场三屏：下一步 / 上一步 / 跳过。
        if self.view.button(cx, ids!(page_intro.in_bar.in_next)).clicked(actions) {
            let step = self.intro.unwrap_or(0);
            if step + 1 < INTRO.len() {
                self.intro = Some(step + 1);
                self.refresh_intro(cx);
            } else {
                self.close_intro(cx);
            }
            self.redraw(cx);
        }
        if self.view.button(cx, ids!(page_intro.in_bar.in_back)).clicked(actions) {
            self.intro = Some(self.intro.unwrap_or(0).saturating_sub(1));
            self.refresh_intro(cx);
            self.redraw(cx);
        }
        if self.view.button(cx, ids!(page_intro.in_top.in_skip)).clicked(actions) {
            self.close_intro(cx);
            self.redraw(cx);
        }
        // 关于卡：重看那三句话。
        if self
            .view
            .button(cx, ids!(page_settings.about_card.ab_intro))
            .clicked(actions)
        {
            self.open_intro(cx, 0);
            self.redraw(cx);
        }
        // 开发者选项：让两条通知现在就到点。
        //
        // 走的是和真实路径同一个 `due_notices`：先把开关打开、把时间挪到
        // 该响的那一刻，再问一次「现在该发什么」—— 而不是直接塞一句假文案。
        // 这样演示里看到的，就是真实逻辑会发的那一条。
        for (id, kind) in [
            (live_id!(dv_nt_pub), NoticeKind::PublishExpiring),
            (live_id!(dv_nt_rwd), NoticeKind::RewardExpiring),
        ] {
            let path = [
                live_id!(page_settings),
                live_id!(dev_card),
                live_id!(dv_notice),
                id,
            ];
            if !self.view.button(cx, &path).clicked(actions) {
                continue;
            }
            match self.demo_notice(kind) {
                Some(n) => self.show_notice(cx, n),
                None => self.toast(cx, match kind {
                    NoticeKind::PublishExpiring => "先发布一条今天的去向，再来按这个",
                    NoticeKind::RewardExpiring => "手里没有可用的券，先去相遇页领一张",
                }),
            }
        }
        // 通知条：知道了。
        if self
            .view
            .button(cx, ids!(notice_layer.nt_card.nt_close))
            .clicked(actions)
        {
            self.close_notice(cx);
        }
        // 「我」页: 三个入口
        if self
            .view
            .button(cx, ids!(page_achieve.hub_card.row_wallet.st_hit))
            .clicked(actions)
        {
            self.sheet = Some(Sheet::Wallet);
            self.refresh_wallet(cx);
            self.update_page_visibility(cx);
        }
        for id in [live_id!(row_settings), live_id!(row_about)] {
            let path = [live_id!(page_achieve), live_id!(hub_card), id, live_id!(st_hit)];
            if self.view.button(cx, &path).clicked(actions) {
                self.sheet = Some(Sheet::Settings);
                self.clear_armed = false;
                self.refresh_settings(cx);
                self.update_page_visibility(cx);
            }
        }
        // 覆盖页: 返回「我」
        if self.view.button(cx, ids!(page_wallet.wl_back)).clicked(actions)
            || self.view.button(cx, ids!(page_settings.se_back)).clicked(actions)
        {
            self.sheet = None;
            self.clear_armed = false;
            self.refresh_achievements(cx);
            self.update_page_visibility(cx);
        }
        // 我的券: 核销（只有「可用」区的券有这个按钮）
        for (sec, rows) in [
            (live_id!(wl_avail), WA_ROWS),
            (live_id!(wl_used), WU_ROWS),
            (live_id!(wl_gone), WG_ROWS),
        ] {
            for (i, id) in rows.iter().enumerate() {
                let path = [live_id!(page_wallet), sec, *id, live_id!(cw_row), live_id!(cw_redeem)];
                if self.view.button(cx, &path).clicked(actions) {
                    // 分区里的第 i 张，换算回 wallet 里的下标。
                    let today = today_days();
                    let st = match sec {
                        s if s == live_id!(wl_avail) => RewardState::Available,
                        s if s == live_id!(wl_used) => RewardState::Redeemed,
                        _ => RewardState::Expired,
                    };
                    let idx = self
                        .state
                        .wallet
                        .iter()
                        .enumerate()
                        .rev()
                        .filter(|(_, r)| r.state(today) == st)
                        .map(|(j, _)| j)
                        .nth(i);
                    if let Some(j) = idx {
                        self.state.redeem_at(j);
                    }
                    self.refresh_wallet(cx);
                    self.refresh_achievements(cx);
                    self.refresh_meet(cx);
                }
            }
        }
        // 设置: 定位权限。真实版本这里跳系统设置，草图里就地切换。
        if self
            .view
            .button(cx, ids!(page_settings.loc_card.row_loc.st_hit))
            .clicked(actions)
        {
            self.state.settings.location_granted = !self.state.settings.location_granted;
            self.state.save();
            self.refresh_settings(cx);
            self.refresh_meet(cx);
        }
        // 设置: 两个通知开关
        for (id, publish) in [
            (live_id!(row_ntf_publish), true),
            (live_id!(row_ntf_reward), false),
        ] {
            let path = [live_id!(page_settings), live_id!(ntf_card), id, live_id!(sw_hit)];
            if self.view.button(cx, &path).clicked(actions) {
                let f = if publish {
                    &mut self.state.settings.notify_publish
                } else {
                    &mut self.state.settings.notify_reward
                };
                *f = !*f;
                self.state.save();
                self.refresh_settings(cx);
            }
        }
        // 设置: 导出
        if self
            .view
            .button(cx, ids!(page_settings.data_card.row_export.st_hit))
            .clicked(actions)
        {
            self.export_note = match self.state.export_data() {
                Some(path) => Some(
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "已导出".to_string()),
                ),
                None => Some("导出失败".to_string()),
            };
            self.refresh_settings(cx);
        }
        // 设置: 清除本机数据。不可撤销，所以这一条保留二次确认。
        if self
            .view
            .button(cx, ids!(page_settings.data_card.row_clear.st_hit))
            .clicked(actions)
        {
            self.clear_armed = !self.clear_armed;
            self.refresh_settings(cx);
        }
        if self
            .view
            .button(cx, ids!(page_settings.data_card.clear_confirm.cf_row.cf_cancel))
            .clicked(actions)
        {
            self.clear_armed = false;
            self.refresh_settings(cx);
        }
        if self
            .view
            .button(cx, ids!(page_settings.data_card.clear_confirm.cf_row.cf_ok))
            .clicked(actions)
        {
            self.state.clear_local_data();
            self.clear_armed = false;
            self.session = None;
            self.sync_tick(cx);
            self.close_undo(cx, false);
            self.refresh_all(cx);
            self.refresh_achievements(cx);
        }
        // 开发者选项: 列表四态
        for (id, st) in [
            (live_id!(dv_ready), ListState::Ready),
            (live_id!(dv_loading), ListState::Loading),
            (live_id!(dv_failed), ListState::Failed),
            (live_id!(dv_offline), ListState::Offline),
        ] {
            let path = [live_id!(page_settings), live_id!(dev_card), live_id!(dv_state), id];
            if self.view.button(cx, &path).clicked(actions) {
                self.list_state = st;
                self.refresh_contacts(cx);
                self.refresh_memories(cx);
                self.refresh_wallet(cx);
            }
        }
        // 四态上的那个按钮：出错 / 离线时是「重试」，空态时是这一页自己的动作。
        if self
            .view
            .button(cx, ids!(page_memories.mem_empty.em_action))
            .clicked(actions)
        {
            self.list_state = ListState::Ready;
            self.refresh_memories(cx);
        }
        if self
            .view
            .button(cx, ids!(page_wallet.wl_state.em_action))
            .clicked(actions)
        {
            self.list_state = ListState::Ready;
            self.refresh_wallet(cx);
        }
        if self
            .view
            .button(cx, ids!(page_contacts.ct_card.ct_empty.em_action))
            .clicked(actions)
        {
            if self.list_state == ListState::Ready {
                self.import_vcard(cx);
            } else {
                self.list_state = ListState::Ready;
                self.refresh_contacts(cx);
            }
        }
        // 撤销条
        if self.view.button(cx, ids!(toast_layer.toast.to_undo)).clicked(actions) {
            self.close_undo(cx, true);
        }
        // 成就页: 生成分享卡 → 预览页
        if self
            .view
            .button(cx, ids!(page_achieve.share_card.sh_go))
            .clicked(actions)
        {
            self.share_open = true;
            self.update_page_visibility(cx);
            self.refresh_share(cx);
        }
        // 分享页: 样式 chips
        for i in 0..STYLE_CHIPS.len() {
            let path = [
                live_id!(page_share),
                live_id!(sp_row),
                live_id!(sp_right),
                live_id!(sp_style),
                live_id!(sps_row),
                STYLE_CHIPS[i],
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.share_style = if i == 0 {
                    ShareStyle::Warm
                } else {
                    ShareStyle::Night
                };
                self.refresh_share(cx);
            }
        }
        // 分享页: 包含每周曲线开关
        if self
            .view
            .check_box(cx, ids!(page_share.sp_row.sp_right.sp_what.sh_curve))
            .changed(actions)
            .is_some()
        {
            self.share_curve = !self.share_curve;
            self.refresh_share(cx);
        }
        // 分享页: 保存分享图片（与预览分开的动作；不代发不上传）
        if self
            .view
            .button(cx, ids!(page_share.sp_row.sp_left.sp_btns.sh_save))
            .clicked(actions)
        {
            let status = self.view.widget(cx, ids!(page_share.sp_row.sp_left.sh_saved));
            let svg = share::render_svg(&self.share_scene());
            match share::save_share_card(&svg) {
                Ok(Some(path)) => {
                    status.set_visible(cx, true);
                    status.set_text(cx, &format!("已保存到 {}", path.display()));
                }
                Ok(None) => {
                    status.set_visible(cx, true);
                    status.set_text(cx, "未设置 MAKEPAD_HOME，无法保存");
                }
                Err(e) => {
                    status.set_visible(cx, true);
                    status.set_text(cx, &format!("保存失败：{e}"));
                }
            }
        }
        // 分享页: 返回个人成就
        if self
            .view
            .button(cx, ids!(page_share.sp_row.sp_left.sp_btns.sh_back))
            .clicked(actions)
        {
            self.share_open = false;
            self.update_page_visibility(cx);
            self.refresh_achievements(cx);
        }
    }
}

impl Widget for OuyuView {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let full = cx.peek_walk_turtle(walk);
        // 以本 tile 实际拿到的尺寸（而非窗口尺寸）决定手机 / 平板 / 桌面形态。
        if full.size.x > 1.0 && full.size.y > 1.0 {
            self.update_responsive(cx, full.size);
        }
        let ret = self.view.draw_walk(cx, scope, walk);
        if self.fade_alpha > 0.001 && full.size.y > 0.0 {
            self.draw_fade.color.w = self.fade_alpha;
            self.draw_fade.draw_abs(cx, full);
        }
        ret
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        if !self.initialized {
            // 演示便利: 状态目录缺 contacts.vcf 时补一份示例, 让导入按钮开箱可点。
            OuyuState::ensure_sample_vcard();
            // 片区 id 不是下标，0 号片区并不存在。
            self.draft = (0, 1, areas::AREAS[0].id, 0);
            self.weeks = 8;
            self.share_style = ShareStyle::Warm;
            self.refresh_all(cx);
            // 第一次打开先讲三句话。跳过也记 onboarded，不会每次都拦。
            if !self.state.settings.onboarded {
                self.open_intro(cx, 0);
            }
            self.notice_poll = cx.start_interval(60.0);
            self.initialized = true;
        }
        // Tab 淡入: 150ms 内 alpha 从 1 衰减到 0。
        if let Some(nf) = self.next_frame.is_event(event) {
            const FADE_SECS: f64 = 0.15;
            match self.fade_start {
                None => {
                    self.fade_start = Some(nf.time);
                    self.next_frame = cx.new_next_frame();
                }
                Some(t0) => {
                    let t = (nf.time - t0) / FADE_SECS;
                    if t >= 1.0 {
                        self.fade_alpha = 0.0;
                        self.fade_start = None;
                    } else {
                        self.fade_alpha = (1.0 - t) as f32;
                        self.next_frame = cx.new_next_frame();
                    }
                }
            }
            self.redraw(cx);
        }
        // 等待态的倒计时。归零即进结果屏（超时）。
        if self.tick.is_event(event).is_some() {
            let changed = self.session.as_mut().map(|s| s.tick()).unwrap_or(false);
            if changed {
                if self.session.as_ref().map(|s| s.stage) == Some(RecogStage::Expired) {
                    self.on_stage_changed(cx);
                } else {
                    self.refresh_meet(cx);
                }
                self.redraw(cx);
            }
        }
        // 通知：借等待态那一拍顺带看一眼。没有会话时由 poll 计时器来敲。
        if self.notice_poll.is_event(event).is_some() {
            self.poll_notices(cx);
        }
        // 撤销窗口到点：快照作废，toast 收起来。
        if self.undo_timer.is_event(event).is_some() {
            self.close_undo(cx, false);
            self.redraw(cx);
        }
        if let Event::Actions(actions) = event {
            self.handle_actions(cx, actions);
        }
    }
}

/// 把一条 id 路径接上一个子 id。
///
/// makepad 的查找本来就是「往下找同名后代」，所以这里只是为了少写一串
/// `[base[0], base[1], ...]` —— 那种写法多一层就得全文改一遍。
fn join(base: &[LiveId], id: LiveId) -> Vec<LiveId> {
    let mut v = Vec::with_capacity(base.len() + 1);
    v.extend_from_slice(base);
    v.push(id);
    v
}

fn choice_index(c: MemoryChoice) -> usize {
    match c {
        MemoryChoice::Save => 0,
        MemoryChoice::Hidden => 1,
        MemoryChoice::Skip => 2,
    }
}

fn index_choice(i: usize) -> MemoryChoice {
    match i {
        1 => MemoryChoice::Hidden,
        2 => MemoryChoice::Skip,
        _ => MemoryChoice::Save,
    }
}

/// 回忆行文案：日期在左侧单独一列，这里是正文。
fn encounter_text(e: &EncounterLocal) -> String {
    if e.note.is_empty() {
        format!("与 {} 重逢", e.label_snapshot)
    } else {
        format!("与 {} 重逢\n「{}」", e.label_snapshot, e.note)
    }
}

pub struct OuyuModule;
pub static OUYU_MODULE: OuyuModule = OuyuModule;

impl AppModule for OuyuModule {
    fn id(&self) -> &'static str {
        "ouyu"
    }
    fn label(&self) -> &'static str {
        "偶遇 OuYu"
    }
    fn register(&self, vm: &mut ScriptVm) {
        canvas::script_mod(vm);
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
            OuyuView {}
        });
        let root = WidgetRef::script_from_value(vm, value);
        InstanceParts {
            root: root.clone(),
            executor: Box::new(OuyuExecutor { root }),
            shutdown: Box::new(|_| {}),
        }
    }
}

/// 模块形态的 executor：借用视图取匿名快照再应答（02 H 节只注册两个 Read 工具）。
struct OuyuExecutor {
    root: WidgetRef,
}
impl ServiceExecutor for OuyuExecutor {
    fn manifest(&self) -> ServiceManifest {
        ai::manifest()
    }
    fn execute(&mut self, _cx: &mut Cx, call: &ServiceCall) -> ExecOutcome {
        let result = self
            .root
            .borrow::<OuyuView>()
            .map(|view| view.ai_answer(call))
            .unwrap_or_else(|| {
                makepad_app_module::makepad_ai_services::wire::ToolResult::unavailable(
                    &call.call_id,
                    "偶遇窗口已关闭",
                )
            });
        ExecOutcome::Done(result)
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    fn size(x: f64, y: f64) -> Vec2d {
        Vec2d { x, y }
    }

    #[test]
    fn auto_follows_the_surface_it_is_actually_given() {
        // 宿主手机壳给的就是 412×892 的一整块面; 桌面 tile 才够放右列。
        assert_eq!(shaping_for(UiMode::Auto, size(412.0, 892.0)).shape, Shape::Phone);
        assert_eq!(shaping_for(UiMode::Auto, size(820.0, 700.0)).shape, Shape::Tablet);
        assert_eq!(shaping_for(UiMode::Auto, size(1280.0, 800.0)).shape, Shape::Desktop);
        // 自动模式永远铺满，不画手机框。
        assert!(!shaping_for(UiMode::Auto, size(1280.0, 800.0)).framed);
    }

    #[test]
    fn the_aside_column_only_appears_when_the_desktop_shape_has_room() {
        assert!(shaping_for(UiMode::Auto, size(1280.0, 800.0)).aside);
        assert!(!shaping_for(UiMode::Auto, size(820.0, 700.0)).aside);
        assert!(!shaping_for(UiMode::Auto, size(412.0, 892.0)).aside);
    }

    #[test]
    fn forcing_phone_mode_frames_a_preview_on_a_wide_surface_only() {
        let wide = shaping_for(UiMode::Phone, size(1280.0, 800.0));
        assert_eq!(wide.shape, Shape::Phone);
        assert!(wide.framed, "宽窗口上手机模式收进居中的手机框");
        assert_eq!(wide.frame.0, FRAME_W);
        assert!(wide.pad.0 > 0.0);
        // 本来就只有一条手机宽，再套框只会更窄——铺满即可。
        let narrow = shaping_for(UiMode::Phone, size(412.0, 892.0));
        assert!(!narrow.framed);
        assert_eq!(narrow.pad, (0.0, 0.0));
    }

    #[test]
    fn wide_mode_never_asks_for_more_than_the_surface_can_hold() {
        // 手机壳里选「大屏」最多到平板，锁不出一个放不下的桌面布局。
        assert_eq!(shaping_for(UiMode::Wide, size(412.0, 892.0)).shape, Shape::Tablet);
        assert_eq!(shaping_for(UiMode::Wide, size(1280.0, 800.0)).shape, Shape::Desktop);
    }

    #[test]
    fn a_short_surface_is_flagged_so_the_illustrations_shrink() {
        assert!(shaping_for(UiMode::Auto, size(1400.0, 440.0)).short);
        assert!(!shaping_for(UiMode::Auto, size(1400.0, 800.0)).short);
    }

    #[test]
    fn the_mode_button_cycles_auto_phone_wide() {
        assert_eq!(UiMode::Auto.next(), UiMode::Phone);
        assert_eq!(UiMode::Phone.next(), UiMode::Wide);
        assert_eq!(UiMode::Wide.next(), UiMode::Auto);
    }
}
