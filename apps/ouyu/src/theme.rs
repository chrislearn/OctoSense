//! 偶遇的色板：一份角色表，两套取值（夜 / 昼）。
//!
//! 全应用不再写颜色字面量。写法是 `ouyu.<角色>`，角色名说的是「这是什么」
//! （正文 / 次文 / 主卡 / 分隔线 / 暖色强调），不是「它多深」—— 换一套主题
//! 只是同一批角色换一组取值，界面的层级关系不跟着变。
//!
//! 切换的路径：`set_mode()` 改进程内的选择 → `OuyuView::restyle` 在自己的
//! 隔离 VM 里重跑 `install` + 两个 `script_mod`，再用 `Apply::ScriptReapply`
//! 把整棵树按新色板刷一遍（宿主换外观时走的是同一条路，见 `module_host.rs`
//! 的 `apply_style`）。
//!
//! 有几组颜色故意不进主题：分享卡的两套配色（它导出成图片，跟界面深浅无关）
//! 和 `#0000`（透明就是透明）。
use makepad_widgets::*;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

script_mod! {
    use mod.prelude.widgets.*

    // ---- 夜（默认）----
    //
    // 底色是夜蓝，强调色是暖杏。暖杏在夜里可以直接当文字色用；到了昼版
    // 它在白底上读不出来，所以「暖色文字」和「暖色面」是两个角色。
    mod.ouyu_themes.dark = {
        // 面
        bg: #x0b1220           // 页面底
        bg_chrome: #x0d1626    // 侧栏 / 底部导航
        card: #x152235         // 主卡
        card_2: #x111c2c       // 次卡
        well: #x101a2c         // 内凹（输入框、分段轨道）
        well_focus: #x142238
        raise: #x1b2a44        // 浮层（toast / 通知）
        face: #x203049         // 首字圆底

        // 线
        line: #x1d2b42         // 常规描边
        line_soft: #x26364c    // 主卡描边、未走到的步点
        line_strong: #x30456b  // hover 描边、浮层描边
        line_notice: #x3a4f74  // 通知卡描边

        // 字
        ink: #xe7edf8          // 正文
        ink_2: #xa4b2c9        // 次文
        ink_3: #x7b8aa3        // 三级文（注脚）
        ink_4: #x6b7a99        // 图表刻度这类最轻的字
        ink_ghost: #x3c4c6b    // 空态图标
        ink_hint: #x6d7d97     // 输入框占位
        ink_arrow: #x5d6f8d    // 设置行的箭头

        // 暖色（强调）
        warm: #xffca91         // 暖色文字 / 图标 / 曲线
        warm_hi: #xffd9a8      // 暖色 hover
        warm_wash: #xffca9166  // 曲线下的浅填充
        map_glow: #xffca91     // 街区插图的光晕

        // 券面（暖底卡：两套主题下都是暖杏纸）
        coupon: #xffca91
        coupon_off: #x8f7c66   // 已核销 / 已过期
        on_warm: #x5a3d1e      // 券面次文
        on_warm_hi: #x2b1c0d   // 券面主文
        warm_btn: #x2b1c0d     // 券面上的按钮底
        warm_btn_hi: #x3a2812
        on_warm_btn: #xffca91  // 券面按钮上的字
        on_warm_btn_hi: #xffd9a8

        // 蓝色（操作）
        blue: #x82b5ff         // 主按钮底 / 文字链 / 选中描边
        blue_hi: #x9cc4ff
        blue_lo: #x6ba3f0
        blue_soft: #xa8ccff    // 文字链 hover
        sel: #x2c4d7d          // 选中文字的底
        on_blue: #x05070e      // 主按钮上的字与图标
        off_bg: #x2a3550       // 禁用底
        off_ink: #x63708a      // 禁用字

        // 语义
        good: #x9be2bf
        bad: #xff9eab
        bad_line: #x4a2530

        // 交互态
        hl: #x1d2b4a           // hover / pressed 高亮块
        hl_soft: #x1d2b4a55
        hl_active: #x24375c    // 分段控件选中
        wash_1: #xffffff08     // 整行按钮的三档蒙版
        wash_2: #xffffff0a
        wash_3: #xffffff14

        // 开关
        track_off: #x24354f
        knob: #x8fa2bf
        knob_on: #x0b1220

        // 街区插图
        map_bg: #x0a1120
        map_road: #x16223c
        map_block: #x101a2e

        // 强度刻度（回忆热力）
        heat_0: #x2b3a52
        heat_1: #x5b6f92
        ring_track: #x22334d

        // 滚动条手柄（半透明，压在页面底上）
        bar: #xffffff1f
        bar_hi: #xffffff33
        bar_drag: #xffffff4d
    }

    // ---- 昼 ----
    //
    // 同一批角色，白纸取值。三条硬约束：正文对底不低于 7:1，次文不低于
    // 4.5:1，暖色和蓝色一旦当文字用就压深 —— 夜版那两个亮调在白底上是看
    // 不见的（输入框占位符就是这么糊掉的）。
    mod.ouyu_themes.light = {
        bg: #xf2f5fa
        bg_chrome: #xe7ecf5
        card: #xffffff
        card_2: #xeef2f9
        well: #xf4f7fc
        well_focus: #xffffff
        raise: #xffffff
        face: #xdde7f9

        line: #xd2dae8
        line_soft: #xe1e7f1
        line_strong: #x9fb6d8
        line_notice: #xc6d6ef

        ink: #x16233c
        ink_2: #x4d5d78
        ink_3: #x6f7d95
        ink_4: #x7d8aa1
        ink_ghost: #xb6c2d6
        ink_hint: #x909db4
        ink_arrow: #x94a2ba

        warm: #xa8611a
        warm_hi: #x86490c
        warm_wash: #xe8a05a55
        map_glow: #xe8994d
        coupon: #xffca91
        coupon_off: #xe0cdb4
        on_warm: #x6b4a1f
        on_warm_hi: #x2b1c0d
        warm_btn: #xf3dcc2
        warm_btn_hi: #xe7cbab
        on_warm_btn: #x6b4a1f
        on_warm_btn_hi: #x4a3111

        blue: #x2f6fd0
        blue_hi: #x225cb6
        blue_lo: #x1a4e9e
        blue_soft: #x1a4e9e
        sel: #xbcd8fb          // 选中文字的底
        on_blue: #xffffff
        off_bg: #xd7dfec
        off_ink: #x9aa6bb

        good: #x16805a
        bad: #xc33a4c
        bad_line: #xe6b6bd

        hl: #xdde6f5
        hl_soft: #xdde6f599
        hl_active: #xcfe0fb
        wash_1: #x00000008
        wash_2: #x0000000f
        wash_3: #x0000001c

        track_off: #xc4cfe0
        knob: #xffffff
        knob_on: #xffffff

        map_bg: #xe8eef8
        map_road: #xdae3f1
        map_block: #xf3f7fd

        heat_0: #xcfd8e6
        heat_1: #x8e9db8
        ring_track: #xd5dded

        bar: #x16233c2e
        bar_hi: #x16233c47
        bar_drag: #x16233c66
    }
}

/// 界面深浅。只有两个值 —— 「跟随系统」留给宿主的外观菜单，应用内不再猜。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    pub const ALL: [Self; 2] = [Self::Dark, Self::Light];

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "夜色",
            Self::Light => "白昼",
        }
    }

    /// 落盘用的名字。认不出来的按夜色 —— 存档里写着未来某个主题名时，
    /// 宁可给一套一定存在的颜色，也不要一屏没有颜色的界面。
    pub fn id(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn parse(name: &str) -> Self {
        match name {
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }
}

/// 进程内的当前选择。`install` 在每个 VM（宿主下每个实例一个隔离 VM）里
/// 读它，所以同一进程里的几扇偶遇窗口共用一套深浅 —— 这是设置，不是每扇
/// 窗口各自的状态。
static MODE: AtomicU8 = AtomicU8::new(0);

pub fn mode() -> ThemeMode {
    if MODE.load(Ordering::Relaxed) == 1 {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    }
}

pub fn set_mode(mode: ThemeMode) {
    MODE.store(u8::from(mode == ThemeMode::Light), Ordering::Relaxed);
}

/// 进程里第一次装色板之前，把落盘的那个选择读进来。
///
/// 只读一次：之后 `set_mode` 说了算，不会有第二扇窗口把别人改过的深浅
/// 又按文件里的旧值退回去。
static LOADED: AtomicBool = AtomicBool::new(false);

fn load_persisted_once() {
    if LOADED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(name) = crate::data::OuyuState::persisted_theme() {
        set_mode(ThemeMode::parse(&name));
    }
}

/// 把两套色板装进这个 VM，再把选中的那套绑到 `mod.ouyu`，最后拼出偶遇自己
/// 的 prelude —— 之后所有预设里写的 `ouyu.ink` 都从这里解析。
///
/// 重跑一次（`vm.with_reload` 里）就是换一套主题：`mod.ouyu` 重新指向另一
/// 张表，后面 `canvas::script_mod` / `crate::script_mod` 重建的预设自然带上
/// 新颜色。
pub fn install(vm: &mut ScriptVm) {
    load_persisted_once();
    vm.bx.heap.new_module(id!(ouyu_themes));
    script_mod(vm);
    match mode() {
        ThemeMode::Dark => {
            script_eval!(vm, { mod.ouyu = mod.ouyu_themes.dark });
        }
        ThemeMode::Light => {
            script_eval!(vm, { mod.ouyu = mod.ouyu_themes.light });
        }
    }
    script_eval!(vm, {
        mod.prelude.ouyu = {
            ..mod.prelude.widgets,
            ouyu: mod.ouyu,
        }
    });
}

/// Rust 侧要用到的那几个角色。
///
/// 界面里绝大多数颜色写在预设里，换主题跟着 reapply 一起变；只有「跟数据走」
/// 的那几处（选中的 Tab、强度分档、开关轨道、券的可用与否）是 Rust 每次刷新
/// 时才知道该用哪一个，所以这里把它们从同一张表里读出来缓存着。
///
/// 读不到的角色（表里少写了一个）退回夜版取值，不会画出一块透明。
#[derive(Clone, Copy, Debug)]
pub struct Pal {
    pub ink: Vec4f,
    pub ink_2: Vec4f,
    pub ink_3: Vec4f,
    pub ink_ghost: Vec4f,
    pub line_soft: Vec4f,
    pub warm: Vec4f,
    pub blue: Vec4f,
    pub bad: Vec4f,
    pub coupon: Vec4f,
    pub coupon_off: Vec4f,
    pub track_off: Vec4f,
    pub heat_0: Vec4f,
    pub heat_1: Vec4f,
}

/// `#xRRGGBBAA` 那一个 u32 转成 shader 用的 0..1 向量。
fn rgba(c: u32) -> Vec4f {
    vec4(
        ((c >> 24) & 0xff) as f32 / 255.0,
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
    )
}

impl Default for Pal {
    fn default() -> Self {
        Self {
            ink: rgba(0xe7edf8ff),
            ink_2: rgba(0xa4b2c9ff),
            ink_3: rgba(0x7b8aa3ff),
            ink_ghost: rgba(0x3c4c6bff),
            line_soft: rgba(0x26364cff),
            warm: rgba(0xffca91ff),
            blue: rgba(0x82b5ffff),
            bad: rgba(0xff9eabff),
            coupon: rgba(0xffca91ff),
            coupon_off: rgba(0x8f7c66ff),
            track_off: rgba(0x24354fff),
            heat_0: rgba(0x2b3a52ff),
            heat_1: rgba(0x5b6f92ff),
        }
    }
}

impl Pal {
    /// 从当前 VM 的 `mod.ouyu` 里读一遍。
    pub fn read(cx: &mut Cx) -> Self {
        let mut p = Self::default();
        cx.with_vm(|vm| {
            let mut role = |name: LiveId, slot: &mut Vec4f| {
                let ouyu = vm.module(id!(ouyu));
                if let Some(c) = vm.bx.heap.value(ouyu, name.into(), NoTrap).as_color() {
                    *slot = rgba(c);
                }
            };
            role(live_id!(ink), &mut p.ink);
            role(live_id!(ink_2), &mut p.ink_2);
            role(live_id!(ink_3), &mut p.ink_3);
            role(live_id!(ink_ghost), &mut p.ink_ghost);
            role(live_id!(line_soft), &mut p.line_soft);
            role(live_id!(warm), &mut p.warm);
            role(live_id!(blue), &mut p.blue);
            role(live_id!(bad), &mut p.bad);
            role(live_id!(coupon), &mut p.coupon);
            role(live_id!(coupon_off), &mut p.coupon_off);
            role(live_id!(track_off), &mut p.track_off);
            role(live_id!(heat_0), &mut p.heat_0);
            role(live_id!(heat_1), &mut p.heat_1);
        });
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 从这个文件自己的源码里把两套色板的角色名抠出来。脚本堆没有「列出
    /// 一张表的键」的接口，而手抄一份名单迟早会和色板对不上 —— 对不上的
    /// 那天，测试反而是绿的。
    fn roles(table: &str) -> Vec<String> {
        let src = include_str!("theme.rs");
        let head = format!("mod.ouyu_themes.{} = {{", table);
        let start = src.find(&head).expect("色板没找到") + head.len();
        let end = start + src[start..].find("
    }").expect("色板没收尾");
        let mut out: Vec<String> = src[start..end]
            .lines()
            .filter_map(|line| {
                let line = line.split("//").next().unwrap_or("").trim();
                let (name, value) = line.split_once(':')?;
                value.trim().starts_with("#x").then(|| name.trim().to_string())
            })
            .collect();
        out.sort();
        out
    }

    /// 两套色板的角色名必须一模一样，而且每个都得解析成颜色。少一个角色
    /// 不会编译失败也不会报错 —— 只是那一处在那套主题下画成透明，而且多半
    /// 是在白昼版上：平时开着的是夜版，看不见。
    #[test]
    fn both_palettes_define_the_same_roles() {
        let dark = roles("dark");
        let light = roles("light");
        assert!(dark.len() > 40, "色板角色太少，大概没抠对: {}", dark.len());
        assert_eq!(dark, light, "两套色板的角色对不上");

        let mut cx = Cx::new(Box::new(|_, _| {}));
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            vm.bx.heap.new_module(id!(ouyu_themes));
            script_mod(vm);
            for table in ["dark", "light"] {
                let themes = vm.module(id!(ouyu_themes));
                for name in &dark {
                    let color = vm
                        .bx
                        .heap
                        .value_path(
                            themes,
                            &[LiveId::from_str(table), LiveId::from_str(name)],
                            NoTrap,
                        )
                        .as_color();
                    assert!(color.is_some(), "{}.{} 解析不出颜色", table, name);
                }
            }
        });
    }

    /// 两套主题下整份 DSL 都要能跑干净：预设里写错一个角色名（`ouyu.lnk`）
    /// 只是一条运行时错误，cargo check 照样通过，界面上那一处变透明。
    #[test]
    fn every_preset_evaluates_in_both_themes() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            for mode in ThemeMode::ALL {
                set_mode(mode);
                vm.bx.captured_errors = Some(Vec::new());
                vm.with_reload(|vm| {
                    install(vm);
                    crate::canvas::script_mod(vm);
                    crate::script_mod(vm);
                });
                let errors = vm.take_errors();
                assert!(errors.is_empty(), "{:?} 下有脚本错误: {:?}", mode, errors);
                assert!(
                    script_eval!(vm, { mod.ouyu.ink }).as_color().is_some(),
                    "{:?} 下 mod.ouyu 没绑上",
                    mode
                );
                assert!(
                    script_eval!(vm, { mod.widgets.OuyuInput }).as_object().is_some(),
                    "{:?} 下 OuyuInput 没注册",
                    mode
                );
            }
            set_mode(ThemeMode::Dark);
        });
    }
}
