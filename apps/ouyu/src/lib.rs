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
pub mod theme;

use canvas::{ChartData, OuyuChart, OuyuShareCard};
use theme::{Pal, ThemeMode};
use data::*;
use share::ShareStyle;

script_mod! {
    use mod.prelude.ouyu.*
    use mod.widgets.*

    mod.widgets.OuyuView = set_type_default() do #(OuyuView::register_widget(vm)) {
        ..mod.widgets.RectView
        width: Fill height: Fill
        show_bg: true
        draw_bg.color: ouyu.bg
        flow: Right

        // Tab 切换淡入: 用应用底色从不透明到透明扫过页面区。
        draw_fade +: { color: ouyu.bg draw_depth: 6.0 }

        // ---- 左侧 208px 侧栏 ----
        sidebar := RoundedView {
            width: 208 height: Fill
            flow: Down
            padding: Inset{left: 12.0, right: 12.0, top: 20.0, bottom: 14.0}
            spacing: 6.0
            draw_bg +: {
                color: ouyu.bg_chrome
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
                        color: ouyu.ink
                        text_style +: { font_size: 18.0 }
                    }
                }
                slogan := Label {
                    text: "给生活留一点偶然"
                    draw_text +: {
                        color: ouyu.ink_2
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
        }

        // ---- 内容壳：手机模式的顶栏 / 底部导航挂在这里 ----
        shell := RoundedView {
            width: Fill height: Fill
            flow: Down
            draw_bg +: {
                color: ouyu.bg
                border_color: ouyu.line_soft
                border_size: 0.0
                border_radius: 0.0
            }

            // ---- 顶栏：当页标题 + 当页主动作 ----
            //
            // 右边那个按钮跟着 Tab 走：发现页是「发布行踪」，相遇页是
            // 「确认相遇」，别的页没有主动作就收起来。它是这两件事唯一的入口，
            // 所以宽屏也留着这条顶栏，不再只给手机。
            // 标题在所有页面上都居中：标题单独一层铺满整条顶栏居中对齐，
            // 主动作按钮叠在上面靠右，按钮在不在都不会把标题挤偏。
            // 两层同高、各自居中，标题和按钮就落在同一条中线上。
            topbar := View {
                width: Fill height: 58
                flow: Overlay
                // 竖向居中靠这层 View，不靠标签自己的 align：DrawText 的
                // layout 只吃 align.x，align.y 一路被丢掉，所以 height: Fill
                // 的标签会把字画在框顶上——跟右边 36 高的按钮差半行。
                tb_mid := View {
                    width: Fill height: Fill
                    align: Align{x: 0.5, y: 0.5}
                    tb_title := Label {
                        width: Fit height: Fit
                        text: "偶遇 OuYu"
                        draw_text +: {
                            color: ouyu.warm
                            text_style +: { font_size: 15.0 }
                        }
                    }
                }
                tb_bar := View {
                    width: Fill height: Fill
                    flow: Right
                    align: Align{x: 1.0, y: 0.5}
                    padding: Inset{left: 18.0, right: 12.0}
                    spacing: 8.0
                    tb_action := OuyuBtnPrimarySm { visible: false width: Fit text: "发布行踪" }
                    // 熟人页的「+」：导入 / 新建的入口，跟标题同一条线，
                    // 别的页收起来。
                    tb_add := OuyuAddBtn {
                        visible: false
                        draw_icon +: { svg: crate_resource("self:resources/icons/plus.svg") }
                    }
                }
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

                    // ---- 发现页（首页：什么时候出门 + 匿名机会）----
                    page_discover := OuyuScrollY {
                        width: Fill height: Fill
                        flow: Down
                        spacing: 16.0

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
                            }
                            rk_note := OuyuMuted {
                                visible: false
                                width: Fill
                                margin: Inset{left: 12.0, right: 12.0, bottom: 2.0}
                                text: ""
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
                                    draw_icon +: { svg: crate_resource("self:resources/icons/nav-discover.svg") color: ouyu.ink_ghost }
                                }
                                em_text := Label {
                                    width: Fit
                                    text: "这一天还没有足够的机会"
                                    draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 13.0 } }
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
                    }

                    // ---- 发布向导（三步：什么时候 / 哪一带 / 想做什么）----
                    //
                    // 发布是一个动作，不是首页上常驻的表单：从顶栏的「发布行踪」
                    // 进来，退出即丢草稿（docs/02 第二节）。行踪可以有多条，
                    // 修改某一条从「我 → 我的行踪」进来。
                    // 底栏不进滚动区：第 2 步的片区列表有十几行，按钮若跟着内容
                    // 走，人得先滚一屏才能点「下一步」。
                    page_publish := View {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 12.0

                        pw_scroll := OuyuScrollY {
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
                            pw_title := OuyuH1 { width: Fill text: "写一下你的行踪" }
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
                                    draw_icon +: { svg: crate_resource("self:resources/icons/search.svg") color: ouyu.ink_ghost }
                                }
                                em_text := Label {
                                    width: Fit
                                    text: "没有匹配的片区"
                                    draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 13.0 } }
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

                    // ---- 相遇页（首屏成就 + 现场互认）----
                    //
                    // 首屏是曲线 / 里程碑 / 分享卡；点顶栏「确认相遇」后进入互认，
                    // 四个用户可见的态，一次只露一个：① 选人 → ② 定位门槛
                    // → ③ 等待 → ④ 结果。六条结局（成功 / 同地不成立 / 无库存
                    // / 超时 / 信息不一致 / 未授权）都停在同一张结果屏上。
                    page_meet := OuyuScrollY {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 16.0

                        mp_title := Label {
                            width: Fill
                            text: "这次，真的遇见了。"
                            draw_text +: {
                                wrap: Words
                                color: ouyu.ink
                                text_style +: { font_size: 24.0 line_spacing: 1.35 }
                            }
                        }
                        mp_sub := Label {
                            width: Fill
                            text: "先在线下认出彼此，再各自确认。"
                            draw_text +: {
                                wrap: Words
                                color: ouyu.ink_2
                                text_style +: { font_size: 14.0 line_spacing: 1.35 }
                            }
                        }

                        // ---- 首屏：成就内容（曲线 / 里程碑 / 分享卡）----
                        //
                        // 相遇页平时就停在这里。互认流程从顶栏「确认相遇」进，
                        // 走完或退出再回到这一屏。
                        meet_home := View {
                            width: Fill height: Fit
                            flow: Down
                            spacing: 14.0
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
                                            color: ouyu.ink
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
                                        color: ouyu.ink_2
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
                                        color: ouyu.warm
                                        text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                    }
                                }
                                curve_caption := Label {
                                    width: Fill
                                    text: "按周汇总，本周还没过完。"
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink_4
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
                                            draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 12.5 } }
                                        }
                                        wk_c := Label {
                                            text: ""
                                            draw_text +: { color: ouyu.warm text_style +: { font_size: 12.5 } }
                                        }
                                    }
                                    wk_r1 := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        wk_d := Label {
                                            width: Fill
                                            text: ""
                                            draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 12.5 } }
                                        }
                                        wk_c := Label {
                                            text: ""
                                            draw_text +: { color: ouyu.warm text_style +: { font_size: 12.5 } }
                                        }
                                    }
                                    wk_r2 := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        wk_d := Label {
                                            width: Fill
                                            text: ""
                                            draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 12.5 } }
                                        }
                                        wk_c := Label {
                                            text: ""
                                            draw_text +: { color: ouyu.warm text_style +: { font_size: 12.5 } }
                                        }
                                    }
                                    wk_r3 := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        wk_d := Label {
                                            width: Fill
                                            text: ""
                                            draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 12.5 } }
                                        }
                                        wk_c := Label {
                                            text: ""
                                            draw_text +: { color: ouyu.warm text_style +: { font_size: 12.5 } }
                                        }
                                    }
                                    wk_r4 := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        wk_d := Label {
                                            width: Fill
                                            text: ""
                                            draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 12.5 } }
                                        }
                                        wk_c := Label {
                                            text: ""
                                            draw_text +: { color: ouyu.warm text_style +: { font_size: 12.5 } }
                                        }
                                    }
                                    wk_r5 := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        wk_d := Label {
                                            width: Fill
                                            text: ""
                                            draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 12.5 } }
                                        }
                                        wk_c := Label {
                                            text: ""
                                            draw_text +: { color: ouyu.warm text_style +: { font_size: 12.5 } }
                                        }
                                    }
                                    wk_r6 := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        wk_d := Label {
                                            width: Fill
                                            text: ""
                                            draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 12.5 } }
                                        }
                                        wk_c := Label {
                                            text: ""
                                            draw_text +: { color: ouyu.warm text_style +: { font_size: 12.5 } }
                                        }
                                    }
                                    wk_r7 := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        wk_d := Label {
                                            width: Fill
                                            text: ""
                                            draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 12.5 } }
                                        }
                                        wk_c := Label {
                                            text: ""
                                            draw_text +: { color: ouyu.warm text_style +: { font_size: 12.5 } }
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
                                            color: ouyu.ink
                                            text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                        }
                                    }
                                    ms_badge := Label {
                                        text: "不用凑次数"
                                        draw_text +: {
                                            color: ouyu.warm
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
                                        ms_top := View {
                                            width: Fill height: Fit
                                            flow: Right
                                            align: Align{x: 0.0, y: 0.5}
                                            ms_icon := Label {
                                                width: Fill
                                                text: "☆"
                                                draw_text +: {
                                                    color: ouyu.warm
                                                    text_style +: { font_size: 17.0 }
                                                }
                                            }
                                            ms_state := Label {
                                                text: "等自然发生"
                                                draw_text +: {
                                                    color: ouyu.ink_4
                                                    text_style +: { font_size: 12.5 }
                                                }
                                            }
                                        }
                                        ms_title := Label {
                                            text: "第一次刚刚好"
                                            draw_text +: {
                                                color: ouyu.ink
                                                text_style +: { font_size: 13.0 }
                                            }
                                        }
                                        ms_desc := Label {
                                            text: "记住一次重逢"
                                            draw_text +: {
                                                color: ouyu.ink_2
                                                text_style +: { font_size: 12.5 }
                                            }
                                        }
                                    }
                                    ms1 := OuyuCard {
                                        width: Fill height: Fit
                                        flow: Down
                                        padding: 14.0
                                        spacing: 6.0
                                        ms_top := View {
                                            width: Fill height: Fit
                                            flow: Right
                                            align: Align{x: 0.0, y: 0.5}
                                            ms_icon := Label {
                                                width: Fill
                                                text: "☆"
                                                draw_text +: {
                                                    color: ouyu.warm
                                                    text_style +: { font_size: 17.0 }
                                                }
                                            }
                                            ms_state := Label {
                                                text: "等自然发生"
                                                draw_text +: {
                                                    color: ouyu.ink_4
                                                    text_style +: { font_size: 12.5 }
                                                }
                                            }
                                        }
                                        ms_title := Label {
                                            text: "生活有回响"
                                            draw_text +: {
                                                color: ouyu.ink
                                                text_style +: { font_size: 13.0 }
                                            }
                                        }
                                        ms_desc := Label {
                                            text: "记住三次相遇"
                                            draw_text +: {
                                                color: ouyu.ink_2
                                                text_style +: { font_size: 12.5 }
                                            }
                                        }
                                    }
                                    ms2 := OuyuCard {
                                        width: Fill height: Fit
                                        flow: Down
                                        padding: 14.0
                                        spacing: 6.0
                                        ms_top := View {
                                            width: Fill height: Fit
                                            flow: Right
                                            align: Align{x: 0.0, y: 0.5}
                                            ms_icon := Label {
                                                width: Fill
                                                text: "☆"
                                                draw_text +: {
                                                    color: ouyu.warm
                                                    text_style +: { font_size: 17.0 }
                                                }
                                            }
                                            ms_state := Label {
                                                text: "等自然发生"
                                                draw_text +: {
                                                    color: ouyu.ink_4
                                                    text_style +: { font_size: 12.5 }
                                                }
                                            }
                                        }
                                        ms_title := Label {
                                            text: "把日常过成故事"
                                            draw_text +: {
                                                color: ouyu.ink
                                                text_style +: { font_size: 13.0 }
                                            }
                                        }
                                        ms_desc := Label {
                                            text: "七个有相遇的日子"
                                            draw_text +: {
                                                color: ouyu.ink_2
                                                text_style +: { font_size: 12.5 }
                                            }
                                        }
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
                                            color: ouyu.ink
                                            text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                        }
                                    }
                                    share_b := Label {
                                        width: Fill
                                        text: "分享卡只有你的汇总，没有别人的身份。"
                                        draw_text +: {
                                            wrap: Words
                                            color: ouyu.ink_2
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                }
                                sh_go := OuyuBtnPrimary { text: "生成分享卡" draw_icon +: { svg: crate_resource("self:resources/icons/share.svg") } }
                            }
                        }

                        // ---- ① 选人 ----
                        meet_pick := View {
                            visible: false
                            width: Fill height: Fit
                            flow: Down
                            spacing: 10.0
                            pick_back := OuyuLink {
                                width: Fit
                                text: "返回"
                                draw_icon +: { svg: crate_resource("self:resources/icons/chevron-left.svg") }
                            }
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
                                        color: ouyu.ink_2
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
                                        color: ouyu.ink_2
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
                                    color: ouyu.good
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
                                        color: ouyu.ink
                                        text_style +: { font_size: 18.0 line_spacing: 1.35 }
                                    }
                                }
                                gt_why := Label {
                                    width: Fill
                                    text: "相遇礼只发给真的在同一处碰上的两个人。"
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink_2
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                gt_how := Label {
                                    width: Fill
                                    text: "只在你点确认的那一刻读一次。"
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink_2
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                gt_where := Label {
                                    width: Fill
                                    text: "只用来比对是否同地，比对完即丢弃，不上传不留存。"
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink_2
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
                                        color: ouyu.warm
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
                                        color: ouyu.ink
                                        text_style +: { font_size: 17.0 line_spacing: 1.35 }
                                    }
                                }
                                wt_count := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.warm
                                        text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                    }
                                }
                                wt_note := Label {
                                    width: Fill
                                    text: "没有已读和在线状态。"
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink_2
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
                                            color: ouyu.ink_2
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    wf_code := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: {
                                            wrap: Words
                                            color: ouyu.ink
                                            text_style +: { font_size: 26.0 line_spacing: 1.35 }
                                        }
                                    }
                                    wf_url := Label {
                                        width: Fill
                                        text: ""
                                        draw_text +: {
                                            wrap: Words
                                            color: ouyu.blue
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
                                            draw_icon +: { svg: crate_resource("self:resources/icons/check-circle.svg") color: ouyu.good }
                                        }
                                    }
                                    rs_no := View {
                                        visible: false
                                        width: Fit height: Fit
                                        rs_no_icon := OuyuIcon {
                                            icon_walk: Walk{ width: 40.0 height: Fit }
                                            draw_icon +: { svg: crate_resource("self:resources/icons/alert-circle.svg") color: ouyu.warm }
                                        }
                                    }
                                }
                                rs_title := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink
                                        text_style +: { font_size: 18.0 line_spacing: 1.35 }
                                    }
                                }
                                rs_sub := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink_2
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
                                    color: ouyu.coupon
                                    border_radius: r.card
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
                                            color: ouyu.on_warm
                                            text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                        }
                                    }
                                    ck_badge := Label {
                                        text: "相遇礼"
                                        draw_text +: {
                                            color: ouyu.on_warm
                                            text_style +: { font_size: 11.0 }
                                        }
                                    }
                                }
                                ck_offer := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.on_warm_hi
                                        text_style +: { font_size: 24.0 line_spacing: 1.35 }
                                    }
                                }
                                ck_terms := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.on_warm
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
                                            color: ouyu.on_warm_hi
                                            text_style +: { font_size: 13.0 }
                                        }
                                    }
                                    ck_expiry := Label {
                                        text: ""
                                        draw_text +: {
                                            color: ouyu.on_warm
                                            text_style +: { font_size: 13.0 }
                                        }
                                    }
                                }
                                ck_status := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.on_warm_hi
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
                                        color: ouyu.ink_2
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
                                        color: ouyu.ink_3
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
                                        color: ouyu.ink_2
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
                                        color: ouyu.ink_2
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
                    page_contacts := OuyuScrollY {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0
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
                                    text: "我的熟人 · 3 位"
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink
                                        text_style +: { font_size: 18.0 line_spacing: 1.35 }
                                    }
                                }
                            }
                            // 「+」点开的导入菜单：就地展开在头部下面，
                            // 选完一条自动收起。
                            ct_menu_row := View {
                                visible: false
                                width: Fill height: Fit
                                flow: Right
                                align: Align{x: 1.0, y: 0.0}
                                ct_menu := OuyuMenu {
                                    im_local := OuyuMenuItem {
                                        text: "从本机导入"
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-contacts.svg") }
                                    }
                                    im_file := OuyuMenuItem {
                                        text: "从文件导入"
                                        draw_icon +: { svg: crate_resource("self:resources/icons/download.svg") }
                                    }
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
                                    color: ouyu.bad
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
                                        color: ouyu.warm
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
                                // 一个熟人两行封顶：名字和次数一行，四个动作
                                // 一行。宽屏上 c_main 仍是一条 Right，两半并排
                                // 成一行；手机上 apply_shaping 把它翻成 Down，
                                // 于是恰好两行 —— 而不是让四个按钮自己乱换行。
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    c_top := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 8.0
                                        c_name := Label {
                                            width: Fit
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink
                                                text_style +: { font_size: 14.0 }
                                            }
                                        }
                                        c_count := Label {
                                            width: Fill
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink_2
                                                text_style +: { font_size: 12.5 }
                                            }
                                        }
                                    }
                                    c_acts := View {
                                        width: Fit height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 6.0
                                        c_view := OuyuBtnSm { text: "回忆" }
                                        c_merge := OuyuBtnSm { text: "合并" }
                                        c_delmem := OuyuBtnSm { text: "清空回忆" }
                                        c_del := OuyuBtnDangerSm { text: "删除" }
                                    }
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
                                            color: ouyu.bad
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
                                // 一个熟人两行封顶：名字和次数一行，四个动作
                                // 一行。宽屏上 c_main 仍是一条 Right，两半并排
                                // 成一行；手机上 apply_shaping 把它翻成 Down，
                                // 于是恰好两行 —— 而不是让四个按钮自己乱换行。
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    c_top := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 8.0
                                        c_name := Label {
                                            width: Fit
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink
                                                text_style +: { font_size: 14.0 }
                                            }
                                        }
                                        c_count := Label {
                                            width: Fill
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink_2
                                                text_style +: { font_size: 12.5 }
                                            }
                                        }
                                    }
                                    c_acts := View {
                                        width: Fit height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 6.0
                                        c_view := OuyuBtnSm { text: "回忆" }
                                        c_merge := OuyuBtnSm { text: "合并" }
                                        c_delmem := OuyuBtnSm { text: "清空回忆" }
                                        c_del := OuyuBtnDangerSm { text: "删除" }
                                    }
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
                                            color: ouyu.bad
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
                                // 一个熟人两行封顶：名字和次数一行，四个动作
                                // 一行。宽屏上 c_main 仍是一条 Right，两半并排
                                // 成一行；手机上 apply_shaping 把它翻成 Down，
                                // 于是恰好两行 —— 而不是让四个按钮自己乱换行。
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    c_top := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 8.0
                                        c_name := Label {
                                            width: Fit
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink
                                                text_style +: { font_size: 14.0 }
                                            }
                                        }
                                        c_count := Label {
                                            width: Fill
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink_2
                                                text_style +: { font_size: 12.5 }
                                            }
                                        }
                                    }
                                    c_acts := View {
                                        width: Fit height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 6.0
                                        c_view := OuyuBtnSm { text: "回忆" }
                                        c_merge := OuyuBtnSm { text: "合并" }
                                        c_delmem := OuyuBtnSm { text: "清空回忆" }
                                        c_del := OuyuBtnDangerSm { text: "删除" }
                                    }
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
                                            color: ouyu.bad
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
                                // 一个熟人两行封顶：名字和次数一行，四个动作
                                // 一行。宽屏上 c_main 仍是一条 Right，两半并排
                                // 成一行；手机上 apply_shaping 把它翻成 Down，
                                // 于是恰好两行 —— 而不是让四个按钮自己乱换行。
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    c_top := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 8.0
                                        c_name := Label {
                                            width: Fit
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink
                                                text_style +: { font_size: 14.0 }
                                            }
                                        }
                                        c_count := Label {
                                            width: Fill
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink_2
                                                text_style +: { font_size: 12.5 }
                                            }
                                        }
                                    }
                                    c_acts := View {
                                        width: Fit height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 6.0
                                        c_view := OuyuBtnSm { text: "回忆" }
                                        c_merge := OuyuBtnSm { text: "合并" }
                                        c_delmem := OuyuBtnSm { text: "清空回忆" }
                                        c_del := OuyuBtnDangerSm { text: "删除" }
                                    }
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
                                            color: ouyu.bad
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
                                // 一个熟人两行封顶：名字和次数一行，四个动作
                                // 一行。宽屏上 c_main 仍是一条 Right，两半并排
                                // 成一行；手机上 apply_shaping 把它翻成 Down，
                                // 于是恰好两行 —— 而不是让四个按钮自己乱换行。
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    c_top := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 8.0
                                        c_name := Label {
                                            width: Fit
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink
                                                text_style +: { font_size: 14.0 }
                                            }
                                        }
                                        c_count := Label {
                                            width: Fill
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink_2
                                                text_style +: { font_size: 12.5 }
                                            }
                                        }
                                    }
                                    c_acts := View {
                                        width: Fit height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 6.0
                                        c_view := OuyuBtnSm { text: "回忆" }
                                        c_merge := OuyuBtnSm { text: "合并" }
                                        c_delmem := OuyuBtnSm { text: "清空回忆" }
                                        c_del := OuyuBtnDangerSm { text: "删除" }
                                    }
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
                                            color: ouyu.bad
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
                                // 一个熟人两行封顶：名字和次数一行，四个动作
                                // 一行。宽屏上 c_main 仍是一条 Right，两半并排
                                // 成一行；手机上 apply_shaping 把它翻成 Down，
                                // 于是恰好两行 —— 而不是让四个按钮自己乱换行。
                                c_main := View {
                                    width: Fill height: Fit
                                    flow: Right
                                    align: Align{x: 0.0, y: 0.5}
                                    spacing: 10.0
                                    c_top := View {
                                        width: Fill height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 8.0
                                        c_name := Label {
                                            width: Fit
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink
                                                text_style +: { font_size: 14.0 }
                                            }
                                        }
                                        c_count := Label {
                                            width: Fill
                                            draw_text +: {
                                                text_overflow: TextOverflow.Ellipsis
                                                max_lines: 1
                                                color: ouyu.ink_2
                                                text_style +: { font_size: 12.5 }
                                            }
                                        }
                                    }
                                    c_acts := View {
                                        width: Fit height: Fit
                                        flow: Right
                                        align: Align{x: 0.0, y: 0.5}
                                        spacing: 6.0
                                        c_view := OuyuBtnSm { text: "回忆" }
                                        c_merge := OuyuBtnSm { text: "合并" }
                                        c_delmem := OuyuBtnSm { text: "清空回忆" }
                                        c_del := OuyuBtnDangerSm { text: "删除" }
                                    }
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
                                            color: ouyu.bad
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
                                    draw_icon +: { svg: crate_resource("self:resources/icons/nav-contacts.svg") color: ouyu.ink_ghost }
                                }
                                em_text := Label {
                                    width: Fit
                                    text: "还没有熟人"
                                    draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 13.0 } }
                                }
                            }
                        }
                    }

                    // ---- 回忆页 ----
                    page_memories := OuyuScrollY {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

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
                                color: ouyu.ink_3
                                text_style +: { font_size: 12.5 line_spacing: 1.35 }
                            }
                        }
                        sec_mem := Label {
                            text: "回忆"
                            draw_text +: {
                                color: ouyu.ink_2
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            m_open := OuyuBtn { text: "打开" }
                        }
                        mem_empty := OuyuEmpty {
                            visible: false
                            em_icon := OuyuIcon {
                                icon_walk: Walk{ width: 28.0 height: Fit }
                                draw_icon +: { svg: crate_resource("self:resources/icons/nav-memories.svg") color: ouyu.ink_ghost }
                            }
                            em_text := Label {
                                width: Fit
                                text: "这里暂时留白"
                                draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 13.0 } }
                            }
                        }
                        sec_hid := Label {
                            width: Fill
                            text: "已隐藏"
                            draw_text +: {
                                wrap: Words
                                color: ouyu.ink_2
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                    color: ouyu.ink_2
                                    text_style +: { font_size: 12.5 }
                                }
                            }
                            m_text := Label {
                                width: Fill
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
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
                                color: ouyu.ink_2
                                text_style +: { font_size: 12.5 line_spacing: 1.35 }
                            }
                        }
                        mm_note := Label {
                            width: Fill
                            text: "隐藏的仍会保存并计次，可随时恢复；隐藏不是加密。"
                            draw_text +: {
                                wrap: Words
                                color: ouyu.ink_3
                                text_style +: { font_size: 12.5 line_spacing: 1.35 }
                            }
                        }
                    }

                    // ---- 「我」页：我的行踪 + 三个入口（券包 / 设置 / 关于）----
                    //
                    // 成就曲线、里程碑和分享卡在相遇页首屏；本机统计那张卡没有了。
                    page_achieve := OuyuScrollY {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

                        // 我的行踪：只放最近发布的几条进行中的；全部历史在「更多」里。
                        tr_head := View {
                            width: Fill height: Fit
                            flow: Right
                            align: Align{x: 0.0, y: 0.5}
                            margin: Inset{top: 6.0, bottom: 2.0}
                            tr_head_text := OuyuGroupHead { width: Fill margin: 0.0 text: "我的行踪" }
                            tr_more := OuyuLink {
                                width: Fit
                                text: "更多"
                                draw_icon +: { svg: crate_resource("self:resources/icons/chevron-right.svg") }
                            }
                        }
                        tr_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 6.0}
                            spacing: 0.0
                            tr0 := OuyuTrackRow { }
                            tr1 := OuyuTrackRow { }
                            tr2 := OuyuTrackRow { }
                            tr_empty := OuyuEmpty {
                                visible: false
                                em_icon := OuyuIcon {
                                    icon_walk: Walk{ width: 28.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/location.svg") color: ouyu.ink_ghost }
                                }
                                em_text := Label {
                                    width: Fit
                                    text: "现在没有进行中的行踪"
                                    draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 13.0 } }
                                }
                            }
                        }
                        // 三个入口：券包、设置、关于。
                        hub_head := OuyuGroupHead { text: "更多" }
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
                    page_wallet := OuyuScrollY {
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
                            draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 24.0 line_spacing: 1.35 } }
                        }
                        wl_sub := Label {
                            width: Fill
                            text: "商户赞助，确认相遇后发放，7 天内使用。"
                            draw_text +: { wrap: Words color: ouyu.ink_2 text_style +: { font_size: 14.0 line_spacing: 1.35 } }
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
                                draw_icon +: { svg: crate_resource("self:resources/icons/lock.svg") color: ouyu.good }
                            }
                            wl_note_text := Label {
                                width: Fill
                                text: "券面只有商户和核销码，没有和谁、在哪、哪天。"
                                draw_text +: { wrap: Words color: ouyu.ink_2 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                        }
                        wl_state := OuyuEmpty {
                            visible: false
                            em_icon := OuyuIcon {
                                icon_walk: Walk{ width: 28.0 height: Fit }
                                draw_icon +: { svg: crate_resource("self:resources/icons/coupon.svg") color: ouyu.ink_ghost }
                            }
                            em_text := Label {
                                width: Fit
                                text: "还没有相遇礼"
                                draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 13.0 } }
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

                    // ---- 我的行踪（「我」页「更多」进入的覆盖页）----
                    page_tracks := OuyuScrollY {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

                        tk_back := OuyuLink {
                            width: Fit
                            text: "返回「我」"
                            draw_icon +: { svg: crate_resource("self:resources/icons/chevron-left.svg") }
                        }
                        tk_title := Label {
                            width: Fill
                            text: "我的行踪"
                            draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 24.0 line_spacing: 1.35 } }
                        }
                        tk_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: Inset{left: 6.0, right: 6.0, top: 6.0, bottom: 6.0}
                            spacing: 0.0
                            tk0 := OuyuTrackRow { }
                            tk1 := OuyuTrackRow { }
                            tk2 := OuyuTrackRow { }
                            tk3 := OuyuTrackRow { }
                            tk4 := OuyuTrackRow { }
                            tk5 := OuyuTrackRow { }
                            tk6 := OuyuTrackRow { }
                            tk7 := OuyuTrackRow { }
                            tk8 := OuyuTrackRow { }
                            tk9 := OuyuTrackRow { }
                            tk10 := OuyuTrackRow { }
                            tk11 := OuyuTrackRow { }
                            tk12 := OuyuTrackRow { }
                            tk13 := OuyuTrackRow { }
                            tk14 := OuyuTrackRow { }
                            tk15 := OuyuTrackRow { }
                            tk16 := OuyuTrackRow { }
                            tk17 := OuyuTrackRow { }
                            tk18 := OuyuTrackRow { }
                            tk19 := OuyuTrackRow { }
                            tk20 := OuyuTrackRow { }
                            tk21 := OuyuTrackRow { }
                            tk22 := OuyuTrackRow { }
                            tk23 := OuyuTrackRow { }
                            tk_empty := OuyuEmpty {
                                visible: false
                                em_icon := OuyuIcon {
                                    icon_walk: Walk{ width: 28.0 height: Fit }
                                    draw_icon +: { svg: crate_resource("self:resources/icons/location.svg") color: ouyu.ink_ghost }
                                }
                                em_text := Label {
                                    width: Fit
                                    text: "还没有发布过行踪"
                                    draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 13.0 } }
                                }
                            }
                        }
                    }

                    // ---- 设置（「我」页进入的覆盖页）----
                    page_settings := OuyuScrollY {
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
                            draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 24.0 line_spacing: 1.35 } }
                        }

                        // ---- 外观 ----
                        //
                        // 放在第一组：它改的是整屏的样子，读设置的人一眼就该
                        // 看见有得选，而不是翻到最后才发现。
                        ap_head := OuyuGroupHead { text: "外观" }
                        ap_card := OuyuCard {
                            width: Fill height: Fit
                            flow: Down
                            padding: 14.0
                            spacing: 10.0
                            ap_name := Label {
                                width: Fill
                                text: "界面深浅"
                                draw_text +: { color: ouyu.ink text_style +: { font_size: 15.0 } }
                            }
                            ap_seg := OuyuSegTrack {
                                ap_dark := OuyuSeg { text: "夜色" }
                                ap_light := OuyuSeg { text: "白昼" }
                            }
                            ap_note := Label {
                                width: Fill
                                text: "只改这台设备上的偶遇，不动系统设置。"
                                draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
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
                                draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
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
                                    draw_text +: { wrap: Words color: ouyu.bad text_style +: { font_size: 12.5 line_spacing: 1.35 } }
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
                                draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
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
                                text: "偶遇 OuYu v0.4"
                                draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 15.0 line_spacing: 1.35 } }
                            }
                            ab_p1 := Label {
                                width: Fill
                                text: "偶遇只做一件事：让本来就可能发生的相遇更容易发生一点，然后退开。"
                                draw_text +: { wrap: Words color: ouyu.ink_2 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                            ab_p2 := Label {
                                width: Fill
                                text: "不做：谁在附近、人数与距离、实时位置、把隐藏回忆算进统计。"
                                draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                            // 跳过引导的人在这里能重看那三句话（02 七.1）。
                            ab_intro := OuyuBtn { text: "重看开场" }
                        }
                    }

                    // ---- 分享卡预览页（成就页「生成分享卡 →」进入；不是侧栏 Tab）----
                    page_share := OuyuScrollY {
                        visible: false
                        width: Fill height: Fill
                        flow: Down
                        spacing: 14.0

                        sp_title := Label {
                            width: Fill
                            text: "让朋友看见，你生活里的光。"
                            draw_text +: {
                                wrap: Words
                                color: ouyu.ink
                                text_style +: { font_size: 24.0 line_spacing: 1.35 }
                            }
                        }
                        sp_sub := Label {
                            width: Fill
                            text: "先预览，再决定发不发。"
                            draw_text +: {
                                wrap: Words
                                color: ouyu.ink_2
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
                                            color: ouyu.ink
                                            text_style +: { font_size: 15.0 line_spacing: 1.35 }
                                        }
                                    }
                                    sp_badge := Label {
                                        text: "仅用可见回忆"
                                        draw_text +: {
                                            color: ouyu.warm
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
                                        color: ouyu.warm
                                        text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                    }
                                }
                                sp_note := Label {
                                    width: Fill
                                    text: "保存后可在任意社交媒体自行发布。"
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink_4
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
                                            color: ouyu.ink
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
                                            color: ouyu.ink
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    spw_b := Label {
                                        width: Fill
                                        text: "汇总次数、成就文案、署名。"
                                        draw_text +: {
                                            wrap: Words
                                            color: ouyu.ink_2
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
                                            color: ouyu.warm
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    spw_note := Label {
                                        width: Fill
                                        text: "曲线会透露近 8 周频率，不含联系人或地点。"
                                        draw_text +: {
                                            wrap: Words
                                            color: ouyu.ink_4
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
                                            color: ouyu.ink
                                            text_style +: { font_size: 14.0 }
                                        }
                                    }
                                    spc_b := Label {
                                        width: Fill
                                        text: "不用专程约，刚好遇见。\n给生活留一点偶然。"
                                        draw_text +: {
                                            wrap: Words
                                            color: ouyu.warm
                                            text_style +: { font_size: 12.5 line_spacing: 1.35 }
                                        }
                                    }
                                    spc_note := Label {
                                        width: Fill
                                        text: "标记朋友前，先问问对方。"
                                        draw_text +: {
                                            wrap: Words
                                            color: ouyu.ink_2
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
                                    color: ouyu.ink_2
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
                    page_memdetail := OuyuScrollY {
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
                                color: ouyu.ink
                                text_style +: { font_size: 22.0 line_spacing: 1.35 }
                            }
                        }
                        md_date := Label {
                            width: Fill
                            text: ""
                            draw_text +: {
                                wrap: Words
                                color: ouyu.ink_2
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
                                draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 15.0 line_spacing: 1.35 } }
                            }
                            md_note := OuyuInput {
                                empty_text: "写一句只给你自己看的"
                            }
                            md_note_tip := Label {
                                width: Fill
                                text: "备注只在本机，不进统计和分享卡。"
                                draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
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
                                draw_text +: { wrap: Words color: ouyu.ink_2 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
                            }
                            md_del := OuyuBtnDanger { width: Fill text: "删除这一条" }
                            md_del_tip := Label {
                                width: Fill
                                text: "删除后 5 秒内可撤销；只删你这一份。"
                                draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.5 line_spacing: 1.35 } }
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
                                draw_bg +: { color: ouyu.blue border_radius: r.tick }
                            }
                            in_d1 := RoundedView {
                                width: 22 height: 4
                                draw_bg +: { color: ouyu.line_soft border_radius: r.tick }
                            }
                            in_d2 := RoundedView {
                                width: 22 height: 4
                                draw_bg +: { color: ouyu.line_soft border_radius: r.tick }
                            }
                            in_gap := View { width: Fill height: Fit }
                            in_skip := OuyuLink { text: "跳过" }
                        }
                        in_mid := OuyuScrollY {
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
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-discover.svg") color: ouyu.warm }
                                    }
                                }
                                in_ic1 := View {
                                    visible: false
                                    width: Fit height: Fit
                                    in_i := OuyuIcon {
                                        icon_walk: Walk{ width: 40.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/nav-meet.svg") color: ouyu.warm }
                                    }
                                }
                                in_ic2 := View {
                                    visible: false
                                    width: Fit height: Fit
                                    in_i := OuyuIcon {
                                        icon_walk: Walk{ width: 40.0 height: Fit }
                                        draw_icon +: { svg: crate_resource("self:resources/icons/lock.svg") color: ouyu.good }
                                    }
                                }
                            }
                            in_title := Label {
                                width: Fill
                                text: ""
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink
                                    text_style +: { font_size: 22.0 line_spacing: 1.35 }
                                }
                            }
                            in_body := Label {
                                width: Fill
                                text: ""
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                        color: ouyu.good
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                ip1 := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.good
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                ip2 := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.good
                                        text_style +: { font_size: 13.0 line_spacing: 1.35 }
                                    }
                                }
                                ip3 := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.good
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
                            draw_bg +: { color: ouyu.card border_color: ouyu.line_notice border_size: 1.0 }
                            nt_icon := OuyuIcon {
                                icon_walk: Walk{ width: 18.0 height: Fit }
                                draw_icon +: { svg: crate_resource("self:resources/icons/bell.svg") color: ouyu.warm }
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
                                        color: ouyu.ink
                                        text_style +: { font_size: 13.5 line_spacing: 1.35 }
                                    }
                                }
                                nt_text := Label {
                                    width: Fill
                                    text: ""
                                    draw_text +: {
                                        wrap: Words
                                        color: ouyu.ink_2
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

                }

                // ---- 右列解释栏（窗口窄于 ~900px 时隐藏）----
                aside := OuyuScrollY {
                    width: 300 height: Fill
                    flow: Down
                    spacing: 14.0

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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            ad1_b := Label {
                                width: Fill
                                text: "看不到谁发布了行程；别人也看不到你的头像、姓名和位置。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            ad2_b := Label {
                                width: Fill
                                text: "1 发布粗区域与时段\n2 正常生活，线下认出彼此\n3 双方互认，可选相遇礼"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            ad3_b := Label {
                                width: Fill
                                text: "发布随时可撤回。宁可少提示，也不披露某个熟人。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            am1_b := Label {
                                width: Fill
                                text: "不持续定位，不向任何人展示坐标。拒绝也能记回忆。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            am2_b := Label {
                                width: Fill
                                text: "保存、隐藏或不保存只影响这一次。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            ac1_b := Label {
                                width: Fill
                                text: "每次确认时选保存、隐藏或不保存，不设永久策略。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 line_spacing: 1.35 }
                                }
                            }
                            ac2_b := Label {
                                width: Fill
                                text: "删除时可选是否连回忆一起删，默认保留。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            ame1_b := Label {
                                width: Fill
                                text: "只记和谁、哪一天，不记时刻、位置或路线。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            ame2_b := Label {
                                width: Fill
                                text: "删不掉对方自己记的那一份。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            aac1_b := Label {
                                width: Fill
                                text: "没有签到、排行榜和惩罚。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                                    color: ouyu.ink
                                    text_style +: { font_size: 14.0 }
                                }
                            }
                            aac2_b := Label {
                                width: Fill
                                text: "分享卡只含汇总次数和文案，不带联系人、地点或日期。"
                                draw_text +: {
                                    wrap: Words
                                    color: ouyu.ink_2
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
                    color: ouyu.bg_chrome
                    border_color: ouyu.line
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
/// 顶栏标题：与导航保持一致。
const TAB_TITLES: [&str; 5] = ["发现", "相遇", "熟人", "回忆", "我"];
/// 「我 → 我的行踪」只铺最近发布的这几条进行中的。
const TRACK_ROWS: [LiveId; 3] = [live_id!(tr0), live_id!(tr1), live_id!(tr2)];
/// 「我的行踪 → 更多」：进行中的全部 + 最多 20 条到期记录。
const HIST_ROWS: [LiveId; 24] = [
    live_id!(tk0),
    live_id!(tk1),
    live_id!(tk2),
    live_id!(tk3),
    live_id!(tk4),
    live_id!(tk5),
    live_id!(tk6),
    live_id!(tk7),
    live_id!(tk8),
    live_id!(tk9),
    live_id!(tk10),
    live_id!(tk11),
    live_id!(tk12),
    live_id!(tk13),
    live_id!(tk14),
    live_id!(tk15),
    live_id!(tk16),
    live_id!(tk17),
    live_id!(tk18),
    live_id!(tk19),
    live_id!(tk20),
    live_id!(tk21),
    live_id!(tk22),
    live_id!(tk23),
];
const PAGES: [LiveId; 5] = [
    live_id!(page_discover),
    live_id!(page_meet),
    live_id!(page_contacts),
    live_id!(page_memories),
    live_id!(page_achieve),
];
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

/// 相遇页首屏：4 / 8 周切换 chips。
const WK_CHIPS: [LiveId; 2] = [live_id!(wk4), live_id!(wk8)];
/// 相遇页首屏：三个里程碑卡片。
const MS_ROWS: [LiveId; 3] = [live_id!(ms0), live_id!(ms1), live_id!(ms2)];
/// 相遇页首屏：「查看每周次数」数据表的 8 行。
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
const PHONE_WRAP_ROWS: [LiveId; 7] = [
    live_id!(rk_head),
    live_id!(ct_add),
    live_id!(ct_merge_bar),
    live_id!(curve_head),
    live_id!(ms_head),
    live_id!(sp_head),
    live_id!(ck_head),
];

/// 每页的大标题：手机形态下统一收小一号。
const PAGE_TITLES: [LiveId; 3] = [
    live_id!(pw_title),
    live_id!(mp_title),
    live_id!(sp_title),
];

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
    /// 右列解释栏是否还有位置。
    aside: bool,
    /// 横屏手机这类矮 surface：压缩街区插图与曲线高度。
    short: bool,
}

/// 开场三屏的正文列宽。
const INTRO_COL: f64 = 620.0;
/// 右列解释栏的栏内容宽度（栏本身还要加一截贴边的内边距）。
const ASIDE_COL: f64 = 300.0;
/// 手机形态的上限宽度：低于此值走底部导航单列。
const PHONE_MAX: f64 = 720.0;
/// 桌面形态（侧栏 + 右列）的下限宽度。
const DESKTOP_MIN: f64 = 1060.0;

/// 自己吃左右留白的那些容器：正文区不留左右内边距，滚动条才贴得住窗口边，
/// 所以这一份留白落到每个可滚动页面（以及发布页那条固定底栏）身上。
const SIDE_PAD_VIEWS: [LiveId; 12] = [
    live_id!(page_discover),
    live_id!(pw_scroll),
    live_id!(pw_bar),
    live_id!(page_meet),
    live_id!(page_contacts),
    live_id!(page_memories),
    live_id!(page_achieve),
    live_id!(page_wallet),
    live_id!(page_tracks),
    live_id!(page_settings),
    live_id!(page_share),
    live_id!(page_memdetail),
];
/// 右列解释栏至少需要的宽度。
const ASIDE_MIN: f64 = 900.0;

/// 可用 surface 尺寸 → 布局形态。宿主把应用放进多大的 tile，这里就按多大
/// 排版：窗口尺寸不代表 tile 尺寸，所以只看自己拿到的 turtle。
/// 手机 / 桌面的切换在宿主的样式菜单里，应用内不再另给一个开关。
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

/// 「我」页进去的两张覆盖页。它们盖住 Tab 页，但不是第六个 Tab ——
/// 券包和设置是从「我」里进去的，退出来还得回到「我」。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sheet {
    Wallet,
    Settings,
    /// 「我的行踪」的全部历史。
    Tracks,
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
    /// 当前色板里 Rust 侧要用到的那几个角色（见 theme.rs 的 `Pal`）。
    /// 换主题时跟着 `restyle` 一起重读。
    #[rust]
    pal: Pal,
    /// Tab 淡入动画: 起始时间与当前不透明度。
    #[rust]
    fade_start: Option<f64>,
    #[rust]
    fade_alpha: f32,
    #[rust]
    next_frame: NextFrame,

    // ---- 发现页 ----
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
    /// 向导正在修改的那条行踪（publish id）；None = 在写新的一条。
    #[rust]
    wizard_edit: Option<usize>,
    /// 「我 → 我的行踪」每一行铺的是哪条（publish id）。
    #[rust]
    track_row_ids: Vec<usize>,
    /// 「我的行踪 → 更多」每一行铺的是哪条（publish id）。
    #[rust]
    hist_row_ids: Vec<usize>,
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
    /// 互认流程是否打开（首屏 → 选人）。会话建立后由 `session` 接管。
    #[rust]
    meet_open: bool,
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
    /// 头部「+」点开的导入菜单是否展开。
    #[rust]
    import_menu: bool,
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
    /// 这一拍点下的深浅。换主题会把整棵树重刷一遍，所以不在事件处理中途
    /// 动手 —— 记下来，等这一拍的 action 全走完再换。
    #[rust]
    pending_theme: Option<ThemeMode>,

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
            self.meet_open = false;
        }
        // 切 Tab 即离开分享卡预览和覆盖页（它们都不是 Tab）。
        // 发布向导也一样离开，草稿丢掉 —— 否则底部导航高亮在「我」，
        // 屏幕上却还摆着发布向导的第 1 步。
        self.share_open = false;
        self.sheet = None;
        self.clear_armed = false;
        self.wizard = None;
        self.wizard_edit = None;
        self.mem_detail = None;
        self.merge_from = None;
        self.add_error = None;
        self.import_menu = false;
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
                    script_apply_eval!(cx, tab, { draw_icon +: { color: #(self.pal.warm) } });
                } else {
                    script_apply_eval!(cx, tab, { draw_icon +: { color: #(self.pal.ink_2) } });
                }
            }
        }
        self.view
            .label(cx, ids!(shell.topbar.tb_mid.tb_title))
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
            4 => self.refresh_me(cx),
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
            .widget(cx, ids!(page_tracks))
            .set_visible(cx, self.sheet == Some(Sheet::Tracks));
        self.view
            .widget(cx, ids!(page_publish))
            .set_visible(cx, wizard && !self.share_open && self.sheet.is_none() && !intro);
        self.refresh_topbar(cx);
    }

    /// 顶栏右边的主动作：发现页「发布行踪」，相遇页首屏「确认相遇」。
    /// 覆盖页开着、互认流程已经进去、或别的 Tab 上都收起来。
    fn refresh_topbar(&mut self, cx: &mut Cx) {
        let overlay = self.share_open
            || self.wizard.is_some()
            || self.sheet.is_some()
            || self.intro.is_some()
            || self.mem_detail.is_some();
        let action = if overlay {
            None
        } else {
            match self.state.tab {
                0 => Some("发布行踪"),
                1 if self.session.is_none() && !self.meet_open => Some("确认相遇"),
                _ => None,
            }
        };
        let btn = self.view.button(cx, ids!(shell.topbar.tb_bar.tb_action));
        btn.set_visible(cx, action.is_some());
        if let Some(text) = action {
            btn.set_text(cx, text);
        }
        // 熟人页的「+」也挂在顶栏上，和标题「熟人」同一条线。
        self.view
            .button(cx, ids!(shell.topbar.tb_bar.tb_add))
            .set_visible(cx, !overlay && self.state.tab == 2);
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
        self.refresh_me(cx);
    }

    // ---- 主题 ----

    /// 换一套深浅。
    ///
    /// 走的是宿主给模块换外观的同一条路（`module_host.rs` 的 `apply_style`）：
    /// 在当前 VM 里重跑一次「装色板 → 重建预设」，再拿新的类型默认值把整棵树
    /// `ScriptReapply` 一遍。之后重读 Rust 侧那几个角色，并把所有跟数据走的
    /// 颜色（选中的 Tab、强度分档、开关）重新写一次 —— reapply 只会把预设里
    /// 的颜色刷回去，那几处是每次刷新时才算出来的。
    fn apply_theme(&mut self, cx: &mut Cx, mode: ThemeMode) {
        if theme::mode() == mode {
            return;
        }
        theme::set_mode(mode);
        cx.with_vm(|vm| {
            vm.with_reload(|vm| {
                theme::install(vm);
                canvas::script_mod(vm);
                script_mod(vm);
            });
            let source = script_eval!(vm, { mod.widgets.OuyuView });
            self.script_apply(vm, &Apply::ScriptReapply, &mut Scope::empty(), source);
        });
        self.after_restyle(cx);
    }

    /// 重刷一遍界面上所有「不在预设里」的东西。换主题之后、以及独立窗口
    /// 走完 `Event::LiveEdit` 之后都要跑一次。
    fn after_restyle(&mut self, cx: &mut Cx) {
        self.pal = Pal::read(cx);
        // reapply 把布局也按预设刷回去了（手机 / 桌面是运行时算的），
        // 清掉缓存的形态，下一帧按当前尺寸重排一次。
        self.shaping = None;
        self.refresh_all(cx);
        self.update_page_visibility(cx);
        self.redraw(cx);
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
                script_apply_eval!(cx, dot, { draw_bg +: { color: #(self.pal.blue) } });
            } else {
                script_apply_eval!(cx, dot, { draw_bg +: { color: #(self.pal.line_soft) } });
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

    /// 按当前 surface 尺寸重排。托管在 tile 里时窗口尺寸没有意义，
    /// 所以只看 draw_walk 拿到的 turtle 矩形（见 Widget::draw_walk）。
    fn update_responsive(&mut self, cx: &mut Cx, size: Vec2d) {
        if size.x < 2.0 || size.y < 2.0 {
            return;
        }
        self.last_size = size;
        let want = shaping_for(size);
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

        // 手机：左侧栏收起，导航去底部。顶栏（标题 + 当页主动作）所有形态都在。
        self.view.widget(cx, ids!(sidebar)).set_visible(cx, !phone && !intro);
        self.view.widget(cx, ids!(shell.topbar)).set_visible(cx, !intro);
        self.view.widget(cx, ids!(shell.tabbar)).set_visible(cx, phone && !intro);
        self.view.widget(cx, ids!(main.aside)).set_visible(cx, s.aside && !intro);
        if let Some(mut top) = self.view.view(cx, ids!(shell.topbar.tb_bar)).borrow_mut() {
            top.layout.padding = if phone {
                Inset { left: 18.0, right: 12.0, top: 0.0, bottom: 0.0 }
            } else {
                Inset { left: 20.0, right: 20.0, top: 0.0, bottom: 0.0 }
            };
        }

        // 左右这两截留白不放在 main 上：滚动条画在滚动视图自己的右边缘，
        // main 一旦有右内边距，整条滚动条就跟着往里挪一截，看着像浮在半空。
        // 所以 main 只留上下，左右由每页（和右列）自己吃，滚动条贴着窗口边。
        let side = if phone { 16.0 } else { 20.0 };

        // 开场三屏：宽屏上把整页收进一条 620px 的列。三句话是要被读完的，
        // 一行铺满 1280px 读起来会跳行。窄屏收不出这条列，至少留住侧边距。
        if let Some(mut v) = self.view.view(cx, ids!(page_intro)).borrow_mut() {
            let col = ((self.last_size.x - INTRO_COL) * 0.5).max(side);
            v.layout.padding = Inset { left: col, right: col, top: 0.0, bottom: 0.0 };
        }

        if let Some(mut main) = self.view.view(cx, ids!(main)).borrow_mut() {
            // 顶栏已经占了一截，正文区上边距收一点。
            main.layout.padding = if phone {
                Inset { left: 0.0, right: 0.0, top: 6.0, bottom: 10.0 }
            } else {
                Inset { left: 0.0, right: 0.0, top: 8.0, bottom: 16.0 }
            };
            // 正文和右列之间的空档改由正文自己的右内边距顶出来。
            main.layout.spacing = 0.0;
        }
        for id in SIDE_PAD_VIEWS {
            if let Some(mut v) = self.view.view(cx, &[id]).borrow_mut() {
                v.layout.padding.left = side;
                v.layout.padding.right = side;
            }
        }
        // 右列：左边紧挨正文的右内边距，右边自己留一截贴住窗口。
        // 宽度补上这一截，栏内容还是 300。
        if let Some(mut aside) = self.view.view(cx, ids!(main.aside)).borrow_mut() {
            aside.layout.padding.left = 0.0;
            aside.layout.padding.right = side;
            aside.walk.width = Size::Fixed(ASIDE_COL + side);
        }

        // 带 Fill 子项的行：手机上换行，宽屏保持单行。
        for id in PHONE_WRAP_ROWS {
            self.set_row_wrap(cx, &[id], phone);
        }
        for row in CONTACT_ROWS {
            // 熟人行不换行，改成整块翻向：宽屏一条 Right（名字次数在左，
            // 动作在右，一行），手机竖过来（名字次数一行，动作一行，两行）。
            // 交给 wrap 的话按钮会按剩余宽度自己断，断出第三行来。
            if let Some(mut view) = self.view.view(cx, &[row, live_id!(c_main)]).borrow_mut() {
                view.layout.flow = if phone {
                    Flow::Down
                } else {
                    Flow::Right { row_align: RowAlign::Top, wrap: false }
                };
                view.layout.spacing = if phone { 6.0 } else { 10.0 };
            }
            self.set_row_wrap(cx, &[row, live_id!(c_confirm)], phone);
        }
        for row in MEM_ROWS.into_iter().chain(HID_ROWS) {
            self.set_row_wrap(cx, &[row], phone);
        }
        for row in TRACK_ROWS {
            self.set_row_wrap(cx, &[live_id!(page_achieve), live_id!(tr_card), row], phone);
        }
        for row in HIST_ROWS {
            self.set_row_wrap(cx, &[live_id!(page_tracks), live_id!(tk_card), row], phone);
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
        let snap = ai::OpportunitySnapshot::from_state(&self.state, true);
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
                script_apply_eval!(cx, lvw, { draw_text +: { color: #(self.pal.warm) } });
                script_apply_eval!(cx, dot, { draw_icon +: { color: #(self.pal.warm) } });
            }
            OppLevel::Possible => {
                script_apply_eval!(cx, lvw, { draw_text +: { color: #(self.pal.blue) } });
                script_apply_eval!(cx, dot, { draw_icon +: { color: #(self.pal.blue) } });
            }
            OppLevel::Few => {
                script_apply_eval!(cx, lvw, { draw_text +: { color: #(self.pal.ink_2) } });
                script_apply_eval!(cx, dot, { draw_icon +: { color: #(self.pal.ink_2) } });
            }
            OppLevel::BelowThreshold => {
                script_apply_eval!(cx, dot, { draw_icon +: { color: #(self.pal.ink_ghost) } });
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
                0 => script_apply_eval!(cx, w, { draw_icon +: { color: #(self.pal.heat_0) } }),
                1..=14 => script_apply_eval!(cx, w, { draw_icon +: { color: #(self.pal.heat_1) } }),
                15..=24 => script_apply_eval!(cx, w, { draw_icon +: { color: #(self.pal.blue) } }),
                _ => script_apply_eval!(cx, w, { draw_icon +: { color: #(self.pal.warm) } }),
            }
        }

        // ---- 排行 ----
        let ranking = opportunity_ranking_at(today, self.day_sel);
        let top: Vec<AreaOpportunity> = ranking
            .iter()
            .copied()
            .filter(|o| o.level.shown())
            .take(RANK_ROWS.len())
            .collect();
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
            .widget(cx, ids!(page_discover.rank_card.rk_note))
            .set_visible(cx, !has_top);
        if !has_top {
            self.view
                .label(cx, ids!(page_discover.rank_card.rk_note))
                .set_text(cx, "这一天参与的人还不够多，先不显示。");
        }
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

    }

    // ---- 发布向导 ----

    /// 当前草稿对应的一行预览文案。
    fn draft_publish(&self) -> Publish {
        Publish {
            id: usize::MAX,
            date: today_days() + self.draft.0.min(DAY_SPAN - 1) as i64,
            slot: self.draft.1,
            area: self.draft.2,
            intent: self.draft.3,
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
        let editing = self.wizard_edit.is_some();
        self.view
            .label(cx, ids!(page_publish.pw_top.pw_title))
            .set_text(cx, if editing { "修改这条行踪" } else { "写一下你的行踪" });
    }

    /// 打开发布向导。`area` 给了就预选那个片区（从排行里点进来的情况）；
    /// `edit` 给了就是在改那一条（从「我的行踪」点「修改」进来）。
    fn open_wizard(&mut self, cx: &mut Cx, area: Option<u16>, edit: Option<usize>) {
        let today = today_days();
        let base = edit.and_then(|id| self.state.publishes.iter().find(|p| p.id == id).cloned());
        self.wizard_edit = base.as_ref().map(|p| p.id);
        self.draft = match &base {
            Some(p) => (p.day_at(today).unwrap_or(0), p.slot, p.area, p.intent),
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
        self.wizard_edit = None;
        self.update_page_visibility(cx);
        self.refresh_discover(cx);
        self.refresh_me(cx);
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
        // 首屏（成就内容）只在没有会话、也没点「确认相遇」的时候露出来。
        let home = stage.is_none() && !self.meet_open;
        self.view
            .widget(cx, ids!(page_meet.meet_home))
            .set_visible(cx, home);
        self.view
            .widget(cx, ids!(page_meet.meet_pick))
            .set_visible(cx, stage.is_none() && self.meet_open);
        // 首屏（成就内容）不再顶大标题；大标题只给互认流程。
        self.view
            .widget(cx, ids!(page_meet.mp_title))
            .set_visible(cx, !home);
        self.view
            .widget(cx, ids!(page_meet.mp_sub))
            .set_visible(cx, !home);
        if home {
            self.refresh_achievements(cx);
        }
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
        self.refresh_topbar(cx);
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
            if r.redeemed { "已核销" } else { "到店出示核销码" },
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
        self.meet_open = false;
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
            .set_text(cx, &format!("我的熟人 · {} 位", n));

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
            "从本机通讯录导入",
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
                    let text = "从我的熟人里移除，不改系统通讯录。默认保留旧回忆。";
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
            .widget(cx, ids!(page_contacts.ct_card.ct_menu_row))
            .set_visible(cx, self.import_menu);
    }

    /// 一批名字加成熟人。重名 / 空名 / 过长的那几条静静跳过 ——
    /// 批量导入不适合逐条弹错，返回真正加进去的人数由调用方去说。
    fn adopt_names(&mut self, names: Vec<String>) -> usize {
        let mut added = 0;
        for n in names {
            if self.state.add_contact(&n).is_ok() {
                added += 1;
            }
        }
        added
    }

    /// 「从本机导入」：把本机通讯录池里还没加的人一次加成熟人。
    fn import_local(&mut self, cx: &mut Cx) {
        let names = self.state.directory.clone();
        let added = self.adopt_names(names);
        if added == 0 {
            self.toast(cx, "本机通讯录里没有新的人");
        } else {
            self.toast(cx, &format!("从本机通讯录加了 {} 位熟人", added));
        }
        self.refresh_contacts(cx);
    }

    /// 「从文件导入」：读 <MAKEPAD_HOME>/ouyu/contacts.vcf，名字加成熟人。
    fn import_vcard(&mut self, cx: &mut Cx) {
        let Some(path) = OuyuState::contacts_vcf() else {
            self.toast(cx, "未设置 MAKEPAD_HOME, 找不到状态目录");
            return;
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                let msg = format!("把 .vcf 放到 {} 再点我", path.display());
                self.toast(cx, &msg);
                return;
            }
        };
        let added = self.adopt_names(parse_vcard(&text));
        self.state.save();
        if added == 0 {
            self.toast(cx, "文件里的人都已经在熟人里了");
        } else {
            self.toast(cx, &format!("从文件加了 {} 位熟人", added));
        }
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

    /// 把一组行踪铺到一张卡的行上：文案、状态、到期的收起修改 / 删除。
    /// 返回铺上去的那些 publish id，行下标对应。
    fn fill_track_rows(
        &mut self,
        cx: &mut Cx,
        card: [LiveId; 2],
        rows: &[LiveId],
        items: &[(usize, String, bool)],
    ) -> Vec<usize> {
        for (i, id) in rows.iter().enumerate() {
            let base = [card[0], card[1], *id];
            // 行里的两行装在 tr_col 里（见 canvas.rs 的 OuyuTrackRow）。
            let col = [card[0], card[1], *id, live_id!(tr_col)];
            let Some((_, text, gone)) = items.get(i) else {
                self.view.widget(cx, &base).set_visible(cx, false);
                continue;
            };
            self.view.widget(cx, &base).set_visible(cx, true);
            let mut text_w = self
                .view
                .widget(cx, &[col[0], col[1], col[2], col[3], live_id!(tr_top), live_id!(tr_text)]);
            text_w.set_text(cx, text);
            if *gone {
                script_apply_eval!(cx, text_w, { draw_text +: { color: #(self.pal.ink_3) } });
            } else {
                script_apply_eval!(cx, text_w, { draw_text +: { color: #(self.pal.ink) } });
            }
            self.view
                .widget(cx, &join(&col, live_id!(tr_state)))
                .set_text(cx, if *gone { "已到期" } else { "进行中 · 到时段结束" });
            self.view
                .widget(cx, &[col[0], col[1], col[2], col[3], live_id!(tr_top), live_id!(tr_edit)])
                .set_visible(cx, !gone);
            self.view
                .widget(cx, &[col[0], col[1], col[2], col[3], live_id!(tr_top), live_id!(tr_del)])
                .set_visible(cx, !gone);
        }
        items.iter().take(rows.len()).map(|r| r.0).collect()
    }

    /// 「我的行踪 → 更多」：进行中的在前（可改可删），到期的在后（只看）。
    fn refresh_tracks(&mut self, cx: &mut Cx) {
        let today = today_days();
        let now = minutes_of_day();
        let items: Vec<(usize, String, bool)> = self
            .state
            .publish_history(today, now)
            .into_iter()
            .take(HIST_ROWS.len())
            .map(|p| (p.id, p.text_at(today), p.expired_at(today, now)))
            .collect();
        self.hist_row_ids =
            self.fill_track_rows(cx, [live_id!(page_tracks), live_id!(tk_card)], &HIST_ROWS, &items);
        self.view
            .widget(cx, ids!(page_tracks.tk_card.tk_empty))
            .set_visible(cx, items.is_empty());
    }

    /// 「我」页：我的行踪（最近发布的几条进行中的）+ 三个入口。
    fn refresh_me(&mut self, cx: &mut Cx) {
        let today = today_days();
        let now = minutes_of_day();
        let mut live = self.state.active_publishes(today, now);
        live.sort_by_key(|p| std::cmp::Reverse(p.id));
        let items: Vec<(usize, String, bool)> = live
            .into_iter()
            .take(TRACK_ROWS.len())
            .map(|p| (p.id, p.text_at(today), false))
            .collect();
        self.track_row_ids =
            self.fill_track_rows(cx, [live_id!(page_achieve), live_id!(tr_card)], &TRACK_ROWS, &items);
        self.view
            .widget(cx, ids!(page_achieve.tr_card.tr_empty))
            .set_visible(cx, items.is_empty());
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
            "v0.4",
        );
    }

    /// 相遇页首屏的成就内容：频率曲线、每周次数、里程碑（分享卡预览开着时一并刷）。
    fn refresh_achievements(&mut self, cx: &mut Cx) {
        let today = today_days();
        let stats = achievement_stats(&self.state.encounters, self.weeks, today);
        // 曲线卡片：chips + 数据 + 留白。
        self.view
            .widget(cx, ids!(page_meet.meet_home.curve_card.curve_head.curve_title))
            .set_text(cx, "相遇频率曲线");
        self.set_chip_group(
            cx,
            &[
                live_id!(page_meet),
                live_id!(meet_home),
                live_id!(curve_card),
                live_id!(curve_head),
            ],
            &WK_CHIPS,
            if self.weeks == 4 { 0 } else { 1 },
        );
        self.view
            .widget(cx, ids!(page_meet.meet_home.curve_card.curve_sub))
            .set_text(cx, &format!("最近 {} 周，记住 {} 次重逢", self.weeks, stats.recent()));
        let has_data = stats.recent() > 0;
        self.view
            .widget(cx, ids!(page_meet.meet_home.curve_card.curve_chart))
            .set_visible(cx, has_data);
        self.view
            .widget(cx, ids!(page_meet.meet_home.curve_card.curve_empty))
            .set_visible(cx, !has_data);
        let chart = self.view.widget(cx, ids!(page_meet.meet_home.curve_card.curve_chart));
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
            .widget(cx, ids!(page_meet.meet_home.curve_card.wk_toggle))
            .set_text(cx, if self.weekly_open { "收起" } else { "每周次数" });
        self.view
            .widget(cx, ids!(page_meet.meet_home.curve_card.wk_table))
            .set_visible(cx, self.weekly_open);
        for (i, id) in WK_ROWS.iter().enumerate() {
            let base = [
                live_id!(page_meet),
                live_id!(meet_home),
                live_id!(curve_card),
                live_id!(wk_table),
                *id,
            ];
            match stats.weekly.get(i) {
                Some(w) if self.weekly_open => {
                    self.view.widget(cx, &base).set_visible(cx, true);
                    self.view
                        .widget(cx, &join(&base, live_id!(wk_d)))
                        .set_text(cx, &format!("{} 这一周", fmt_md(w.start)));
                    self.view
                        .widget(cx, &join(&base, live_id!(wk_c)))
                        .set_text(cx, &format!("{} 次", w.count));
                }
                _ => self.view.widget(cx, &base).set_visible(cx, false),
            }
        }
        // 里程碑：点亮 / 等自然发生。
        let ms = milestones(&stats);
        for (i, id) in MS_ROWS.iter().enumerate() {
            let base = [
                live_id!(page_meet),
                live_id!(meet_home),
                live_id!(ms_card),
                live_id!(ms_row),
                *id,
            ];
            let m = &ms[i];
            self.view
                .widget(cx, &join(&base, live_id!(ms_icon)))
                .set_text(cx, if m.lit { "★" } else { "☆" });
            self.view
                .widget(cx, &join(&base, live_id!(ms_state)))
                .set_text(cx, if m.lit { "已点亮" } else { "等自然发生" });
        }
        if self.share_open {
            self.refresh_share(cx);
        }
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
        let show_list = n > 0;
        self.view.widget(cx, slot).set_visible(cx, !show_list);
        if show_list {
            return true;
        }
        self.view
            .widget(cx, &join(slot, live_id!(em_text)))
            .set_text(cx, empty_text);
        self.view
            .widget(cx, &join(slot, live_id!(em_sub)))
            .set_visible(cx, false);
        self.view
            .widget(cx, &join(slot, live_id!(em_action)))
            .set_visible(cx, !empty_action.is_empty());
        self.view
            .widget(cx, &join(slot, live_id!(em_action)))
            .set_text(cx, empty_action);
        false
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
            let show = !items.is_empty();
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
            script_apply_eval!(cx, card, { draw_bg +: { color: #(self.pal.coupon) } });
        } else {
            script_apply_eval!(cx, card, { draw_bg +: { color: #(self.pal.coupon_off) } });
        }
    }

    // ---- 设置 ----

    fn refresh_settings(&mut self, cx: &mut Cx) {
        let mode = theme::mode();
        for (id, m) in [
            (live_id!(ap_dark), ThemeMode::Dark),
            (live_id!(ap_light), ThemeMode::Light),
        ] {
            let path = [live_id!(page_settings), live_id!(ap_card), live_id!(ap_seg), id];
            self.view
                .check_box(cx, &path)
                .set_active(cx, m == mode, Animate::No);
        }
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
        script_apply_eval!(cx, clear_name, { draw_text +: { color: #(self.pal.bad) } });
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
            script_apply_eval!(cx, track_w, { draw_bg +: { color: #(self.pal.blue) } });
        } else {
            script_apply_eval!(cx, track_w, { draw_bg +: { color: #(self.pal.track_off) } });
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
        // 顶栏主动作：发现页写一条新行踪；相遇页首屏进互认流程。
        if self.view.button(cx, ids!(shell.topbar.tb_bar.tb_action)).clicked(actions) {
            match self.state.tab {
                0 => self.open_wizard(cx, None, None),
                1 => {
                    self.meet_open = true;
                    self.view
                        .widget(cx, ids!(page_meet.meet_pick.pick_done))
                        .set_visible(cx, false);
                    self.refresh_meet(cx);
                    self.redraw(cx);
                }
                _ => {}
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
                self.open_wizard(cx, area, None);
            }
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
                    let (day, slot, area, intent) = self.draft;
                    match self.wizard_edit {
                        Some(id) => {
                            self.state.update_publish(id, day, slot, area, intent);
                            self.toast(cx, "这条行踪改好了");
                        }
                        None => {
                            self.state.publish(day, slot, area, intent);
                            self.toast(cx, "已发布，到期自动退出");
                        }
                    }
                    self.close_wizard(cx);
                }
                Some(step) => {
                    self.goto_step(cx, step + 1);
                }
                None => {}
            }
        }
        // 相遇页 ①: 返回首屏
        if self
            .view
            .button(cx, ids!(page_meet.meet_pick.pick_back))
            .clicked(actions)
        {
            self.meet_open = false;
            self.refresh_meet(cx);
            self.redraw(cx);
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
        // 熟人页: 顶栏「+」展开 / 收起导入菜单
        if self
            .view
            .button(cx, ids!(shell.topbar.tb_bar.tb_add))
            .clicked(actions)
        {
            self.import_menu = !self.import_menu;
            self.refresh_contacts(cx);
        }
        // 熟人页: 导入菜单的两条
        if self
            .view
            .button(cx, ids!(page_contacts.ct_card.ct_menu_row.ct_menu.im_local))
            .clicked(actions)
        {
            self.import_menu = false;
            self.import_local(cx);
        }
        if self
            .view
            .button(cx, ids!(page_contacts.ct_card.ct_menu_row.ct_menu.im_file))
            .clicked(actions)
        {
            self.import_menu = false;
            self.import_vcard(cx);
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

        // 相遇页首屏: 4 / 8 周切换
        for i in 0..WK_CHIPS.len() {
            let path = [
                live_id!(page_meet),
                live_id!(meet_home),
                live_id!(curve_card),
                live_id!(curve_head),
                WK_CHIPS[i],
            ];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                self.weeks = if i == 0 { 4 } else { 8 };
                self.set_chip_group(cx, &path[..4], &WK_CHIPS, i);
                self.refresh_achievements(cx);
            }
        }
        // 相遇页首屏: 查看 / 收起每周次数
        if self
            .view
            .button(cx, ids!(page_meet.meet_home.curve_card.wk_toggle))
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
        // 「我」页: 我的行踪 → 更多（全部历史）
        if self
            .view
            .button(cx, ids!(page_achieve.tr_head.tr_more))
            .clicked(actions)
        {
            self.sheet = Some(Sheet::Tracks);
            self.refresh_tracks(cx);
            self.update_page_visibility(cx);
        }
        // 覆盖页: 返回「我」
        if self.view.button(cx, ids!(page_wallet.wl_back)).clicked(actions)
            || self.view.button(cx, ids!(page_settings.se_back)).clicked(actions)
            || self.view.button(cx, ids!(page_tracks.tk_back)).clicked(actions)
        {
            self.sheet = None;
            self.clear_armed = false;
            self.refresh_me(cx);
            self.update_page_visibility(cx);
        }
        // 我的行踪（「我」页那几条 / 「更多」里的全部）—— 修改 / 删除。
        // 只有没到期的行才露出这两个按钮；向导是盖在 Tab 页上的，从「更多」
        // 进去改，就先收起「更多」，改完落回「我」。
        let mut track_edit: Option<usize> = None;
        let mut track_del: Option<usize> = None;
        let lists: [([LiveId; 2], &[LiveId], &[usize]); 2] = [
            ([live_id!(page_achieve), live_id!(tr_card)], &TRACK_ROWS, &self.track_row_ids),
            ([live_id!(page_tracks), live_id!(tk_card)], &HIST_ROWS, &self.hist_row_ids),
        ];
        for (card, rows, ids) in lists {
            for (i, id) in rows.iter().enumerate() {
                let Some(pid) = ids.get(i).copied() else { continue };
                let top = [card[0], card[1], *id, live_id!(tr_col), live_id!(tr_top)];
                if self.view.button(cx, &join(&top, live_id!(tr_edit))).clicked(actions) {
                    track_edit = Some(pid);
                }
                if self.view.button(cx, &join(&top, live_id!(tr_del))).clicked(actions) {
                    track_del = Some(pid);
                }
            }
        }
        if let Some(pid) = track_edit {
            self.sheet = None;
            self.open_wizard(cx, None, Some(pid));
        }
        if let Some(pid) = track_del {
            self.state.withdraw(pid);
            self.toast(cx, "已删除这条行踪");
            self.refresh_me(cx);
            if self.sheet == Some(Sheet::Tracks) {
                self.refresh_tracks(cx);
            }
            self.refresh_discover(cx);
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
                    self.refresh_me(cx);
                    self.refresh_meet(cx);
                }
            }
        }
        // 设置: 界面深浅。这一拍只记下来，真正换在 handle_event 末尾。
        for (id, m) in [
            (live_id!(ap_dark), ThemeMode::Dark),
            (live_id!(ap_light), ThemeMode::Light),
        ] {
            let path = [live_id!(page_settings), live_id!(ap_card), live_id!(ap_seg), id];
            if self.view.check_box(cx, &path).changed(actions).is_some() {
                if theme::mode() == m {
                    // 点的是已经选中的那一格：分段不该像开关一样被关掉。
                    self.refresh_settings(cx);
                } else {
                    self.state.settings.set_theme_mode(m);
                    self.state.save();
                    self.pending_theme = Some(m);
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
        // 空态上的那个按钮：熟人页是「从本机通讯录导入」。
        if self
            .view
            .button(cx, ids!(page_contacts.ct_card.ct_empty.em_action))
            .clicked(actions)
        {
            self.import_local(cx);
        }
        // 撤销条
        if self.view.button(cx, ids!(toast_layer.toast.to_undo)).clicked(actions) {
            self.close_undo(cx, true);
        }
        // 相遇页首屏: 生成分享卡 → 预览页
        if self
            .view
            .button(cx, ids!(page_meet.meet_home.share_card.sh_go))
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
            self.pal = Pal::read(cx);
            self.refresh_all(cx);
            // 第一次打开先讲三句话。跳过也记 onboarded，不会每次都拦。
            if !self.state.settings.onboarded {
                self.open_intro(cx, 0);
            }
            self.notice_poll = cx.start_interval(60.0);
            self.initialized = true;
        }
        // 独立窗口换主题走的是 app_main 的 LiveEdit（窗口底色和标题栏也得跟着
        // 换，那两处不在 OuyuView 里）。Rebake 会把 DSL 里的文案刷回去，所以
        // 收到之后把界面重新铺一遍。
        if let Event::LiveEdit = event {
            self.after_restyle(cx);
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
        // 这一拍的 action 都走完了，现在换主题才不会把半路用到的 widget
        // 引用换掉。
        if let Some(mode) = self.pending_theme.take() {
            self.apply_theme(cx, mode);
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
        // 色板先进 VM：后面两个 script_mod 里的预设都按 `ouyu.<角色>` 取色。
        theme::install(vm);
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
    fn the_layout_follows_the_surface_it_is_actually_given() {
        // 宿主手机壳给的就是 412×892 的一整块面; 桌面 tile 才够放右列。
        assert_eq!(shaping_for(size(412.0, 892.0)).shape, Shape::Phone);
        assert_eq!(shaping_for(size(820.0, 700.0)).shape, Shape::Tablet);
        assert_eq!(shaping_for(size(1280.0, 800.0)).shape, Shape::Desktop);
    }

    #[test]
    fn the_aside_column_only_appears_when_the_desktop_shape_has_room() {
        assert!(shaping_for(size(1280.0, 800.0)).aside);
        assert!(!shaping_for(size(820.0, 700.0)).aside);
        assert!(!shaping_for(size(412.0, 892.0)).aside);
    }

    #[test]
    fn a_short_surface_is_flagged_so_the_illustrations_shrink() {
        assert!(shaping_for(size(1400.0, 440.0)).short);
        assert!(!shaping_for(size(1400.0, 800.0)).short);
    }
}
