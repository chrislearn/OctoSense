//! 礼遇的自绘画布组件与页面预设：圆角矩形 shader、神秘礼卡预览，
//! 以及各页共用的卡片 / 字体层级 / 按钮 / 列表行。
//!
//! 页面只用这里的预设，不再逐处写颜色 / 字号字面量；颜色一律取自 `liyu.*`
//! 色板（theme.rs），圆角取 `r.card / button / chip / tick`。
use crate::share::{ShareCardScene, ShareStyle};
use makepad_widgets::*;

script_mod! {
    use mod.prelude.liyu.*
    use mod.widgets.*

    // 圆角矩形 + 可选描边: 卡片骨架、礼卡圆环都靠它。
    set_type_default() do #(DrawRoundBox::script_shader(vm)){
        ..mod.draw.DrawQuad
        pixel: fn(){
            let sdf = Sdf2d.viewport(self.pos * self.rect_size)
            let bs = min(self.border_size, min(self.rect_size.x, self.rect_size.y) * 0.5)
            let radius = min(self.radius, min(self.rect_size.x, self.rect_size.y) * 0.5)
            sdf.box(bs, bs, self.rect_size.x - bs * 2.0, self.rect_size.y - bs * 2.0, radius)
            sdf.fill_keep(vec4(self.color.rgb, self.color.a * self.alpha_scale))
            if bs > 0.01 {
                sdf.stroke(self.border_color, bs)
            }
            return sdf.result
        }
    }

    // 滚动条：手柄颜色也从色板取。
    //
    // makepad 的 ScrollBar 默认取 `theme.color_outset` —— 那是 widget 主题的
    // 颜色，宿主的样式表说了算，不跟着礼遇的深浅走。白昼版跑在夜色宿主里时，
    // 它会在白纸上画一条深蓝手柄（独立窗口下反过来：白色 10% 几乎看不见）。
    // 每个 ScrollYView 都换成这个预设，深浅两套都由 `liyu.bar` 决定。
    mod.widgets.LiyuScrollBar = mod.widgets.ScrollBar {
        draw_bg +: {
            color: liyu.bar
            color_hover: liyu.bar_hi
            color_drag: liyu.bar_drag
        }
    }

    // 和 makepad 的 ScrollYView 一样的一份，只换掉手柄。
    mod.widgets.LiyuScrollY = mod.widgets.ViewBase {
        scroll_bars: mod.widgets.ScrollBars {
            show_scroll_x: false
            show_scroll_y: true
            scroll_bar_y: mod.widgets.LiyuScrollBar { drag_scrolling: true }
        }
    }

    // 卡片分三级（docs/02 六·1）：主卡 / 次卡 / 内联行。
    // 主卡: 20 圆角 + 1px 描边 + 略提亮底，用于页面核心内容。
    mod.widgets.LiyuCard = mod.widgets.RoundedView{
        width: Fill height: Fit
        flow: Down
        padding: 18.0
        spacing: 12.0
        draw_bg +: {
            color: liyu.card
            border_color: liyu.line_soft
            border_size: 1.0
            border_radius: r.card
        }
    }

    // 次卡: 无描边、更暗、更小圆角，用于附属信息。
    mod.widgets.LiyuCard2 = mod.widgets.RoundedView{
        width: Fill height: Fit
        flow: Down
        padding: 14.0
        spacing: 10.0
        draw_bg +: {
            color: liyu.card_2
            border_color: #0000
            border_size: 0.0
            border_radius: r.card
        }
    }

    // 内联行: 无底色，底部一条分隔线（列表项）。
    mod.widgets.LiyuRow = mod.widgets.RoundedView{
        width: Fill height: Fit
        flow: Right
        align: Align{x: 0.0, y: 0.5}
        padding: Inset{left: 0.0, right: 0.0, top: 12.0, bottom: 12.0}
        spacing: 10.0
        draw_bg +: {
            color: #0000
            border_color: #0000
            border_size: 0.0
            border_radius: 0.0
        }
    }

    // ---------------------------------------------------------------
    // 字体层级：标题 24 / 18 / 15，正文 14.5，次文 13，徽章 11；段落行距 1.35。
    // 页面一律用这些预置，不再逐处写 color / font_size 字面量。
    // ---------------------------------------------------------------
    // Label 的换行默认值（right_wrap）只在首次构造时生效：换主题走 ScriptReapply 时
    // 没写的 flow 会被重置成不换行的 Right，所以每个 Label 都把 flow 明着写出来。
    mod.widgets.LiyuH1 = mod.widgets.Label{
        flow: Right{wrap: true}
        width: Fill
        draw_text +: { wrap: Words color: liyu.ink text_style +: { font_size: 24.0 line_spacing: 1.25 } }
    }
    mod.widgets.LiyuH2 = mod.widgets.Label{
        flow: Right{wrap: true}
        width: Fill
        draw_text +: { wrap: Words color: liyu.ink text_style +: { font_size: 18.0 line_spacing: 1.3 } }
    }
    mod.widgets.LiyuH3 = mod.widgets.Label{
        flow: Right{wrap: true}
        width: Fill
        draw_text +: { wrap: Words color: liyu.ink text_style +: { font_size: 15.0 line_spacing: 1.3 } }
    }
    mod.widgets.LiyuBody = mod.widgets.Label{
        flow: Right{wrap: true}
        width: Fill
        draw_text +: { wrap: Words color: liyu.ink text_style +: { font_size: 14.5 line_spacing: 1.35 } }
    }
    // 次文下限 13：05 明确「放弃旧版 11px 正文」。
    mod.widgets.LiyuMuted = mod.widgets.Label{
        flow: Right{wrap: true}
        width: Fill
        draw_text +: { wrap: Words color: liyu.ink_2 text_style +: { font_size: 13.0 line_spacing: 1.35 } }
    }
    // 11px 只留给徽章，绝不用于正文。
    mod.widgets.LiyuBadge = mod.widgets.Label{
        flow: Right{wrap: true}
        width: Fit
        draw_text +: { color: liyu.ink_2 text_style +: { font_size: 11.0 } }
    }
    mod.widgets.LiyuBadgeWarm = mod.widgets.LiyuBadge{
        draw_text +: { color: liyu.warm }
    }
    mod.widgets.LiyuBadgeBlue = mod.widgets.LiyuBadge{
        draw_text +: { color: liyu.blue }
    }
    mod.widgets.LiyuGood = mod.widgets.LiyuMuted{
        draw_text +: { color: liyu.good }
    }
    mod.widgets.LiyuBad = mod.widgets.LiyuMuted{
        draw_text +: { color: liyu.bad }
    }
    mod.widgets.LiyuWarmText = mod.widgets.LiyuMuted{
        draw_text +: { color: liyu.warm }
    }

    // ---------------------------------------------------------------
    // 图标：宿主壳同一规格（16 画布 / 1.35 单线 / 圆端点）。
    // 用法：LiyuIcon { draw_icon +: { svg: crate_resource("self:resources/icons/x.svg") } }
    // ---------------------------------------------------------------
    mod.widgets.LiyuIcon = mod.widgets.Icon{
        icon_walk: Walk{ width: 16.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: liyu.ink_2 }
    }
    mod.widgets.LiyuIconWarm = mod.widgets.LiyuIcon{
        draw_icon +: { preserve_viewbox: true color: liyu.warm }
    }
    mod.widgets.LiyuIconBlue = mod.widgets.LiyuIcon{
        draw_icon +: { preserve_viewbox: true color: liyu.blue }
    }
    mod.widgets.LiyuIconText = mod.widgets.LiyuIcon{
        draw_icon +: { preserve_viewbox: true color: liyu.ink }
    }


    // 主按钮: 操作蓝底 + 深字（12 圆角）。lg 档，触控目标 44。
    mod.widgets.LiyuBtnPrimary = mod.widgets.ButtonFlat{
        height: 44
        spacing: 6.0
        padding: Inset{left: 18.0, right: 18.0, top: 10.0, bottom: 10.0}
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 16.0 height: Fit }
        draw_bg +: {
            border_radius: r.button
            // ButtonFlat 默认带一圈 theme.beveling 的斜边，描边色取自 makepad
            // 的默认主题。实心按钮不要这圈边，否则礼遇的蓝上会压一道灰。
            border_size: 0.0
            color: liyu.blue
            color_hover: liyu.blue_hi
            color_down: liyu.blue_lo
            color_focus: liyu.blue
            color_disabled: liyu.off_bg
        }
        draw_icon +: { preserve_viewbox: true color: liyu.on_blue }
        // 不写字号就是 ButtonFlat 的默认小字，比 Sm 档还小；三档按钮字号要递减。
        draw_text +: {
            text_style +: { font_size: 14.5 }
            color: liyu.on_blue
            color_hover: liyu.on_blue
            color_down: liyu.on_blue
            color_focus: liyu.on_blue
            color_disabled: liyu.off_ink
        }
    }

    // md 档：次级位置上的主按钮。
    mod.widgets.LiyuBtnPrimarySm = mod.widgets.LiyuBtnPrimary{
        height: 36
        padding: Inset{left: 14.0, right: 14.0, top: 7.0, bottom: 7.0}
        draw_text +: { text_style +: { font_size: 13.0 } }
    }

    // 次按钮: 透明底 + 描边 + 正文字。md 档。
    mod.widgets.LiyuBtn = mod.widgets.ButtonFlat{
        height: 36
        spacing: 6.0
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 15.0 height: Fit }
        padding: Inset{left: 14.0, right: 14.0, top: 8.0, bottom: 8.0}
        draw_icon +: { preserve_viewbox: true color: liyu.ink }
        draw_bg +: {
            border_radius: r.button
            border_size: 1.0
            color: #0000
            color_hover: liyu.hl
            color_down: liyu.hl
            color_focus: #0000
            border_color: liyu.line
            border_color_hover: liyu.blue
            border_color_down: liyu.blue
            border_color_focus: liyu.blue
        }
        draw_text +: {
            text_style +: { font_size: 13.5 }
            color: liyu.ink
            color_hover: liyu.ink
            color_down: liyu.ink
            color_focus: liyu.ink
        }
    }

    // sm 档：一行里并排好几个动作时用（熟人行的「送礼 / 删除」、
    // 契约行的「确认已兑现 / 提醒 TA / 免了吧」）。md 档在手机上排不进一行。
    mod.widgets.LiyuBtnSm = mod.widgets.LiyuBtn{
        height: 28
        padding: Inset{left: 10.0, right: 10.0, top: 4.0, bottom: 4.0}
        icon_walk: Walk{ width: 13.0 height: Fit }
        draw_bg +: { border_radius: r.chip }
        draw_text +: { text_style +: { font_size: 12.5 } }
    }

    // 危险按钮: 描边 + 错误色文字（删除联系人等）。
    mod.widgets.LiyuBtnDanger = mod.widgets.LiyuBtn{
        draw_icon +: { preserve_viewbox: true color: liyu.bad }
        draw_bg +: {
            border_color: liyu.bad_line
            border_color_hover: liyu.bad
            border_color_down: liyu.bad
            border_color_focus: liyu.bad
        }
        draw_text +: {
            color: liyu.bad
            color_hover: liyu.bad
            color_down: liyu.bad
            color_focus: liyu.bad
        }
    }

    // sm 档的危险按钮，配 LiyuBtnSm 用。
    mod.widgets.LiyuBtnDangerSm = mod.widgets.LiyuBtnDanger{
        height: 28
        padding: Inset{left: 10.0, right: 10.0, top: 4.0, bottom: 4.0}
        icon_walk: Walk{ width: 13.0 height: Fit }
        draw_bg +: { border_radius: r.chip }
        draw_text +: { text_style +: { font_size: 12.5 } }
    }

    // 券卡（暖杏底）上的深色按钮。
    mod.widgets.LiyuBtnWarm = mod.widgets.ButtonFlat{
        height: 44
        spacing: 6.0
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 16.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: liyu.on_warm_btn }
        padding: Inset{left: 18.0, right: 18.0, top: 10.0, bottom: 10.0}
        draw_bg +: {
            border_radius: r.button
            border_size: 0.0
            color: liyu.warm_btn
            color_hover: liyu.warm_btn_hi
            color_down: liyu.warm_btn_hi
            color_focus: liyu.warm_btn
        }
        draw_text +: {
            text_style +: { font_size: 14.5 }
            color: liyu.on_warm_btn
            color_hover: liyu.on_warm_btn_hi
            color_down: liyu.on_warm_btn_hi
            color_focus: liyu.on_warm_btn
        }
    }

    // 侧栏 Tab: 整行宽, 选中时高亮块 + 亮字。自定义 pixel 只画圆角块,
    // 不画 CheckBoxFlat 默认的勾选框。
    mod.widgets.LiyuTab = mod.widgets.CheckBoxFlat{
        width: Fill
        height: 42
        padding: Inset{left: 14.0, right: 14.0}
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
            text_style +: { font_size: 14.0 }
        }
    }

    // 芯片: 全圆角, 选中时填充高亮块色。
    mod.widgets.LiyuChip = mod.widgets.CheckBoxFlat{
        width: Fit
        height: Fit
        padding: Inset{left: 14.0, right: 14.0, top: 8.0, bottom: 8.0}
        align: Align{x: 0.5, y: 0.5}
        label_walk +: { margin: Inset{left: 0.0} }
        draw_bg +: {
            border_size: 1.0
            color: #0000
            color_hover: liyu.hl_soft
            color_down: liyu.hl
            color_focus: #0000
            color_active: liyu.hl
            border_color: liyu.line
            border_color_hover: liyu.blue
            border_color_down: liyu.blue
            border_color_focus: liyu.blue
            border_color_active: liyu.blue
            pixel: fn(){
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let bs = self.border_size
                sdf.box(bs, bs, self.rect_size.x - bs * 2.0, self.rect_size.y - bs * 2.0, self.rect_size.y * 0.5 - bs)
                let fill = self.color
                    .mix(self.color_focus, self.focus)
                    .mix(self.color_active, self.active)
                    .mix(self.color_hover, self.hover)
                    .mix(self.color_down, self.down)
                let stroke = self.border_color
                    .mix(self.border_color_focus, self.focus)
                    .mix(self.border_color_active, self.active)
                    .mix(self.border_color_hover, self.hover)
                    .mix(self.border_color_down, self.down)
                sdf.fill_keep(fill)
                sdf.stroke(stroke, bs)
                return sdf.result
            }
        }
        draw_text +: {
            color: liyu.ink
            color_hover: liyu.ink
            color_down: liyu.ink
            color_focus: liyu.ink
            color_active: liyu.ink
            text_style +: { font_size: 13.0 }
        }
    }

    // 手机模式底部导航项: 图标在上、文字在下，等宽居中，选中时暖杏 + 高亮药丸。
    // CheckBox 的 draw_check_box 先画 draw_icon 再画 draw_text，flow: Down 即得上下结构。
    // 图标本身没有 active 通道，选中色由 sync_tabs 逐个 apply（见 lib.rs）。
    mod.widgets.LiyuNavTab = mod.widgets.CheckBoxFlat{
        width: Fill
        height: Fill
        flow: Down
        spacing: 3.0
        padding: Inset{left: 2.0, right: 2.0, top: 6.0, bottom: 5.0}
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 21.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: liyu.ink_2 }
        label_walk +: { margin: Inset{left: 0.0} }
        draw_bg +: {
            border_radius: r.button
            color: #0000
            color_hover: liyu.hl_soft
            color_down: liyu.hl
            color_focus: #0000
            color_active: #0000
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
            color_active: liyu.warm
            text_style +: { font_size: 11.5 }
        }
    }

    // 侧栏 Tab 也带图标（桌面形态）。
    mod.widgets.LiyuTabIcon = mod.widgets.LiyuTab{
        spacing: 10.0
        icon_walk: Walk{ width: 17.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: liyu.ink_2 }
    }

    // 纯图标工具按钮（关闭、返回、更多）。
    mod.widgets.LiyuIconBtn = mod.widgets.ButtonFlat{
        width: 34 height: 34
        spacing: 0.0
        text: ""
        padding: 0.0
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 16.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: liyu.ink_2 }
        draw_bg +: {
            border_radius: r.chip
            border_size: 0.0
            color: #0000
            color_hover: liyu.hl
            color_down: liyu.hl
            color_focus: #0000
        }
    }

    // 圆形「+」按钮：头部的新建 / 导入入口，点开一张菜单。
    mod.widgets.LiyuAddBtn = mod.widgets.LiyuIconBtn{
        width: 32 height: 32
        icon_walk: Walk{ width: 15.0 height: Fit }
        draw_icon +: { color: liyu.ink }
        draw_bg +: {
            border_radius: r.card
            border_size: 1.0
            border_color: liyu.line
            border_color_hover: liyu.blue
            border_color_down: liyu.blue
            border_color_focus: liyu.line
        }
    }

    // 下拉菜单面板：一张小卡，里面若干 LiyuMenuItem。
    //
    // 由调用方挂进浮层、锚在触发按钮下方。它本身只是个「面板」——定位交给
    // 外面那层。曾在熟人卡片里就地展开，那会把卡片撑高、把下面的输入框顶下去，
    // 所以改成浮层（见 lib.rs 的 ct_menu_layer）。
    mod.widgets.LiyuMenu = mod.widgets.RoundedView{
        width: 172 height: Fit
        flow: Down
        padding: 6.0
        spacing: 2.0
        draw_bg +: {
            color: liyu.card_2
            border_color: liyu.line
            border_size: 1.0
            border_radius: r.button
        }
    }

    // 菜单条目：整条可点，文字左对齐，左边留一个图标位。
    mod.widgets.LiyuMenuItem = mod.widgets.ButtonFlat{
        width: Fill height: 34
        spacing: 8.0
        align: Align{x: 0.0, y: 0.5}
        icon_walk: Walk{ width: 15.0 height: Fit }
        padding: Inset{left: 10.0, right: 14.0, top: 6.0, bottom: 6.0}
        label_walk +: { margin: Inset{left: 0.0} }
        draw_icon +: { preserve_viewbox: true color: liyu.ink_2 }
        draw_bg +: {
            border_radius: r.chip
            border_size: 0.0
            color: #0000
            color_hover: liyu.hl
            color_down: liyu.hl
            color_focus: #0000
        }
        draw_text +: {
            color: liyu.ink
            color_hover: liyu.ink
            color_down: liyu.ink
            color_focus: liyu.ink
            text_style +: { font_size: 13.0 }
        }
    }

    // 文字链：次要出口（「以后再说」这类），不与主按钮争夺注意力。
    mod.widgets.LiyuLink = mod.widgets.ButtonFlat{
        height: Fit
        spacing: 4.0
        padding: Inset{left: 2.0, right: 2.0, top: 6.0, bottom: 6.0}
        icon_walk: Walk{ width: 13.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: liyu.blue }
        draw_bg +: {
            border_size: 0.0
            border_radius: r.button
            color: #0000
            color_hover: #0000
            color_down: #0000
            color_focus: #0000
        }
        draw_text +: {
            color: liyu.blue
            color_hover: liyu.blue_soft
            color_down: liyu.blue_soft
            color_focus: liyu.blue
            text_style +: { font_size: 13.0 }
        }
    }

    // 分段控件的一段（收到的 / 送出的、折成余额 / 换一份）：等宽、方角药丸。
    mod.widgets.LiyuSeg = mod.widgets.CheckBoxFlat{
        width: Fill
        height: 36
        padding: Inset{left: 6.0, right: 6.0}
        align: Align{x: 0.5, y: 0.5}
        label_walk +: { margin: Inset{left: 0.0} }
        draw_bg +: {
            border_radius: r.chip
            color: #0000
            color_hover: liyu.hl_soft
            color_down: liyu.hl
            color_focus: #0000
            color_active: liyu.hl_active
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
            text_style +: { font_size: 14.0 }
        }
    }

    // 分段控件外框：一条内凹的轨道，里面放若干 LiyuSeg。
    mod.widgets.LiyuSegTrack = mod.widgets.RoundedView{
        width: Fill height: Fit
        flow: Right
        spacing: 4.0
        padding: 4.0
        draw_bg +: {
            color: liyu.well
            border_color: liyu.line
            border_size: 1.0
            border_radius: r.button
        }
    }


    // 列表四态（空 / 加载 / 出错 / 离线）共用的一个组件（docs/02 六·6、七.5）。
    //
    // 只有一个预设，四个态在 Rust 里换图标、颜色、两行文案和那个按钮 ——
    // 四份各写一遍的话，迟早有一页的「离线」长得和别页不一样。
    // 文案保持短（≤16 字），以便在 320pt 下用 Fit 居中也不会溢出。
    mod.widgets.LiyuEmpty = mod.widgets.View{
        width: Fill height: Fit
        flow: Down
        align: Align{x: 0.5, y: 0.5}
        spacing: 10.0
        padding: Inset{left: 12.0, right: 12.0, top: 24.0, bottom: 24.0}
        em_icon := mod.widgets.LiyuIcon {
            icon_walk: Walk{ width: 28.0 height: Fit }
            draw_icon +: { color: liyu.ink_ghost }
        }
        em_text := Label {
            flow: Right{wrap: true}
            width: Fit
            draw_text +: { color: liyu.ink_2 text_style +: { font_size: 13.0 } }
        }
        // 第二行说「为什么」和「怎么办」，普通空态用不上，默认收着。
        em_sub := Label {
            flow: Right{wrap: true}
            visible: false
            width: Fill
            margin: Inset{left: 16.0, right: 16.0}
            draw_text +: { wrap: Words color: liyu.ink_3 text_style +: { font_size: 12.0 } }
        }
        em_action := mod.widgets.LiyuBtn { visible: false width: Fit text: "" }
    }

    // 分组小标题（A、B… 字母分组、「已兑现」这类），列表里起分隔作用。
    mod.widgets.LiyuGroupHead = mod.widgets.Label{
        flow: Right{wrap: true}
        width: Fill
        margin: Inset{top: 6.0, bottom: 2.0}
        draw_text +: { wrap: Words color: liyu.ink_3 text_style +: { font_size: 12.0 } }
    }

    // 搜索框：深色内凹，占位文字说明能输入什么。
    // 输入框。TextInputFlat 的每层都有 hover / focus / down / empty /
    // disabled 五档，漏掉哪一档就从 makepad 默认主题继承哪一档 —— 之前
    // 聚焦时占位文字用的正是那套默认色，落在礼遇的深井上几乎看不见。
    // 所以下面把每一档都写死，一档不留。
    mod.widgets.LiyuInput = mod.widgets.TextInputFlat{
        width: Fill height: Fit
        margin: 0.0
        padding: Inset{left: 12.0, right: 12.0, top: 10.0, bottom: 10.0}
        empty_text: ""
        draw_bg +: {
            border_radius: r.button
            border_size: 1.0
            color: liyu.well
            color_hover: liyu.well
            color_focus: liyu.well_focus
            color_down: liyu.well_focus
            color_empty: liyu.well
            color_disabled: liyu.off_bg
            border_color: liyu.line
            border_color_hover: liyu.line_strong
            border_color_focus: liyu.blue
            border_color_down: liyu.blue
            border_color_empty: liyu.line
            border_color_disabled: liyu.line
        }
        draw_text +: {
            // 正文一律 ink；占位一律 ink_hint，聚焦也不变浅。
            color: liyu.ink
            color_hover: liyu.ink
            color_focus: liyu.ink
            color_down: liyu.ink
            color_disabled: liyu.off_ink
            color_empty: liyu.ink_hint
            color_empty_hover: liyu.ink_hint
            color_empty_focus: liyu.ink_hint
            text_style +: { font_size: 14.0 line_spacing: 1.3 }
        }
        draw_selection +: {
            color: liyu.sel
            color_hover: liyu.sel
            color_focus: liyu.sel
            color_down: liyu.sel
            color_empty: #0000
            color_disabled: #0000
        }
        draw_cursor +: { color: liyu.blue }
    }

    // 设置行：名称 + 右侧当前值 + 箭头，整行可点。
    //
    // 右侧那一格是「现在是什么样」（已开启 / 3 张可用），不是一句解释；
    // 需要解释的写在 st_sub 里，读的人不用点进去才知道这一项管什么。
    //
    // 没有行首图标：图标得在标记里写死（script_apply_eval! 的作用域里没有
    // crate_resource），一行一个预设就得复制一份，不如都不要。
    mod.widgets.LiyuSetRow = mod.widgets.View{
        width: Fill height: Fit
        flow: Overlay
        st_body := mod.widgets.View {
            width: Fill height: Fit
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            padding: Inset{left: 12.0, right: 12.0, top: 11.0, bottom: 11.0}
            spacing: 10.0
            st_col := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 0.0
                st_name := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    max_lines: 1
                    text_overflow: Ellipsis
                    text: ""
                    draw_text +: { color: liyu.ink text_style +: { font_size: 15.0 } }
                }
                st_sub := Label {
                    flow: Right{wrap: true}
                    visible: false
                    width: Fill
                    text: ""
                    draw_text +: { wrap: Words color: liyu.ink_3 text_style +: { font_size: 12.0 } }
                }
            }
            st_val := Label {
                flow: Right{wrap: true}
                width: Fit
                text: ""
                draw_text +: { color: liyu.ink_2 text_style +: { font_size: 13.0 } }
            }
            st_arrow := mod.widgets.LiyuIcon {
                icon_walk: Walk{ width: 13.0 height: Fit }
                draw_icon +: { svg: crate_resource("self:resources/icons/chevron-right.svg") color: liyu.ink_arrow }
            }
        }
        st_hit := mod.widgets.ButtonFlat {
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

    // 开关行：右边是一枚药丸开关而不是箭头。
    //
    // 开关自己不接事件（整行都是点击区），状态由 Rust 改轨道色、并在左右两枚
    // 滑块之间选一枚露出来 —— 这样「开着还是关着」只有 Settings 一个来源，
    // 不会出现界面上是开的、存下来是关的。
    //
    // 为什么是两枚而不是挪一枚：script_apply_eval! 里改不了 align/padding，
    // 而滑块的显隐是能改的。
    mod.widgets.LiyuSwitchRow = mod.widgets.View{
        width: Fill height: Fit
        flow: Overlay
        sw_body := mod.widgets.View {
            width: Fill height: Fit
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            padding: Inset{left: 12.0, right: 12.0, top: 11.0, bottom: 11.0}
            spacing: 10.0
            sw_col := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 2.0
                sw_name := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    text: ""
                    draw_text +: { wrap: Words color: liyu.ink text_style +: { font_size: 15.0 } }
                }
                sw_sub := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    text: ""
                    draw_text +: { wrap: Words color: liyu.ink_3 text_style +: { font_size: 12.0 } }
                }
            }
            sw_track := mod.widgets.RoundedView {
                width: 44 height: 26
                flow: Right
                align: Align{x: 0.0, y: 0.5}
                padding: Inset{left: 3.0, right: 3.0}
                draw_bg +: { color: liyu.track_off border_radius: 13.0 }   // 胶囊:= height/2,形状本身,不参与圆角收敛
                sw_off := mod.widgets.RoundedView {
                    width: 20 height: 20
                    draw_bg +: { color: liyu.knob border_radius: 10.0 }       // 正圆:= height/2,同上
                }
                sw_gap := mod.widgets.View { width: Fill height: Fit }
                sw_on := mod.widgets.RoundedView {
                    visible: false
                    width: 20 height: 20
                    draw_bg +: { color: liyu.knob_on border_radius: 10.0 }    // 正圆:= height/2,同上
                }
            }
        }
        sw_hit := mod.widgets.ButtonFlat {
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

    // Toast：写入成功 / 撤销提示，浮在底部导航之上。
    mod.widgets.LiyuToast = mod.widgets.RoundedView{
        visible: false
        width: Fill height: Fit
        flow: Right
        align: Align{x: 0.0, y: 0.5}
        spacing: 8.0
        padding: Inset{left: 14.0, right: 10.0, top: 10.0, bottom: 10.0}
        draw_bg +: {
            color: liyu.raise
            border_color: liyu.line_strong
            border_size: 1.0
            border_radius: r.button
        }
        to_text := Label {
            flow: Right{wrap: true}
            width: Fill
            max_lines: 1
            text_overflow: Ellipsis
            text: ""
            draw_text +: { color: liyu.ink text_style +: { font_size: 13.0 } }
        }
        to_undo := mod.widgets.LiyuBtn {
            width: Fit height: 30
            text: "撤销"
            padding: Inset{left: 12.0, right: 12.0, top: 5.0, bottom: 5.0}
            icon_walk: Walk{ width: 13.0 height: Fit }
            draw_icon +: { preserve_viewbox: true svg: crate_resource("self:resources/icons/undo.svg") color: liyu.blue }
            draw_text +: { color: liyu.blue color_hover: liyu.blue color_down: liyu.blue color_focus: liyu.blue }
        }
    }

    // 神秘礼卡预览（3:4）：与 share.rs 的 SVG 同一张版式，按卡片尺寸等比缩放。
    // 两套色板都是 live 字段，样式切换只是换一组颜色。
    mod.widgets.LiyuShareCardBase = #(LiyuShareCard::register_widget(vm))
    mod.widgets.LiyuShareCard = set_type_default() do mod.widgets.LiyuShareCardBase{
        width: 270
        height: 360
        warm_bg: #f3d9b4
        warm_fg: #43331f
        warm_sub: #7a6647
        warm_accent: #43331f
        warm_ring: #d9bd93
        night_bg: #0e1830
        night_fg: #e7edf8
        night_sub: #a4b2c9
        night_accent: #ffca91
        night_ring: #26375c
        draw_bg +: { radius: r.card }
        draw_ring +: { color: #0000 border_size: 1.5 radius: 500.0 }
        draw_rule +: { radius: 0.0 }
        draw_head +: { text_style: theme.font_regular{ font_size: 12.0 } }
        draw_text +: { text_style: theme.font_regular{ font_size: 16.0 } }
        draw_big +: { text_style: theme.font_bold{ font_size: 60.0 } }
    }

    // 挑礼页的一行：品类首字方块 + 礼物名 / 形态·规格 + 价格，整行可点。
    //
    // 同 LiyuSetRow 的 Overlay 套路：下层 gr_body 只管显示，上层 gr_hit 是铺满的
    // 透明按钮接点击。两行文字竖直内边距清零（Label 默认四边各 3，两行白吃 12）。
    mod.widgets.LiyuGiftRow = mod.widgets.View{
        width: Fill height: 64
        flow: Overlay
        gr_body := mod.widgets.View {
            width: Fill height: Fill
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            padding: Inset{left: 12.0, right: 12.0}
            spacing: 12.0
            gr_face := mod.widgets.RoundedView {
                width: 40 height: 40
                flow: Down
                align: Align{x: 0.5, y: 0.5}
                draw_bg +: { color: liyu.face border_radius: r.card }
                gr_letter := Label {
                    flow: Right{wrap: true}
                    text: ""
                    draw_text +: { color: liyu.warm text_style +: { font_size: 15.0 } }
                }
            }
            gr_col := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 2.0
                gr_name := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    max_lines: 1
                    text_overflow: Ellipsis
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: ""
                    draw_text +: { color: liyu.ink text_style +: { font_size: 15.0 } }
                }
                gr_sub := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    max_lines: 1
                    text_overflow: Ellipsis
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: ""
                    draw_text +: { color: liyu.ink_2 text_style +: { font_size: 12.0 } }
                }
            }
            gr_price := Label {
                flow: Right{wrap: true}
                width: Fit
                text: ""
                draw_text +: { color: liyu.ink text_style +: { font_size: 15.0 } }
            }
        }
        gr_hit := mod.widgets.ButtonFlat {
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

    // 礼盒里的一份礼物：方块（未揭晓是「?」，揭晓后是品类首字）+ 标题 / 副行 + 状态。
    //
    // 状态的颜色（待办蓝 / 完成绿 / 退回灰）由 Rust 按 Gift::tone 改。
    mod.widgets.LiyuBoxRow = mod.widgets.View{
        width: Fill height: 64
        flow: Overlay
        bx_body := mod.widgets.View {
            width: Fill height: Fill
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            padding: Inset{left: 12.0, right: 12.0}
            spacing: 12.0
            bx_face := mod.widgets.RoundedView {
                width: 40 height: 40
                flow: Down
                align: Align{x: 0.5, y: 0.5}
                draw_bg +: { color: liyu.face border_radius: r.card }
                bx_letter := Label {
                    flow: Right{wrap: true}
                    text: "?"
                    draw_text +: { color: liyu.warm text_style +: { font_size: 15.0 } }
                }
            }
            bx_col := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 2.0
                bx_title := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    max_lines: 1
                    text_overflow: Ellipsis
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: ""
                    draw_text +: { color: liyu.ink text_style +: { font_size: 15.0 } }
                }
                bx_sub := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    max_lines: 1
                    text_overflow: Ellipsis
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: ""
                    draw_text +: { color: liyu.ink_2 text_style +: { font_size: 12.0 } }
                }
            }
            bx_state := Label {
                flow: Right{wrap: true}
                width: Fit
                text: ""
                draw_text +: { color: liyu.blue text_style +: { font_size: 12.5 } }
            }
        }
        bx_hit := mod.widgets.ButtonFlat {
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

    // 契约的一条：契约文字 + 谁对谁 · 源自哪份礼物 · 到期 + 动作按钮。
    //
    // 竖排放在 pc_col 这一层：预设自己的 flow: Down 到不了实例。
    mod.widgets.LiyuPactRow = mod.widgets.View{
        width: Fill height: Fit
        pc_col := mod.widgets.View {
            width: Fill height: Fit
            flow: Down
            spacing: 4.0
            padding: Inset{left: 12.0, right: 12.0, top: 10.0, bottom: 10.0}
            pc_text := Label {
                flow: Right{wrap: true}
                width: Fill
                text: ""
                draw_text +: { wrap: Words color: liyu.ink text_style +: { font_size: 15.0 line_spacing: 1.3 } }
            }
            pc_sub := Label {
                flow: Right{wrap: true}
                width: Fill
                text: ""
                draw_text +: { wrap: Words color: liyu.ink_3 text_style +: { font_size: 12.0 } }
            }
            pc_acts := mod.widgets.View {
                width: Fill height: Fit
                flow: Right{wrap: true}
                wrap_spacing: 6.0
                spacing: 8.0
                margin: Inset{top: 4.0}
                pc_done := mod.widgets.LiyuBtnSm { width: Fit text: "标记已兑现" }
                pc_nudge := mod.widgets.LiyuBtnSm { width: Fit text: "提醒 TA" }
                pc_waive := mod.widgets.LiyuBtnSm { width: Fit text: "免了吧" }
            }
        }
    }

    // 钱包流水的一行：说明 / 日期 + 右侧金额（入账绿、支出正文色，由 Rust 改）。
    mod.widgets.LiyuLedgerRow = mod.widgets.View{
        width: Fill height: Fit
        flow: Right
        align: Align{x: 0.0, y: 0.5}
        padding: Inset{left: 12.0, right: 12.0, top: 9.0, bottom: 9.0}
        spacing: 10.0
        ld_col := mod.widgets.View {
            width: Fill height: Fit
            flow: Down
            spacing: 2.0
            ld_text := Label {
                flow: Right{wrap: true}
                width: Fill
                max_lines: 1
                text_overflow: Ellipsis
                padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                text: ""
                draw_text +: { color: liyu.ink text_style +: { font_size: 14.0 } }
            }
            ld_date := Label {
                flow: Right{wrap: true}
                width: Fill
                padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                text: ""
                draw_text +: { color: liyu.ink_3 text_style +: { font_size: 12.0 } }
            }
        }
        ld_amt := Label {
            flow: Right{wrap: true}
            width: Fit
            text: ""
            draw_text +: { color: liyu.ink text_style +: { font_size: 15.0 } }
        }
    }

    // 时间线的一步：空心点（最新一步是实心暖杏）+ 日期 + 事件。
    //
    // Icon 没有 visible，实心 / 空心两枚各套一层 View，由 Rust 选一枚露出来。
    mod.widgets.LiyuStep = mod.widgets.View{
        width: Fill height: Fit
        flow: Right
        align: Align{x: 0.0, y: 0.5}
        spacing: 10.0
        padding: Inset{top: 4.0, bottom: 4.0}
        tl_on := mod.widgets.View {
            visible: false
            width: Fit height: Fit
            tl_on_icon := mod.widgets.LiyuIconWarm {
                icon_walk: Walk{ width: 10.0 height: Fit }
                draw_icon +: { svg: crate_resource("self:resources/icons/dot.svg") }
            }
        }
        tl_off := mod.widgets.View {
            width: Fit height: Fit
            tl_off_icon := mod.widgets.LiyuIcon {
                icon_walk: Walk{ width: 10.0 height: Fit }
                draw_icon +: { svg: crate_resource("self:resources/icons/dot-hollow.svg") color: liyu.ink_3 }
            }
        }
        tl_date := Label {
            flow: Right{wrap: true}
            width: 52
            text: ""
            draw_text +: { color: liyu.ink_3 text_style +: { font_size: 12.0 } }
        }
        tl_text := Label {
            flow: Right{wrap: true}
            width: Fill
            text: ""
            draw_text +: { wrap: Words color: liyu.ink text_style +: { font_size: 13.5 } }
        }
    }

    // 换购的一个选项：品类首字 + 礼物名 / 规格 + 右侧「退 ¥x / 补 ¥y」+ 选中勾。
    mod.widgets.LiyuChoice = mod.widgets.View{
        width: Fill height: 56
        flow: Overlay
        ch_body := mod.widgets.View {
            width: Fill height: Fill
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            padding: Inset{left: 10.0, right: 12.0}
            spacing: 10.0
            ch_face := mod.widgets.RoundedView {
                width: 34 height: 34
                flow: Down
                align: Align{x: 0.5, y: 0.5}
                draw_bg +: { color: liyu.face border_radius: r.button }
                ch_letter := Label {
                    flow: Right{wrap: true}
                    text: ""
                    draw_text +: { color: liyu.warm text_style +: { font_size: 13.0 } }
                }
            }
            ch_col := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 1.0
                ch_name := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    max_lines: 1
                    text_overflow: Ellipsis
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: ""
                    draw_text +: { color: liyu.ink text_style +: { font_size: 14.0 } }
                }
                ch_sub := Label {
                    flow: Right{wrap: true}
                    width: Fill
                    max_lines: 1
                    text_overflow: Ellipsis
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: ""
                    draw_text +: { color: liyu.ink_3 text_style +: { font_size: 11.5 } }
                }
            }
            ch_diff := Label {
                flow: Right{wrap: true}
                width: Fit
                text: ""
                draw_text +: { color: liyu.ink_2 text_style +: { font_size: 13.0 } }
            }
            ch_tick := mod.widgets.View {
                visible: false
                width: Fit height: Fit
                ch_tick_icon := mod.widgets.LiyuIconBlue {
                    icon_walk: Walk{ width: 15.0 height: Fit }
                    draw_icon +: { svg: crate_resource("self:resources/icons/check.svg") }
                }
            }
        }
        ch_hit := mod.widgets.ButtonFlat {
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

    // 熟人的一行：首字方块 + 姓名 / 「送过 N 份 · 收到 N 份」+「送礼」「删除」。
    //
    // 首字而不是头像：这一页本来就只列本机通讯录里的人，首字已经够认。
    mod.widgets.LiyuContactRow = mod.widgets.View{
        width: Fill height: 60
        flow: Right
        align: Align{x: 0.0, y: 0.5}
        padding: Inset{left: 12.0, right: 12.0}
        spacing: 12.0
        cr_face := mod.widgets.RoundedView {
            width: 36 height: 36
            flow: Down
            align: Align{x: 0.5, y: 0.5}
            draw_bg +: { color: liyu.face border_radius: r.card }
            cr_initial := Label {
                flow: Right{wrap: true}
                text: ""
                draw_text +: { color: liyu.blue text_style +: { font_size: 15.0 } }
            }
        }
        cr_col := mod.widgets.View {
            width: Fill height: Fit
            flow: Down
            spacing: 2.0
            cr_name := Label {
                flow: Right{wrap: true}
                width: Fill
                max_lines: 1
                text_overflow: Ellipsis
                padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                text: ""
                draw_text +: { color: liyu.ink text_style +: { font_size: 15.0 } }
            }
            cr_sub := Label {
                flow: Right{wrap: true}
                width: Fill
                max_lines: 1
                text_overflow: Ellipsis
                padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                text: ""
                draw_text +: { color: liyu.ink_3 text_style +: { font_size: 12.0 } }
            }
        }
        cr_send := mod.widgets.LiyuBtnSm { width: Fit text: "送礼" }
        cr_del := mod.widgets.LiyuBtnDangerSm { width: Fit text: "删除" }
    }

    // 揭晓后的礼物卡：暖杏底，形态·品类 / 礼物名 / 规格 / 来自谁 · 寄语。
    //
    // 送礼时的价格不上这张卡 —— 收礼人看到的是礼物，不是标价；
    // 换购 / 折现时才在计算那一栏里出现金额。
    mod.widgets.LiyuGiftCard = mod.widgets.RoundedView{
        width: Fill height: Fit
        flow: Down
        padding: 18.0
        spacing: 6.0
        draw_bg +: { color: liyu.coupon border_radius: r.card }
        gc_kind := Label {
            flow: Right{wrap: true}
            width: Fill
            text: ""
            draw_text +: { color: liyu.on_warm text_style +: { font_size: 12.0 } }
        }
        gc_name := Label {
            flow: Right{wrap: true}
            width: Fill
            text: ""
            draw_text +: { wrap: Words color: liyu.warm_btn text_style +: { font_size: 22.0 line_spacing: 1.25 } }
        }
        gc_spec := Label {
            flow: Right{wrap: true}
            width: Fill
            text: ""
            draw_text +: { wrap: Words color: liyu.on_warm text_style +: { font_size: 13.0 } }
        }
        gc_from := Label {
            flow: Right{wrap: true}
            width: Fill
            margin: Inset{top: 4.0}
            text: ""
            draw_text +: { wrap: Words color: liyu.on_warm text_style +: { font_size: 13.0 line_spacing: 1.35 } }
        }
    }
}

/// 圆角矩形 shader：颜色/圆角/描边都是 instance 字段，逐次绘制可改。
#[derive(Script, ScriptHook)]
#[repr(C)]
pub struct DrawRoundBox {
    #[deref]
    draw_super: DrawQuad,
    #[live]
    pub color: Vec4f,
    #[live(4.0)]
    pub radius: f32,
    #[live]
    pub border_color: Vec4f,
    #[live(0.0)]
    pub border_size: f32,
    #[live(1.0)]
    pub alpha_scale: f32,
}

// ---- 神秘礼卡预览（3:4，暖杏 / 夜蓝两样式；内容边界同 share.rs 场景）----

#[derive(Script, ScriptHook, Widget)]
pub struct LiyuShareCard {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,

    #[redraw]
    #[live]
    draw_bg: DrawRoundBox,
    #[live]
    draw_ring: DrawRoundBox,
    #[live]
    draw_rule: DrawRoundBox,
    #[live]
    draw_head: DrawText,
    #[live]
    draw_text: DrawText,
    #[live]
    draw_big: DrawText,

    #[live]
    warm_bg: Vec4f,
    #[live]
    warm_fg: Vec4f,
    #[live]
    warm_sub: Vec4f,
    #[live]
    warm_accent: Vec4f,
    #[live]
    warm_ring: Vec4f,
    #[live]
    night_bg: Vec4f,
    #[live]
    night_fg: Vec4f,
    #[live]
    night_sub: Vec4f,
    #[live]
    night_accent: Vec4f,
    #[live]
    night_ring: Vec4f,

    #[rust]
    scene: ShareCardScene,
}

impl LiyuShareCard {
    pub fn set_scene(&mut self, scene: ShareCardScene) {
        self.scene = scene;
    }
}

/// SVG 的字号是像素、文字按基线定位；DrawText 的字号是磅（×4/3 落到像素）、
/// 按左上角定位。换算：磅 = 像素 × 0.75，左上 y ≈ 基线 − 0.86 × 字高。
const PT_PER_PX: f64 = 0.75;
const ASCENT: f64 = 0.86;

impl Widget for LiyuShareCard {
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        let r = cx.walk_turtle(walk);
        if r.size.x < 40.0 || r.size.y < 40.0 {
            return DrawStep::done();
        }
        // 布局直接按 900×1200 海报坐标等比缩放（与 share.rs 的 SVG 一致）。
        let sx = r.size.x / 900.0;
        let sy = r.size.y / 1200.0;
        let px = move |x: f64| r.pos.x + x * sx;
        let py = move |y: f64| r.pos.y + y * sy;
        let (bg, fg, sub, accent, ring) = match self.scene.style {
            ShareStyle::Warm => (self.warm_bg, self.warm_fg, self.warm_sub, self.warm_accent, self.warm_ring),
            ShareStyle::Night => (self.night_bg, self.night_fg, self.night_sub, self.night_accent, self.night_ring),
        };
        self.draw_bg.color = bg;
        self.draw_bg.draw_abs(cx, r);

        // 礼盒徽记：两道圆环 + 中间一个问号。
        self.draw_ring.border_color = ring;
        for rad in [170.0, 120.0] {
            let rr = rad * (sx + sy) * 0.5;
            self.draw_ring.draw_abs(
                cx,
                Rect { pos: dvec2(px(450.0) - rr, py(330.0) - rr), size: dvec2(rr * 2.0, rr * 2.0) },
            );
        }
        self.draw_big.color = accent;
        self.draw_big.text_style.font_size = (170.0 * sy * PT_PER_PX).max(5.0) as f32;
        self.draw_big.draw_abs(cx, dvec2(px(450.0 - 170.0 * 0.28), py(390.0 - 170.0 * ASCENT)), "?");

        let scene = self.scene.clone();
        let line = |cx: &mut Cx2d, d: &mut DrawText, x: f64, y: f64, size: f64, color: Vec4f, text: &str| {
            d.color = color;
            d.text_style.font_size = (size * sy * PT_PER_PX).max(4.0) as f32;
            d.draw_abs(cx, dvec2(px(x), py(y - size * ASCENT)), text);
        };
        line(cx, &mut self.draw_head, 64.0, 92.0, 22.0, sub, "LIYU / 神秘礼卡");
        line(cx, &mut self.draw_text, 64.0, 600.0, 52.0, fg, "一份神秘礼物 · 等你来拆");
        line(cx, &mut self.draw_head, 64.0, 656.0, 26.0, sub, &scene.play);
        if !scene.prompt_lines.is_empty() {
            let mut y = 740.0;
            line(cx, &mut self.draw_head, 64.0, y, 24.0, sub, &scene.prompt_title);
            y += 58.0;
            for l in &scene.prompt_lines {
                line(cx, &mut self.draw_text, 64.0, y, 40.0, fg, l);
                y += 54.0;
            }
        }
        line(cx, &mut self.draw_head, 64.0, 950.0, 24.0, sub, "口令");
        line(cx, &mut self.draw_big, 64.0, 1030.0, 84.0, accent, &scene.code);
        self.draw_rule.color = ring;
        self.draw_rule.draw_abs(
            cx,
            Rect { pos: dvec2(px(64.0), py(1080.0)), size: dvec2(px(836.0) - px(64.0), 1.0) },
        );
        line(cx, &mut self.draw_head, 64.0, 1130.0, 26.0, fg, "礼遇 LiYu");
        line(cx, &mut self.draw_head, 530.0, 1130.0, 18.0, sub, "礼遇 · 礼盒 · 输入口令");
        DrawStep::done()
    }

    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) {}
}
