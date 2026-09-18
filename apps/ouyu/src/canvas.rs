//! 偶遇的自绘画布组件：圆角矩形 shader、匿名机会街区插图。
//!
//! 新版设计（05-visual-style）：地图只是街区插图——没有人形光斑、头像、姓名、
//! 实时坐标，也不用脉冲 / 呼吸动画暗示有人移动。中央只有一个静态虚线圆环、
//! 一点暖杏光点和「给偶然留一点可能 / 区域机会 · 不知道是谁」。
use crate::share::{ShareCardScene, ShareStyle};
use makepad_widgets::*;

script_mod! {
    use mod.prelude.ouyu.*
    use mod.widgets.*

    // 圆角矩形 + 可选描边: 卡片骨架、地图光圈都靠它。
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

    mod.widgets.OuyuMapBase = #(OuyuMap::register_widget(vm))

    // 匿名机会街区插图: 夜光城市示意底图 + 静态虚线圆环 + 暖杏光点。无人。
    mod.widgets.OuyuMap = set_type_default() do mod.widgets.OuyuMapBase{
        width: Fill
        height: 300

        draw_bg +: { color: ouyu.map_bg draw_depth: 0.0 }
        draw_road +: { color: ouyu.map_road draw_depth: 0.5 }
        draw_block +: { color: ouyu.map_block radius: 4.0 draw_depth: 1.0 }
        draw_glow +: { color: ouyu.map_glow radius: 500.0 alpha_scale: 0.05 draw_depth: 1.5 }
        draw_ring +: { color: ouyu.warm radius: 500.0 alpha_scale: 0.55 draw_depth: 2.0 }
        draw_dot +: { color: ouyu.warm radius: 500.0 draw_depth: 2.5 }
        draw_label +: {
            color: ouyu.ink_4
            draw_depth: 3.0
            text_style: theme.font_regular{ font_size: 10.0 }
        }
        draw_title +: {
            color: ouyu.warm
            draw_depth: 3.0
            text_style: theme.font_regular{ font_size: 13.0 }
        }
        draw_sub +: {
            color: ouyu.ink_2
            draw_depth: 3.0
            text_style: theme.font_regular{ font_size: 11.0 }
        }
        draw_hint +: {
            color: ouyu.ink_4
            draw_depth: 3.0
            text_style: theme.font_regular{ font_size: 10.0 }
        }
    }

    // 滚动条：手柄颜色也从色板取。
    //
    // makepad 的 ScrollBar 默认取 `theme.color_outset` —— 那是 widget 主题的
    // 颜色，宿主的样式表说了算，不跟着偶遇的深浅走。白昼版跑在夜色宿主里时，
    // 它会在白纸上画一条深蓝手柄（独立窗口下反过来：白色 10% 几乎看不见）。
    // 每个 ScrollYView 都换成这个预设，深浅两套都由 `ouyu.bar` 决定。
    mod.widgets.OuyuScrollBar = mod.widgets.ScrollBar {
        draw_bg +: {
            color: ouyu.bar
            color_hover: ouyu.bar_hi
            color_drag: ouyu.bar_drag
        }
    }

    // 和 makepad 的 ScrollYView 一样的一份，只换掉手柄。
    mod.widgets.OuyuScrollY = mod.widgets.ViewBase {
        scroll_bars: mod.widgets.ScrollBars {
            show_scroll_x: false
            show_scroll_y: true
            scroll_bar_y: mod.widgets.OuyuScrollBar { drag_scrolling: true }
        }
    }

    // 卡片分三级（docs/02 六·1）：主卡 / 次卡 / 内联行。
    // 主卡: 20 圆角 + 1px 描边 + 略提亮底，用于页面核心内容。
    mod.widgets.OuyuCard = mod.widgets.RoundedView{
        width: Fill height: Fit
        flow: Down
        padding: 18.0
        spacing: 12.0
        draw_bg +: {
            color: ouyu.card
            border_color: ouyu.line_soft
            border_size: 1.0
            border_radius: r.card
        }
    }

    // 次卡: 无描边、更暗、更小圆角，用于附属信息。
    mod.widgets.OuyuCard2 = mod.widgets.RoundedView{
        width: Fill height: Fit
        flow: Down
        padding: 14.0
        spacing: 10.0
        draw_bg +: {
            color: ouyu.card_2
            border_color: #0000
            border_size: 0.0
            border_radius: r.card
        }
    }

    // 内联行: 无底色，底部一条分隔线（列表项）。
    mod.widgets.OuyuRow = mod.widgets.RoundedView{
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
    mod.widgets.OuyuH1 = mod.widgets.Label{
        width: Fill
        draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 24.0 line_spacing: 1.25 } }
    }
    mod.widgets.OuyuH2 = mod.widgets.Label{
        width: Fill
        draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 18.0 line_spacing: 1.3 } }
    }
    mod.widgets.OuyuH3 = mod.widgets.Label{
        width: Fill
        draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 15.0 line_spacing: 1.3 } }
    }
    mod.widgets.OuyuBody = mod.widgets.Label{
        width: Fill
        draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 14.5 line_spacing: 1.35 } }
    }
    // 次文下限 13：05 明确「放弃旧版 11px 正文」。
    mod.widgets.OuyuMuted = mod.widgets.Label{
        width: Fill
        draw_text +: { wrap: Words color: ouyu.ink_2 text_style +: { font_size: 13.0 line_spacing: 1.35 } }
    }
    // 11px 只留给徽章，绝不用于正文。
    mod.widgets.OuyuBadge = mod.widgets.Label{
        width: Fit
        draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 11.0 } }
    }
    mod.widgets.OuyuBadgeWarm = mod.widgets.OuyuBadge{
        draw_text +: { color: ouyu.warm }
    }
    mod.widgets.OuyuBadgeBlue = mod.widgets.OuyuBadge{
        draw_text +: { color: ouyu.blue }
    }
    mod.widgets.OuyuGood = mod.widgets.OuyuMuted{
        draw_text +: { color: ouyu.good }
    }
    mod.widgets.OuyuBad = mod.widgets.OuyuMuted{
        draw_text +: { color: ouyu.bad }
    }
    mod.widgets.OuyuWarmText = mod.widgets.OuyuMuted{
        draw_text +: { color: ouyu.warm }
    }

    // ---------------------------------------------------------------
    // 图标：宿主壳同一规格（16 画布 / 1.35 单线 / 圆端点）。
    // 用法：OuyuIcon { draw_icon +: { svg: crate_resource("self:resources/icons/x.svg") } }
    // ---------------------------------------------------------------
    mod.widgets.OuyuIcon = mod.widgets.Icon{
        icon_walk: Walk{ width: 16.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: ouyu.ink_2 }
    }
    mod.widgets.OuyuIconWarm = mod.widgets.OuyuIcon{
        draw_icon +: { preserve_viewbox: true color: ouyu.warm }
    }
    mod.widgets.OuyuIconBlue = mod.widgets.OuyuIcon{
        draw_icon +: { preserve_viewbox: true color: ouyu.blue }
    }
    mod.widgets.OuyuIconText = mod.widgets.OuyuIcon{
        draw_icon +: { preserve_viewbox: true color: ouyu.ink }
    }

    // 折线段：SDF 胶囊，转角圆润（参考 makepad finance 图表，去掉 glow）。
    set_type_default() do #(DrawLineSeg::script_shader(vm)){
        ..mod.draw.DrawQuad
        pixel: fn() {
            let p = self.pos * self.rect_size
            let a = vec2(0.0, self.y0 * self.rect_size.y)
            let b = vec2(self.rect_size.x, self.y1 * self.rect_size.y)
            let pa = p - a
            let ba = b - a
            let h = clamp(dot(pa, ba) / max(dot(ba, ba), 0.0001), 0.0, 1.0)
            let d = length(pa - ba * h)
            let half = self.thickness * 0.5
            let line = clamp(1.0 - smoothstep(half - 0.75, half + 0.75, d), 0.0, 1.0)
            let alpha = line * self.color_line.a
            return vec4(self.color_line.rgb * alpha, alpha)
        }
    }

    // 曲线下的浅填充：每个区间一个梯形，向基线渐隐。
    set_type_default() do #(DrawAreaFill::script_shader(vm)){
        ..mod.draw.DrawQuad
        pixel: fn() {
            let top = mix(self.top_left, self.top_right, self.pos.x)
            let y = self.pos.y
            let feather = 1.0 / max(self.rect_size.y, 1.0)
            let inside = smoothstep(top - feather, top + feather, y)
            if inside <= 0.0 {
                return #0000
            }
            let depth = (y - top) / max(1.0 - top, 0.001)
            let strength = clamp(1.0 - depth, 0.0, 1.0)
            let alpha = inside * strength * strength * self.color_top.a
            return vec4(self.color_top.rgb * alpha, alpha)
        }
    }

    // 相遇频率曲线：纵轴从零开始、整数刻度、首尾日期、末周标注。
    mod.widgets.OuyuChartBase = #(OuyuChart::register_widget(vm))
    mod.widgets.OuyuChart = set_type_default() do mod.widgets.OuyuChartBase{
        width: Fill
        height: 220
        draw_area +: { color_top: ouyu.warm_wash }
        draw_line +: { color_line: ouyu.warm thickness: 2.5 }
        draw_dot +: { color: ouyu.warm radius: 500.0 }
        draw_rule +: { color: ouyu.line }
        draw_text +: {
            color: ouyu.ink_4
            text_style: theme.font_regular{ font_size: 10.0 }
        }
    }

    // 分享卡预览（3:4）：装饰圆环 + 汇总次数 + 成就文案 + 署名。
    // 两套色板都是 live 字段，样式切换只是换一组颜色。
    mod.widgets.OuyuShareCardBase = #(OuyuShareCard::register_widget(vm))
    mod.widgets.OuyuShareCard = set_type_default() do mod.widgets.OuyuShareCardBase{
        width: 300
        height: 400
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
        draw_dot +: { radius: 500.0 }
        draw_line +: { thickness: 3.0 }
        draw_head +: { text_style: theme.font_regular{ font_size: 12.0 } }
        draw_text +: { text_style: theme.font_regular{ font_size: 16.0 } }
        draw_big +: { text_style: theme.font_regular{ font_size: 120.0 } }
    }

    // 主按钮: 操作蓝底 + 深字（12 圆角）。lg 档，触控目标 44。
    mod.widgets.OuyuBtnPrimary = mod.widgets.ButtonFlat{
        height: 44
        spacing: 6.0
        padding: Inset{left: 18.0, right: 18.0, top: 10.0, bottom: 10.0}
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 16.0 height: Fit }
        draw_bg +: {
            border_radius: r.button
            // ButtonFlat 默认带一圈 theme.beveling 的斜边，描边色取自 makepad
            // 的默认主题。实心按钮不要这圈边，否则偶遇的蓝上会压一道灰。
            border_size: 0.0
            color: ouyu.blue
            color_hover: ouyu.blue_hi
            color_down: ouyu.blue_lo
            color_focus: ouyu.blue
            color_disabled: ouyu.off_bg
        }
        draw_icon +: { preserve_viewbox: true color: ouyu.on_blue }
        draw_text +: {
            color: ouyu.on_blue
            color_hover: ouyu.on_blue
            color_down: ouyu.on_blue
            color_focus: ouyu.on_blue
            color_disabled: ouyu.off_ink
        }
    }

    // md 档：次级位置上的主按钮。
    mod.widgets.OuyuBtnPrimarySm = mod.widgets.OuyuBtnPrimary{
        height: 36
        padding: Inset{left: 14.0, right: 14.0, top: 7.0, bottom: 7.0}
        draw_text +: { text_style +: { font_size: 13.0 } }
    }

    // 次按钮: 透明底 + 描边 + 正文字。md 档。
    mod.widgets.OuyuBtn = mod.widgets.ButtonFlat{
        height: 36
        spacing: 6.0
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 15.0 height: Fit }
        padding: Inset{left: 14.0, right: 14.0, top: 8.0, bottom: 8.0}
        draw_icon +: { preserve_viewbox: true color: ouyu.ink }
        draw_bg +: {
            border_radius: r.button
            border_size: 1.0
            color: #0000
            color_hover: ouyu.hl
            color_down: ouyu.hl
            color_focus: #0000
            border_color: ouyu.line
            border_color_hover: ouyu.blue
            border_color_down: ouyu.blue
            border_color_focus: ouyu.blue
        }
        draw_text +: {
            color: ouyu.ink
            color_hover: ouyu.ink
            color_down: ouyu.ink
            color_focus: ouyu.ink
        }
    }

    // sm 档：一行里并排好几个动作时用（熟人行的「回忆 / 合并 / 清空回忆 /
    // 删除」）。手机上四个 md 按钮排不进一行，只好把行拆成三行 —— 一个熟人
    // 占三行，一屏看不到几个人。
    mod.widgets.OuyuBtnSm = mod.widgets.OuyuBtn{
        height: 28
        padding: Inset{left: 10.0, right: 10.0, top: 4.0, bottom: 4.0}
        icon_walk: Walk{ width: 13.0 height: Fit }
        draw_bg +: { border_radius: r.chip }
        draw_text +: { text_style +: { font_size: 12.5 } }
    }

    // 危险按钮: 描边 + 错误色文字（删除联系人等）。
    mod.widgets.OuyuBtnDanger = mod.widgets.OuyuBtn{
        draw_icon +: { preserve_viewbox: true color: ouyu.bad }
        draw_bg +: {
            border_color: ouyu.bad_line
            border_color_hover: ouyu.bad
            border_color_down: ouyu.bad
            border_color_focus: ouyu.bad
        }
        draw_text +: {
            color: ouyu.bad
            color_hover: ouyu.bad
            color_down: ouyu.bad
            color_focus: ouyu.bad
        }
    }

    // sm 档的危险按钮，配 OuyuBtnSm 用。
    mod.widgets.OuyuBtnDangerSm = mod.widgets.OuyuBtnDanger{
        height: 28
        padding: Inset{left: 10.0, right: 10.0, top: 4.0, bottom: 4.0}
        icon_walk: Walk{ width: 13.0 height: Fit }
        draw_bg +: { border_radius: r.chip }
        draw_text +: { text_style +: { font_size: 12.5 } }
    }

    // 券卡（暖杏底）上的深色按钮。
    mod.widgets.OuyuBtnWarm = mod.widgets.ButtonFlat{
        height: 44
        spacing: 6.0
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 16.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: ouyu.on_warm_btn }
        padding: Inset{left: 18.0, right: 18.0, top: 10.0, bottom: 10.0}
        draw_bg +: {
            border_radius: r.button
            border_size: 0.0
            color: ouyu.warm_btn
            color_hover: ouyu.warm_btn_hi
            color_down: ouyu.warm_btn_hi
            color_focus: ouyu.warm_btn
        }
        draw_text +: {
            color: ouyu.on_warm_btn
            color_hover: ouyu.on_warm_btn_hi
            color_down: ouyu.on_warm_btn_hi
            color_focus: ouyu.on_warm_btn
        }
    }

    // 侧栏 Tab: 整行宽, 选中时高亮块 + 亮字。自定义 pixel 只画圆角块,
    // 不画 CheckBoxFlat 默认的勾选框。
    mod.widgets.OuyuTab = mod.widgets.CheckBoxFlat{
        width: Fill
        height: 42
        padding: Inset{left: 14.0, right: 14.0}
        align: Align{x: 0.0, y: 0.5}
        label_walk +: { margin: Inset{left: 0.0} }
        draw_bg +: {
            border_radius: r.button
            color: #0000
            color_hover: ouyu.hl_soft
            color_down: ouyu.hl
            color_focus: #0000
            color_active: ouyu.hl
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
            color: ouyu.ink_2
            color_hover: ouyu.ink
            color_down: ouyu.ink
            color_focus: ouyu.ink_2
            color_active: ouyu.ink
            text_style +: { font_size: 14.0 }
        }
    }

    // 芯片: 全圆角, 选中时填充高亮块色。
    mod.widgets.OuyuChip = mod.widgets.CheckBoxFlat{
        width: Fit
        height: Fit
        padding: Inset{left: 14.0, right: 14.0, top: 8.0, bottom: 8.0}
        align: Align{x: 0.5, y: 0.5}
        label_walk +: { margin: Inset{left: 0.0} }
        draw_bg +: {
            border_size: 1.0
            color: #0000
            color_hover: ouyu.hl_soft
            color_down: ouyu.hl
            color_focus: #0000
            color_active: ouyu.hl
            border_color: ouyu.line
            border_color_hover: ouyu.blue
            border_color_down: ouyu.blue
            border_color_focus: ouyu.blue
            border_color_active: ouyu.blue
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
            color: ouyu.ink
            color_hover: ouyu.ink
            color_down: ouyu.ink
            color_focus: ouyu.ink
            color_active: ouyu.ink
            text_style +: { font_size: 13.0 }
        }
    }

    // 手机模式底部导航项: 图标在上、文字在下，等宽居中，选中时暖杏 + 高亮药丸。
    // CheckBox 的 draw_check_box 先画 draw_icon 再画 draw_text，flow: Down 即得上下结构。
    // 图标本身没有 active 通道，选中色由 sync_tabs 逐个 apply（见 lib.rs）。
    mod.widgets.OuyuNavTab = mod.widgets.CheckBoxFlat{
        width: Fill
        height: Fill
        flow: Down
        spacing: 3.0
        padding: Inset{left: 2.0, right: 2.0, top: 6.0, bottom: 5.0}
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 21.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: ouyu.ink_2 }
        label_walk +: { margin: Inset{left: 0.0} }
        draw_bg +: {
            border_radius: r.button
            color: #0000
            color_hover: ouyu.hl_soft
            color_down: ouyu.hl
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
            color: ouyu.ink_2
            color_hover: ouyu.ink
            color_down: ouyu.ink
            color_focus: ouyu.ink_2
            color_active: ouyu.warm
            text_style +: { font_size: 11.5 }
        }
    }

    // 侧栏 Tab 也带图标（桌面形态）。
    mod.widgets.OuyuTabIcon = mod.widgets.OuyuTab{
        spacing: 10.0
        icon_walk: Walk{ width: 17.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: ouyu.ink_2 }
    }

    // 纯图标工具按钮（关闭、返回、更多）。
    mod.widgets.OuyuIconBtn = mod.widgets.ButtonFlat{
        width: 34 height: 34
        spacing: 0.0
        text: ""
        padding: 0.0
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{ width: 16.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: ouyu.ink_2 }
        draw_bg +: {
            border_radius: r.chip
            border_size: 0.0
            color: #0000
            color_hover: ouyu.hl
            color_down: ouyu.hl
            color_focus: #0000
        }
    }

    // 圆形「+」按钮：头部的新建 / 导入入口，点开一张菜单。
    mod.widgets.OuyuAddBtn = mod.widgets.OuyuIconBtn{
        width: 32 height: 32
        icon_walk: Walk{ width: 15.0 height: Fit }
        draw_icon +: { color: ouyu.ink }
        draw_bg +: {
            border_radius: r.card
            border_size: 1.0
            border_color: ouyu.line
            border_color_hover: ouyu.blue
            border_color_down: ouyu.blue
            border_color_focus: ouyu.line
        }
    }

    // 下拉菜单面板：贴在触发按钮下方的一张小卡，里面若干 OuyuMenuItem。
    // 不做浮层 —— 这一版菜单只有两三条，就地展开比盖一层更好收回。
    mod.widgets.OuyuMenu = mod.widgets.RoundedView{
        width: 172 height: Fit
        flow: Down
        padding: 6.0
        spacing: 2.0
        draw_bg +: {
            color: ouyu.card_2
            border_color: ouyu.line
            border_size: 1.0
            border_radius: r.button
        }
    }

    // 菜单条目：整条可点，文字左对齐，左边留一个图标位。
    mod.widgets.OuyuMenuItem = mod.widgets.ButtonFlat{
        width: Fill height: 34
        spacing: 8.0
        align: Align{x: 0.0, y: 0.5}
        icon_walk: Walk{ width: 15.0 height: Fit }
        padding: Inset{left: 10.0, right: 14.0, top: 6.0, bottom: 6.0}
        label_walk +: { margin: Inset{left: 0.0} }
        draw_icon +: { preserve_viewbox: true color: ouyu.ink_2 }
        draw_bg +: {
            border_radius: r.chip
            border_size: 0.0
            color: #0000
            color_hover: ouyu.hl
            color_down: ouyu.hl
            color_focus: #0000
        }
        draw_text +: {
            color: ouyu.ink
            color_hover: ouyu.ink
            color_down: ouyu.ink
            color_focus: ouyu.ink
            text_style +: { font_size: 13.0 }
        }
    }

    // 文字链：次要出口（「只留一条回忆」这类），不与主按钮争夺注意力。
    mod.widgets.OuyuLink = mod.widgets.ButtonFlat{
        height: Fit
        spacing: 4.0
        padding: Inset{left: 2.0, right: 2.0, top: 6.0, bottom: 6.0}
        icon_walk: Walk{ width: 13.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: ouyu.blue }
        draw_bg +: {
            border_size: 0.0
            border_radius: r.button
            color: #0000
            color_hover: #0000
            color_down: #0000
            color_focus: #0000
        }
        draw_text +: {
            color: ouyu.blue
            color_hover: ouyu.blue_soft
            color_down: ouyu.blue_soft
            color_focus: ouyu.blue
            text_style +: { font_size: 13.0 }
        }
    }

    // 分段控件的一段（今天 / 明天 / 本周）：等宽、方角药丸。
    mod.widgets.OuyuSeg = mod.widgets.CheckBoxFlat{
        width: Fill
        height: 36
        padding: Inset{left: 6.0, right: 6.0}
        align: Align{x: 0.5, y: 0.5}
        label_walk +: { margin: Inset{left: 0.0} }
        draw_bg +: {
            border_radius: r.chip
            color: #0000
            color_hover: ouyu.hl_soft
            color_down: ouyu.hl
            color_focus: #0000
            color_active: ouyu.hl_active
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
            color: ouyu.ink_2
            color_hover: ouyu.ink
            color_down: ouyu.ink
            color_focus: ouyu.ink_2
            color_active: ouyu.ink
            text_style +: { font_size: 14.0 }
        }
    }

    // 分段控件外框：一条内凹的轨道，里面放若干 OuyuSeg。
    mod.widgets.OuyuSegTrack = mod.widgets.RoundedView{
        width: Fill height: Fit
        flow: Right
        spacing: 4.0
        padding: 4.0
        draw_bg +: {
            color: ouyu.well
            border_color: ouyu.line
            border_size: 1.0
            border_radius: r.button
        }
    }

    // 日期格（本周的 7 天）：上星期几、下日期，底部一个强度点。
    mod.widgets.OuyuDay = mod.widgets.CheckBoxFlat{
        width: Fill
        height: 58
        flow: Down
        spacing: 3.0
        icon_walk: Walk{ width: 9.0 height: Fit }
        draw_icon +: { preserve_viewbox: true color: ouyu.ink_2 }
        padding: Inset{left: 2.0, right: 2.0, top: 7.0, bottom: 6.0}
        align: Align{x: 0.5, y: 0.5}
        label_walk +: { margin: Inset{left: 0.0} }
        draw_bg +: {
            border_radius: r.button
            color: #0000
            color_hover: ouyu.hl_soft
            color_down: ouyu.hl
            color_focus: #0000
            color_active: ouyu.hl_active
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
            color: ouyu.ink_2
            color_hover: ouyu.ink
            color_down: ouyu.ink
            color_focus: ouyu.ink_2
            color_active: ouyu.ink
            text_style +: { font_size: 12.0 }
        }
    }

    // 列表四态（空 / 加载 / 出错 / 离线）共用的一个组件（docs/02 六·6、七.5）。
    //
    // 只有一个预设，四个态在 Rust 里换图标、颜色、两行文案和那个按钮 ——
    // 四份各写一遍的话，迟早有一页的「离线」长得和别页不一样。
    // 文案保持短（≤16 字），以便在 320pt 下用 Fit 居中也不会溢出。
    mod.widgets.OuyuEmpty = mod.widgets.View{
        width: Fill height: Fit
        flow: Down
        align: Align{x: 0.5, y: 0.5}
        spacing: 10.0
        padding: Inset{left: 12.0, right: 12.0, top: 24.0, bottom: 24.0}
        em_icon := mod.widgets.OuyuIcon {
            icon_walk: Walk{ width: 28.0 height: Fit }
            draw_icon +: { color: ouyu.ink_ghost }
        }
        em_text := Label {
            width: Fit
            draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 13.0 } }
        }
        // 第二行说「为什么」和「怎么办」，普通空态用不上，默认收着。
        em_sub := Label {
            visible: false
            width: Fill
            margin: Inset{left: 16.0, right: 16.0}
            draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.0 } }
        }
        em_action := mod.widgets.OuyuBtn { visible: false width: Fit text: "" }
    }

    // 列表行（机会排行 / 区域选择器共用）：整行可点。
    //
    // Makepad 的 Button 只画自己的 icon + text，装不下「两行文字 + 右侧分档」，
    // 所以这里用 Flow::Overlay：下层 ar_body 负责显示，上层 ar_hit 是一个透明
    // 按钮铺满整行负责接点击与 hover 高亮。行高固定，Overlay 里的 Fill 才有解。
    mod.widgets.OuyuAreaRow = mod.widgets.View{
        width: Fill height: 56
        flow: Overlay
        ar_body := mod.widgets.View {
            width: Fill height: Fill
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            padding: Inset{left: 12.0, right: 12.0}
            spacing: 10.0
            ar_dot := mod.widgets.OuyuIcon {
                icon_walk: Walk{ width: 15.0 height: Fit }
                draw_icon +: { svg: crate_resource("self:resources/icons/location.svg") color: ouyu.ink_2 }
            }
            // 两行字叠在固定 56 的行里：Label 默认带 theme.mspace_1（四边各
            // 3）的内边距，两行就白吃 12，加上 15pt/12pt 两行文字本身的
            // 24 + 19.2（font_size 是磅，落到 4/3 像素），Fit 算出来是 67 —— 比行高
            // 还多 11，于是第二行被 ar_col 自己的矩形裁掉半截。
            // 竖直方向清零、横向保留 3（跟右侧 ar_level 的内边距对齐），
            // 行内文字回到 43.2，56 的行里还剩下六个多像素的上下留白。
            ar_col := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 2.0
                ar_name := Label {
                    width: Fill
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: "片区"
                    draw_text +: { wrap: Ellipsis color: ouyu.ink text_style +: { font_size: 15.0 } }
                }
                ar_sub := Label {
                    width: Fill
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: ""
                    draw_text +: { wrap: Ellipsis color: ouyu.ink_2 text_style +: { font_size: 12.0 } }
                }
            }
            ar_level := Label {
                width: Fit
                text: ""
                draw_text +: { color: ouyu.warm text_style +: { font_size: 12.5 } }
            }
        }
        ar_hit := mod.widgets.ButtonFlat {
            width: Fill height: Fill
            text: ""
            margin: 0.0
            padding: 0.0
            draw_bg +: {
                border_size: 0.0
                border_radius: r.button
                color: #0000
                color_hover: ouyu.wash_2
                color_down: ouyu.wash_3
                color_focus: ouyu.wash_1
            }
        }
    }

    // 分组小标题（行政区、「其它片区」这类），列表里起分隔作用。
    mod.widgets.OuyuGroupHead = mod.widgets.Label{
        width: Fill
        margin: Inset{top: 6.0, bottom: 2.0}
        draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.0 } }
    }

    // 搜索框：深色内凹，占位文字说明能输入什么。
    // 输入框。TextInputFlat 的每层都有 hover / focus / down / empty /
    // disabled 五档，漏掉哪一档就从 makepad 默认主题继承哪一档 —— 之前
    // 聚焦时占位文字用的正是那套默认色，落在偶遇的深井上几乎看不见。
    // 所以下面把每一档都写死，一档不留。
    mod.widgets.OuyuInput = mod.widgets.TextInputFlat{
        width: Fill height: Fit
        margin: 0.0
        padding: Inset{left: 12.0, right: 12.0, top: 10.0, bottom: 10.0}
        empty_text: "搜片区、行政区或拼音"
        draw_bg +: {
            border_radius: r.button
            border_size: 1.0
            color: ouyu.well
            color_hover: ouyu.well
            color_focus: ouyu.well_focus
            color_down: ouyu.well_focus
            color_empty: ouyu.well
            color_disabled: ouyu.off_bg
            border_color: ouyu.line
            border_color_hover: ouyu.line_strong
            border_color_focus: ouyu.blue
            border_color_down: ouyu.blue
            border_color_empty: ouyu.line
            border_color_disabled: ouyu.line
        }
        draw_text +: {
            // 正文一律 ink；占位一律 ink_hint，聚焦也不变浅。
            color: ouyu.ink
            color_hover: ouyu.ink
            color_focus: ouyu.ink
            color_down: ouyu.ink
            color_disabled: ouyu.off_ink
            color_empty: ouyu.ink_hint
            color_empty_hover: ouyu.ink_hint
            color_empty_focus: ouyu.ink_hint
            text_style +: { font_size: 14.0 line_spacing: 1.3 }
        }
        draw_selection +: {
            color: ouyu.sel
            color_hover: ouyu.sel
            color_focus: ouyu.sel
            color_down: ouyu.sel
            color_empty: #0000
            color_disabled: #0000
        }
        draw_cursor +: { color: ouyu.blue }
    }

    // 人物行（现场互认第 ① 屏）：首字色块 + 姓名 + 相遇次数，整行可点。
    //
    // 用首字圆形而不是头像：头像是一张随时会泄露身份的图，而这一屏本来就
    // 只在本机联系人里挑人，首字已经够认。
    mod.widgets.OuyuPersonRow = mod.widgets.View{
        width: Fill height: 64
        flow: Overlay
        ps_body := mod.widgets.View {
            width: Fill height: Fill
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            padding: Inset{left: 12.0, right: 12.0}
            spacing: 12.0
            ps_face := mod.widgets.RoundedView {
                width: 40 height: 40
                flow: Down
                align: Align{x: 0.5, y: 0.5}
                draw_bg +: { color: ouyu.face border_radius: r.card }
                ps_initial := Label {
                    text: ""
                    draw_text +: { color: ouyu.blue text_style +: { font_size: 17.0 } }
                }
            }
            // 跟 OuyuAreaRow 同一笔账：Label 默认四边各 3 的内边距，两行就白
            // 吃 12，加上两行文字的 24 + 19.2 是 67，比 64 的行高还多，第二行
            // 会被裁掉底下一截。竖直清零、横向留 3。
            ps_col := mod.widgets.View {
                width: Fill height: Fit
                flow: Down
                spacing: 2.0
                ps_name := Label {
                    width: Fill
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: ""
                    draw_text +: { wrap: Ellipsis color: ouyu.ink text_style +: { font_size: 15.0 } }
                }
                ps_sub := Label {
                    width: Fill
                    padding: Inset{left: 3.0, right: 3.0, top: 0.0, bottom: 0.0}
                    text: ""
                    draw_text +: { wrap: Ellipsis color: ouyu.ink_2 text_style +: { font_size: 12.0 } }
                }
            }
            // Icon 自己没有 visible，套一层 View 才能整体藏起来。
            ps_tick := mod.widgets.View {
                visible: false
                width: Fit height: Fit
                ps_tick_icon := mod.widgets.OuyuIconWarm {
                    icon_walk: Walk{ width: 16.0 height: Fit }
                    draw_icon +: { svg: crate_resource("self:resources/icons/check.svg") }
                }
            }
        }
        ps_hit := mod.widgets.ButtonFlat {
            width: Fill height: Fill
            text: ""
            margin: 0.0
            padding: 0.0
            draw_bg +: {
                border_size: 0.0
                border_radius: r.button
                color: #0000
                color_hover: ouyu.wash_2
                color_down: ouyu.wash_3
                color_focus: ouyu.wash_1
            }
        }
    }

    // 等待态的进度环。整圈是已走掉的时间，从 12 点方向顺时针长出来。
    //
    // 纯 script 预设，不需要 Rust widget：进度用 instance 字段，刷新时
    // script_apply_eval! 改一下就行。
    mod.widgets.OuyuRing = mod.widgets.View{
        width: 96 height: 96
        show_bg: true
        draw_bg +: {
            progress: instance(0.0)
            track_color: uniform(ouyu.ring_track)
            arc_color: uniform(ouyu.warm)
            stroke: uniform(5.0)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let center = self.rect_size * 0.5
                let radius = min(center.x, center.y) - self.stroke
                // 底环整圈画满，再把已过去的那一段盖成暖色。
                sdf.arc_round_caps(center.x, center.y, radius, 0.0, 2.0 * PI, self.stroke)
                sdf.fill(self.track_color)
                let start = -0.5 * PI
                let sweep = max(self.progress, 0.001) * 2.0 * PI
                sdf.arc_round_caps(center.x, center.y, radius, start, start + sweep, self.stroke)
                return sdf.fill(self.arc_color)
            }
        }
    }

    // 结果屏的店家行：名称 + 距离档位 + 地址 + 营业时间。
    //
    // 距离只到「步行可达」这一档 —— 米数等于在告诉你对方此刻离你多远。
    mod.widgets.OuyuShopRow = mod.widgets.View{
        width: Fill height: Fit
        flow: Down
        spacing: 3.0
        padding: Inset{top: 8.0, bottom: 8.0}
        sp_head := mod.widgets.View {
            width: Fill height: Fit
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            spacing: 8.0
            sp_name := Label {
                width: Fill
                text: ""
                draw_text +: { wrap: Ellipsis color: ouyu.ink text_style +: { font_size: 15.0 } }
            }
            sp_walk := Label {
                width: Fit
                text: ""
                draw_text +: { color: ouyu.good text_style +: { font_size: 12.0 } }
            }
        }
        sp_addr := Label {
            width: Fill
            text: ""
            draw_text +: { wrap: Words color: ouyu.ink_2 text_style +: { font_size: 12.0 } }
        }
        sp_hours := Label {
            width: Fill
            text: ""
            draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.0 } }
        }
    }

    // 设置行：名称 + 右侧当前值 + 箭头，整行可点。
    //
    // 右侧那一格是「现在是什么样」（已开启 / 3 张可用），不是一句解释；
    // 需要解释的写在 st_sub 里，读的人不用点进去才知道这一项管什么。
    //
    // 没有行首图标：图标得在标记里写死（script_apply_eval! 的作用域里没有
    // crate_resource），一行一个预设就得复制一份，不如都不要。
    mod.widgets.OuyuSetRow = mod.widgets.View{
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
                    width: Fill
                    text: ""
                    draw_text +: { wrap: Ellipsis color: ouyu.ink text_style +: { font_size: 15.0 } }
                }
                st_sub := Label {
                    visible: false
                    width: Fill
                    text: ""
                    draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.0 } }
                }
            }
            st_val := Label {
                width: Fit
                text: ""
                draw_text +: { color: ouyu.ink_2 text_style +: { font_size: 13.0 } }
            }
            st_arrow := mod.widgets.OuyuIcon {
                icon_walk: Walk{ width: 13.0 height: Fit }
                draw_icon +: { svg: crate_resource("self:resources/icons/chevron-right.svg") color: ouyu.ink_arrow }
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
                color_hover: ouyu.wash_2
                color_down: ouyu.wash_3
                color_focus: ouyu.wash_1
            }
        }
    }

    // 「我的行踪」里的一行，固定两行高：文案和「修改 / 删除」并排占第一行，
    // 状态占第二行。文案让出按钮那点宽度后一行放不下就省略号收尾 —— 一条行踪
    // 换行铺成三四行，列表上一屏只剩两三条，扫一眼看不到今天都发了什么。
    //
    // 到期的那几条只留文案和「已过期」，两个按钮由 Rust 收起 —— 改一条已经
    // 结束的行踪没有意义，删掉它也只是在删自己的回看记录。
    mod.widgets.OuyuTrackRow = mod.widgets.View{
        width: Fill height: Fit
        // 两行堆在 tr_col 里而不是直接堆在行上：预设自己写的
        // flow: Down 到不了实例（tr0 := OuyuTrackRow { }），行会蹦回横排，
        // 状态那一行被挤成零宽、每个字一行，就成了两条行踪中间那块空白。
        // 子节点自己的 flow 是稳的，所以把竖排放在这一层。
        tr_col := mod.widgets.View {
            width: Fill height: Fit
            flow: Down
            spacing: 2.0
            padding: Inset{left: 12.0, right: 12.0, top: 10.0, bottom: 10.0}
            tr_top := mod.widgets.View {
                width: Fill height: Fit
                flow: Right
                align: Align{x: 0.0, y: 0.5}
                spacing: 8.0
                tr_text := Label {
                    width: Fill
                    max_lines: 1
                    text_overflow: Ellipsis
                    text: ""
                    draw_text +: { color: ouyu.ink text_style +: { font_size: 15.0 } }
                }
                tr_edit := mod.widgets.OuyuBtnSm { width: Fit text: "修改" }
                tr_del := mod.widgets.OuyuBtnDangerSm { width: Fit text: "删除" }
            }
            tr_state := Label {
                width: Fill
                text: ""
                draw_text +: { color: ouyu.ink_3 text_style +: { font_size: 12.0 } }
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
    mod.widgets.OuyuSwitchRow = mod.widgets.View{
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
                    width: Fill
                    text: ""
                    draw_text +: { wrap: Words color: ouyu.ink text_style +: { font_size: 15.0 } }
                }
                sw_sub := Label {
                    width: Fill
                    text: ""
                    draw_text +: { wrap: Words color: ouyu.ink_3 text_style +: { font_size: 12.0 } }
                }
            }
            sw_track := mod.widgets.RoundedView {
                width: 44 height: 26
                flow: Right
                align: Align{x: 0.0, y: 0.5}
                padding: Inset{left: 3.0, right: 3.0}
                draw_bg +: { color: ouyu.track_off border_radius: 13.0 }   // 胶囊:= height/2,形状本身,不参与圆角收敛
                sw_off := mod.widgets.RoundedView {
                    width: 20 height: 20
                    draw_bg +: { color: ouyu.knob border_radius: 10.0 }       // 正圆:= height/2,同上
                }
                sw_gap := mod.widgets.View { width: Fill height: Fit }
                sw_on := mod.widgets.RoundedView {
                    visible: false
                    width: 20 height: 20
                    draw_bg +: { color: ouyu.knob_on border_radius: 10.0 }    // 正圆:= height/2,同上
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
                color_hover: ouyu.wash_2
                color_down: ouyu.wash_3
                color_focus: ouyu.wash_1
            }
        }
    }

    // 券包里的一张券。和结果屏那张同一套视觉，只是多了「可用 / 已核销 / 已过期」。
    //
    // 券面上没有联系人、没有坐标、没有相遇日期 —— 券会被截图、会被转发，
    // 上面只能有商户愿意公开的那些字（02 B 节）。
    mod.widgets.OuyuCouponCard = mod.widgets.RoundedView{
        width: Fill height: Fit
        flow: Down
        padding: 16.0
        spacing: 8.0
        draw_bg +: { color: ouyu.coupon border_radius: r.card }
        cw_head := mod.widgets.View {
            width: Fill height: Fit
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            spacing: 8.0
            cw_venue := Label {
                width: Fill
                text: ""
                draw_text +: { wrap: Ellipsis color: ouyu.on_warm text_style +: { font_size: 13.0 } }
            }
            cw_state := Label {
                width: Fit
                text: ""
                draw_text +: { color: ouyu.on_warm text_style +: { font_size: 11.5 } }
            }
        }
        cw_offer := Label {
            width: Fill
            text: ""
            draw_text +: { wrap: Words color: ouyu.warm_btn text_style +: { font_size: 24.0 } }
        }
        cw_meta := mod.widgets.View {
            width: Fill height: Fit
            flow: Right{wrap: true}
            wrap_spacing: 4.0
            spacing: 12.0
            cw_token := Label {
                text: ""
                draw_text +: { color: ouyu.warm_btn text_style +: { font_size: 13.0 } }
            }
            cw_expiry := Label {
                text: ""
                draw_text +: { color: ouyu.on_warm text_style +: { font_size: 13.0 } }
            }
        }
        cw_terms := Label {
            width: Fill
            text: ""
            draw_text +: { wrap: Words color: ouyu.on_warm text_style +: { font_size: 11.5 } }
        }
        cw_row := mod.widgets.View {
            width: Fill height: Fit
            flow: Right{wrap: true}
            wrap_spacing: 8.0
            spacing: 8.0
            cw_redeem := mod.widgets.OuyuBtnWarm { text: "到店核销" }
        }
    }

    // Toast：写入成功 / 撤销提示，浮在底部导航之上。
    mod.widgets.OuyuToast = mod.widgets.RoundedView{
        visible: false
        width: Fill height: Fit
        flow: Right
        align: Align{x: 0.0, y: 0.5}
        spacing: 8.0
        padding: Inset{left: 14.0, right: 10.0, top: 10.0, bottom: 10.0}
        draw_bg +: {
            color: ouyu.raise
            border_color: ouyu.line_strong
            border_size: 1.0
            border_radius: r.button
        }
        to_text := Label {
            width: Fill
            text: ""
            draw_text +: { wrap: Ellipsis color: ouyu.ink text_style +: { font_size: 13.0 } }
        }
        to_undo := mod.widgets.OuyuBtn {
            width: Fit height: 30
            text: "撤销"
            padding: Inset{left: 12.0, right: 12.0, top: 5.0, bottom: 5.0}
            icon_walk: Walk{ width: 13.0 height: Fit }
            draw_icon +: { preserve_viewbox: true svg: crate_resource("self:resources/icons/undo.svg") color: ouyu.blue }
            draw_text +: { color: ouyu.blue color_hover: ouyu.blue color_down: ouyu.blue color_focus: ouyu.blue }
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

/// 一段折线（SDF 胶囊）：y0/y1 是段两端在本 quad 内的纵坐标（0=顶）。
#[derive(Script, ScriptHook)]
#[repr(C)]
pub struct DrawLineSeg {
    #[deref]
    draw_super: DrawQuad,
    #[live]
    pub y0: f32,
    #[live]
    pub y1: f32,
    #[live]
    pub color_line: Vec4f,
    #[live(2.0)]
    pub thickness: f32,
}

/// 曲线下方的浅填充：一个区间一个梯形，top_left/right 是曲线高度（0=顶）。
#[derive(Script, ScriptHook)]
#[repr(C)]
pub struct DrawAreaFill {
    #[deref]
    draw_super: DrawQuad,
    #[live]
    pub top_left: f32,
    #[live]
    pub top_right: f32,
    #[live]
    pub color_top: Vec4f,
}

// ---- 匿名机会街区插图 ----

#[derive(Clone, Debug)]
pub struct MapLabel {
    pub text: String,
    pub x: f64,
    pub y: f64,
}

/// 父级交给地图的显示快照（地图坐标 0..1000）。只有街区标签与中心文案，
/// 没有任何指向具体人的内容。
#[derive(Clone, Debug, Default)]
pub struct MapScene {
    pub labels: Vec<MapLabel>,
    pub center_title: String,
    pub center_sub: String,
    pub hint: String,
}

#[derive(Script, ScriptHook, Widget)]
pub struct OuyuMap {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,

    #[redraw]
    #[live]
    draw_bg: DrawColor,
    #[live]
    draw_road: DrawColor,
    #[live]
    draw_block: DrawRoundBox,
    #[live]
    draw_glow: DrawRoundBox,
    #[live]
    draw_ring: DrawRoundBox,
    #[live]
    draw_dot: DrawRoundBox,
    #[live]
    draw_label: DrawText,
    #[live]
    draw_title: DrawText,
    #[live]
    draw_sub: DrawText,
    #[live]
    draw_hint: DrawText,

    #[rust]
    scene: MapScene,
    #[rust]
    rect: Rect,
}

impl OuyuMap {
    pub fn set_scene(&mut self, scene: MapScene) {
        self.scene = scene;
    }

    fn px(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.rect.pos.x + x / 1000.0 * self.rect.size.x,
            self.rect.pos.y + y / 1000.0 * self.rect.size.y,
        )
    }

    fn draw_circle(&mut self, cx: &mut Cx2d, d: usize, px: f64, py: f64, r: f64) {
        let r = r.max(1.0);
        let d: &mut DrawRoundBox = match d {
            0 => &mut self.draw_glow,
            1 => &mut self.draw_ring,
            _ => &mut self.draw_dot,
        };
        d.draw_abs(
            cx,
            Rect {
                pos: dvec2(px - r, py - r),
                size: dvec2(r * 2.0, r * 2.0),
            },
        );
    }
}

impl Widget for OuyuMap {
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        self.rect = cx.walk_turtle(walk);
        let r = self.rect;
        self.draw_bg.draw_abs(cx, r);
        let scene = self.scene.clone();

        // 街区网格块 + 主路（示意图，不是卫星图）。
        let cols = 6;
        let rows = 4;
        let road_w = 14.0;
        let cell_w = ((r.size.x - road_w * (cols as f64 + 1.0)) / cols as f64).max(4.0);
        let cell_h = ((r.size.y - road_w * (rows as f64 + 1.0)) / rows as f64).max(4.0);
        for i in 0..=cols {
            self.draw_road.draw_abs(
                cx,
                Rect {
                    pos: dvec2(r.pos.x + i as f64 * (cell_w + road_w), r.pos.y),
                    size: dvec2(road_w, r.size.y),
                },
            );
        }
        for j in 0..=rows {
            self.draw_road.draw_abs(
                cx,
                Rect {
                    pos: dvec2(r.pos.x, r.pos.y + j as f64 * (cell_h + road_w)),
                    size: dvec2(r.size.x, road_w),
                },
            );
        }
        self.draw_block.radius = 4.0;
        for i in 0..cols {
            for j in 0..rows {
                self.draw_block.draw_abs(
                    cx,
                    Rect {
                        pos: dvec2(
                            r.pos.x + road_w + i as f64 * (cell_w + road_w) + 3.0,
                            r.pos.y + road_w + j as f64 * (cell_h + road_w) + 3.0,
                        ),
                        size: dvec2(cell_w - 6.0, cell_h - 6.0),
                    },
                );
            }
        }

        // 区域标签
        for label in &scene.labels {
            let (px, py) = self.px(label.x, label.y);
            self.draw_label.draw_abs(cx, dvec2(px - 16.0, py), &label.text);
        }

        // 匿名机会光圈：静态虚线圆环（一圈暖杏小点，隔一个画一个）+ 中心光点。
        // 05 明确「不用脉冲暗示有人移动」，所以没有任何动画。
        let (cxp, cyp) = self.px(500.0, 470.0);
        let ring_r = r.size.x.min(r.size.y) * 0.30;
        self.draw_circle(cx, 0, cxp, cyp, ring_r * 1.15);
        const SEGS: usize = 44;
        for i in (0..SEGS).step_by(2) {
            let a = i as f64 / SEGS as f64 * std::f64::consts::TAU;
            self.draw_circle(cx, 1, cxp + ring_r * a.cos(), cyp + ring_r * a.sin(), 2.0);
        }
        self.draw_circle(cx, 2, cxp, cyp, 5.0);
        if !scene.center_title.is_empty() {
            self.draw_title
                .draw_abs(cx, dvec2(cxp - 62.0, cyp + ring_r * 0.35), &scene.center_title);
        }
        if !scene.center_sub.is_empty() {
            self.draw_sub
                .draw_abs(cx, dvec2(cxp - 76.0, cyp + ring_r * 0.35 + 20.0), &scene.center_sub);
        }

        if !scene.hint.is_empty() {
            self.draw_hint.draw_abs(
                cx,
                dvec2(r.pos.x + 12.0, r.pos.y + r.size.y - 24.0),
                &scene.hint,
            );
        }
        DrawStep::done()
    }

    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) {}
}

// ---- 相遇频率曲线（成就页，06 节）----

/// 父级交给曲线的显示数据：每周次数 + 首尾日期标注 + 末周标注。
/// 只来自可见回忆的周桶汇总，没有姓名 / 地点 / 具体某一天。
#[derive(Clone, Debug, Default)]
pub struct ChartData {
    pub counts: Vec<usize>,
    pub first_label: String,
    pub last_label: String,
    /// 末周标注（「本周尚未结束」）。
    pub last_note: String,
}

#[derive(Script, ScriptHook, Widget)]
pub struct OuyuChart {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,

    #[redraw]
    #[live]
    draw_area: DrawAreaFill,
    #[live]
    draw_line: DrawLineSeg,
    #[live]
    draw_dot: DrawRoundBox,
    #[live]
    draw_rule: DrawColor,
    #[live]
    draw_text: DrawText,

    #[rust]
    data: ChartData,
}

impl OuyuChart {
    pub fn set_data(&mut self, data: ChartData) {
        self.data = data;
    }
}

impl Widget for OuyuChart {
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        let r = cx.walk_turtle(walk);
        let n = self.data.counts.len();
        if n == 0 || r.size.x < 60.0 || r.size.y < 60.0 {
            return DrawStep::done();
        }
        // 布局：左侧放整数刻度，底部放首尾日期。
        let (pad_l, pad_r, pad_t, pad_b) = (34.0, 10.0, 12.0, 24.0);
        let plot = Rect {
            pos: dvec2(r.pos.x + pad_l, r.pos.y + pad_t),
            size: dvec2(
                (r.size.x - pad_l - pad_r).max(1.0),
                (r.size.y - pad_t - pad_b).max(1.0),
            ),
        };
        let ymax = self.data.counts.iter().copied().max().unwrap_or(0).max(1);
        // 纵轴从零开始、整数刻度：刻度过密时只标 0 与最大值。
        let ticks: Vec<usize> = if ymax <= 6 {
            (0..=ymax).collect()
        } else {
            vec![0, ymax]
        };
        for v in &ticks {
            let y = plot.pos.y + plot.size.y * (1.0 - *v as f64 / ymax as f64);
            self.draw_rule.draw_abs(
                cx,
                Rect { pos: dvec2(plot.pos.x, y), size: dvec2(plot.size.x, 1.0) },
            );
            self.draw_text
                .draw_abs(cx, dvec2(r.pos.x, y - 6.0), &format!("{v}次"));
        }
        let px = |i: usize| {
            if n == 1 {
                plot.pos.x + plot.size.x * 0.5
            } else {
                plot.pos.x + plot.size.x * i as f64 / (n - 1) as f64
            }
        };
        let py = |v: usize| plot.pos.y + plot.size.y * (1.0 - v as f64 / ymax as f64);
        // 填充 + 折线 + 圆点。
        for i in 0..n.saturating_sub(1) {
            let (v0, v1) = (self.data.counts[i], self.data.counts[i + 1]);
            let quad = Rect {
                pos: dvec2(px(i), plot.pos.y),
                size: dvec2((px(i + 1) - px(i)).max(1.0), plot.size.y),
            };
            self.draw_area.top_left = (1.0 - v0 as f32 / ymax as f32).clamp(0.0, 1.0);
            self.draw_area.top_right = (1.0 - v1 as f32 / ymax as f32).clamp(0.0, 1.0);
            self.draw_area.draw_abs(cx, quad);
            self.draw_line.y0 = (1.0 - v0 as f32 / ymax as f32).clamp(0.0, 1.0);
            self.draw_line.y1 = (1.0 - v1 as f32 / ymax as f32).clamp(0.0, 1.0);
            self.draw_line.draw_abs(cx, quad);
        }
        for (i, v) in self.data.counts.iter().enumerate() {
            self.draw_dot.draw_abs(
                cx,
                Rect {
                    pos: dvec2(px(i) - 3.0, py(*v) - 3.0),
                    size: dvec2(6.0, 6.0),
                },
            );
        }
        // x 轴首尾日期 + 末周标注。
        self.draw_text
            .draw_abs(cx, dvec2(plot.pos.x, r.pos.y + r.size.y - 16.0), &self.data.first_label);
        self.draw_text.draw_abs(
            cx,
            dvec2(r.pos.x + r.size.x - 52.0, r.pos.y + r.size.y - 16.0),
            &self.data.last_label,
        );
        if !self.data.last_note.is_empty() {
            let x = (px(n - 1) - 90.0).max(plot.pos.x);
            let y = (py(self.data.counts[n - 1]) - 18.0).max(r.pos.y);
            self.draw_text.draw_abs(cx, dvec2(x, y), &self.data.last_note);
        }
        DrawStep::done()
    }

    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) {}
}

// ---- 分享卡预览（3:4，暖杏 / 夜蓝两样式；内容边界同 share.rs 场景）----

#[derive(Script, ScriptHook, Widget)]
pub struct OuyuShareCard {
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
    draw_dot: DrawRoundBox,
    #[live]
    draw_line: DrawLineSeg,
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

impl OuyuShareCard {
    pub fn set_scene(&mut self, scene: ShareCardScene) {
        self.scene = scene;
    }
}

impl Widget for OuyuShareCard {
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
        let fs = |pt: f64| (pt * sy).max(5.0);
        let (bg, fg, sub, accent, ring) = match self.scene.style {
            ShareStyle::Warm => (
                self.warm_bg,
                self.warm_fg,
                self.warm_sub,
                self.warm_accent,
                self.warm_ring,
            ),
            ShareStyle::Night => (
                self.night_bg,
                self.night_fg,
                self.night_sub,
                self.night_accent,
                self.night_ring,
            ),
        };
        self.draw_bg.color = bg;
        self.draw_bg.draw_abs(cx, r);
        // 装饰圆环（描边圆，超出卡面的部分不管，预览只是示意）。
        self.draw_ring.border_color = ring;
        for (cxp, cyp, rad) in [(800.0, 150.0, 170.0), (800.0, 150.0, 120.0), (60.0, 1010.0, 200.0)] {
            let rr = rad * (sx + sy) * 0.5;
            self.draw_ring.draw_abs(
                cx,
                Rect {
                    pos: dvec2(px(cxp) - rr, py(cyp) - rr),
                    size: dvec2(rr * 2.0, rr * 2.0),
                },
            );
        }
        // 文本（字号随卡片高度缩放）。
        self.draw_head.color = sub;
        self.draw_text.color = fg;
        self.draw_big.color = accent;
        self.draw_head.text_style.font_size = fs(22.0) as f32;
        self.draw_head.draw_abs(cx, dvec2(px(64.0), py(70.0)), "OUYU / 我的相遇手记");
        self.draw_text.text_style.font_size = fs(40.0) as f32;
        self.draw_text.draw_abs(cx, dvec2(px(64.0), py(140.0)), "★");
        self.draw_text.text_style.font_size = fs(64.0) as f32;
        self.draw_text.draw_abs(cx, dvec2(px(64.0), py(236.0)), "给生活");
        self.draw_text.draw_abs(cx, dvec2(px(64.0), py(320.0)), "留一点偶然。");
        self.draw_text.text_style.font_size = fs(26.0) as f32;
        self.draw_text.color = sub;
        self.draw_text.draw_abs(cx, dvec2(px(64.0), py(422.0)), "不用专程约，也许刚好遇见。");
        self.draw_text.color = fg;
        self.draw_big.text_style.font_size = fs(200.0) as f32;
        self.draw_big
            .draw_abs(cx, dvec2(px(64.0), py(500.0)), &self.scene.total.to_string());
        self.draw_text.text_style.font_size = fs(28.0) as f32;
        self.draw_text
            .draw_abs(cx, dvec2(px(64.0), py(742.0)), "次，我愿意记住的相遇");
        self.draw_text.text_style.font_size = fs(30.0) as f32;
        self.draw_text
            .draw_abs(cx, dvec2(px(64.0), py(846.0)), &self.scene.milestone_title);
        if !self.scene.milestone_sub.is_empty() {
            self.draw_text.text_style.font_size = fs(20.0) as f32;
            self.draw_text.color = sub;
            self.draw_text
                .draw_abs(cx, dvec2(px(64.0), py(902.0)), &self.scene.milestone_sub);
            self.draw_text.color = fg;
        }
        // 可选曲线：折线 + 圆点，不标日期。
        if let Some(counts) = self.scene.curve.clone() {
            if !counts.is_empty() {
                let ymax = counts.iter().copied().max().unwrap_or(0).max(1) as f64;
                let n = counts.len();
                let lx = |i: usize| {
                    if n == 1 {
                        px(450.0)
                    } else {
                        px(64.0) + (px(836.0) - px(64.0)) * i as f64 / (n - 1) as f64
                    }
                };
                let ly = |v: usize| py(1040.0) - (py(1040.0) - py(960.0)) * v as f64 / ymax;
                self.draw_line.color_line = accent;
                for i in 0..n.saturating_sub(1) {
                    let quad = Rect {
                        pos: dvec2(lx(i), py(960.0)),
                        size: dvec2((lx(i + 1) - lx(i)).max(1.0), py(1040.0) - py(960.0)),
                    };
                    self.draw_line.y0 = (1.0 - counts[i] as f32 / ymax as f32).clamp(0.0, 1.0);
                    self.draw_line.y1 =
                        (1.0 - counts[i + 1] as f32 / ymax as f32).clamp(0.0, 1.0);
                    self.draw_line.draw_abs(cx, quad);
                }
                self.draw_dot.color = accent;
                for (i, v) in counts.iter().enumerate() {
                    self.draw_dot.draw_abs(
                        cx,
                        Rect {
                            pos: dvec2(lx(i) - 3.0, ly(*v) - 3.0),
                            size: dvec2(6.0, 6.0),
                        },
                    );
                }
            }
        }
        // 底部署名。
        self.draw_dot.color = ring;
        self.draw_dot.draw_abs(
            cx,
            Rect {
                pos: dvec2(px(64.0), py(1080.0)),
                size: dvec2(px(836.0) - px(64.0), 1.5),
            },
        );
        self.draw_head.color = fg;
        self.draw_head.text_style.font_size = fs(26.0) as f32;
        self.draw_head.draw_abs(cx, dvec2(px(64.0), py(1104.0)), "偶遇 OuYu");
        self.draw_head.color = sub;
        self.draw_head.text_style.font_size = fs(16.0) as f32;
        self.draw_head
            .draw_abs(cx, dvec2(px(610.0), py(1114.0)), "个人记录 · 非社交排名");
        DrawStep::done()
    }

    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) {}
}
