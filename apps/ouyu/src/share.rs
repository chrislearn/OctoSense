//! 分享卡（06 节）：900×1200 矢量海报的纯文本生成。
//!
//! 取舍说明：平台有 `Cx::encode_rgba_as_png` 与 `Texture::read_back`，但把
//! 一棵 widget 树离屏渲染到纹理再异步回读，在应用层没有先例（read_back 只有
//! 平台 remote / 测试在用），需要自管 DrawPass、跨后端验证成本高。本轮退而
//! 生成 SVG：零依赖、纯文本、可单测，浏览器 / 聊天工具都能直接打开看。
//!
//! 隐私边界（06 节硬要求）：场景结构里就没有姓名 / 地点 / 具体日期 / 隐藏
//! 记录字段；曲线默认不含，主动打开后也不标具体日期。

use crate::data::{AchievementStats, Milestone};

/// 海报样式：暖杏 / 夜蓝。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShareStyle {
    #[default]
    Warm,
    Night,
}

impl ShareStyle {
    fn bg(self) -> &'static str {
        match self {
            ShareStyle::Warm => "#f3d9b4",
            ShareStyle::Night => "#0e1830",
        }
    }
    fn text(self) -> &'static str {
        match self {
            ShareStyle::Warm => "#43331f",
            ShareStyle::Night => "#e7edf8",
        }
    }
    fn sub(self) -> &'static str {
        match self {
            ShareStyle::Warm => "#7a6647",
            ShareStyle::Night => "#a4b2c9",
        }
    }
    fn accent(self) -> &'static str {
        match self {
            ShareStyle::Warm => "#43331f",
            ShareStyle::Night => "#ffca91",
        }
    }
    fn ring(self) -> &'static str {
        match self {
            ShareStyle::Warm => "#d9bd93",
            ShareStyle::Night => "#26375c",
        }
    }
}

/// 分享卡的全部内容（只来自可见回忆的汇总统计与固定文案）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShareCardScene {
    pub style: ShareStyle,
    /// 我愿意记住的相遇次数。
    pub total: usize,
    /// 成就文案（最高已点亮里程碑；一个都没有时是留白文案）。
    pub milestone_title: String,
    pub milestone_sub: String,
    /// 可选的每周次数（「包含每周曲线」打开时才有；不标具体日期）。
    pub curve: Option<Vec<usize>>,
}

/// 从成就统计生成默认场景（不含曲线）。
pub fn scene_from_stats(stats: &AchievementStats, style: ShareStyle, ms: &[Milestone; 3]) -> ShareCardScene {
    // 取最高一档已点亮里程碑；都没有时是 06 节的留白文案。
    let (title, sub) = if ms[2].lit {
        ("把日常过成故事。".to_string(), "七个平常的日子，都在悄悄发光。".to_string())
    } else if ms[1].lit {
        ("生活有回响。".to_string(), "平常的日子，也有回响。".to_string())
    } else if ms[0].lit {
        ("第一次，刚刚好。".to_string(), "那些平常的日子，也在悄悄发光。".to_string())
    } else {
        ("下一次偶然，值得期待。".to_string(), String::new())
    };
    ShareCardScene {
        style,
        total: stats.remembered,
        milestone_title: title,
        milestone_sub: sub,
        curve: None,
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn text(x: f64, y: f64, size: u32, color: &str, weight: &str, body: &str) -> String {
    format!(
        r#"<text x="{x}" y="{y}" font-family="system-ui, 'PingFang SC', 'Microsoft YaHei', sans-serif" font-size="{size}" fill="{color}" font-weight="{weight}">{}</text>"#,
        esc(body)
    )
}

/// 生成 900×1200 SVG 海报。曲线（如有）只画折线与圆点，不标日期。
pub fn render_svg(s: &ShareCardScene) -> String {
    let (bg, fg, sub, accent, ring) = (
        s.style.bg(),
        s.style.text(),
        s.style.sub(),
        s.style.accent(),
        s.style.ring(),
    );
    let mut out = String::with_capacity(2048);
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    out.push('\n');
    out.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" width="900" height="1200" viewBox="0 0 900 1200">"#);
    out.push_str(&format!(r#"<rect width="900" height="1200" fill="{bg}"/>"#));
    // 装饰圆环（右上两个、左下一个），描边不填充。
    out.push_str(&format!(
        r#"<circle cx="800" cy="150" r="170" fill="none" stroke="{ring}" stroke-width="2"/>"#
    ));
    out.push_str(&format!(
        r#"<circle cx="800" cy="150" r="120" fill="none" stroke="{ring}" stroke-width="2"/>"#
    ));
    out.push_str(&format!(
        r#"<circle cx="60" cy="1010" r="200" fill="none" stroke="{ring}" stroke-width="2"/>"#
    ));
    // 正文。
    out.push_str(&text(64.0, 92.0, 22, sub, "normal", "OUYU / 我的相遇手记"));
    out.push_str(&text(64.0, 168.0, 40, fg, "normal", "★"));
    out.push_str(&text(64.0, 300.0, 64, fg, "bold", "给生活"));
    out.push_str(&text(64.0, 384.0, 64, fg, "bold", "留一点偶然。"));
    out.push_str(&text(64.0, 448.0, 26, sub, "normal", "不用专程约，也许刚好遇见。"));
    out.push_str(&text(64.0, 700.0, 200, accent, "bold", &s.total.to_string()));
    out.push_str(&text(64.0, 770.0, 28, fg, "normal", "次，我愿意记住的相遇"));
    out.push_str(&text(64.0, 876.0, 30, fg, "normal", &s.milestone_title));
    if !s.milestone_sub.is_empty() {
        out.push_str(&text(64.0, 922.0, 20, sub, "normal", &s.milestone_sub));
    }
    // 可选曲线：折线 + 圆点，不标日期（06 节：仍可能被关联推断，界面有提示）。
    if let Some(counts) = &s.curve {
        if !counts.is_empty() {
            let ymax = counts.iter().copied().max().unwrap_or(0).max(1) as f64;
            let (x0, x1, ybase, ytop) = (64.0_f64, 836.0_f64, 1040.0_f64, 960.0_f64);
            let n = counts.len();
            let px = |i: usize| {
                if n == 1 {
                    (x0 + x1) / 2.0
                } else {
                    x0 + (x1 - x0) * i as f64 / (n - 1) as f64
                }
            };
            let py = |v: usize| ybase - (ybase - ytop) * v as f64 / ymax;
            let pts: Vec<String> = counts
                .iter()
                .enumerate()
                .map(|(i, v)| format!("{:.1},{:.1}", px(i), py(*v)))
                .collect();
            out.push_str(&format!(
                r#"<polyline points="{}" fill="none" stroke="{accent}" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>"#,
                pts.join(" ")
            ));
            for (i, v) in counts.iter().enumerate() {
                out.push_str(&format!(
                    r#"<circle cx="{:.1}" cy="{:.1}" r="6" fill="{accent}"/>"#,
                    px(i),
                    py(*v)
                ));
            }
        }
    }
    // 底部署名。
    out.push_str(&format!(
        r#"<line x1="64" y1="1080" x2="836" y2="1080" stroke="{ring}" stroke-width="1.5"/>"#
    ));
    out.push_str(&text(64.0, 1130.0, 26, fg, "normal", "偶遇 OuYu"));
    out.push_str(&text(610.0, 1130.0, 16, sub, "normal", "个人记录 · 非社交排名"));
    out.push_str("</svg>");
    out
}

/// 分享卡保存路径：`<MAKEPAD_HOME>/ouyu/share-card.svg`。
pub fn share_card_file() -> Option<std::path::PathBuf> {
    std::env::var_os("MAKEPAD_HOME")
        .map(|h| std::path::Path::new(&h).join("ouyu").join("share-card.svg"))
}

/// 写分享卡文件（保存与预览是分开的动作；不代发不上传）。
pub fn save_share_card(svg: &str) -> std::io::Result<Option<std::path::PathBuf>> {
    let Some(path) = share_card_file() else {
        return Ok(None);
    };
    save_share_card_to(&path, svg)?;
    Ok(Some(path))
}

fn save_share_card_to(path: &std::path::Path, svg: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, svg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::*;

    fn stats_with(remembered: usize, days_apart: usize) -> AchievementStats {
        let today = civil_to_days(2026, 9, 17);
        let list: Vec<EncounterLocal> = (0..remembered)
            .map(|i| EncounterLocal {
                id: i,
                contact_id: Some(0),
                label_snapshot: format!("朋友{i}"),
                date: fmt_days(today - (i % days_apart.max(1)) as i64),
                hidden: false,
                note: String::new(),
            })
            .collect();
        achievement_stats(&list, 8, today)
    }

    #[test]
    fn scene_picks_highest_lit_milestone() {
        let s0 = stats_with(0, 1);
        let sc = scene_from_stats(&s0, ShareStyle::Warm, &milestones(&s0));
        assert_eq!(sc.milestone_title, "下一次偶然，值得期待。");
        assert!(sc.curve.is_none()); // 默认不含曲线

        let s1 = stats_with(1, 1);
        let sc = scene_from_stats(&s1, ShareStyle::Warm, &milestones(&s1));
        assert_eq!(sc.milestone_title, "第一次，刚刚好。");

        let s3 = stats_with(3, 3);
        let sc = scene_from_stats(&s3, ShareStyle::Night, &milestones(&s3));
        assert_eq!(sc.milestone_title, "生活有回响。");

        let s7 = stats_with(7, 7);
        let sc = scene_from_stats(&s7, ShareStyle::Night, &milestones(&s7));
        assert_eq!(sc.milestone_title, "把日常过成故事。");
    }

    #[test]
    fn svg_is_900x1200_and_has_signature() {
        let s = stats_with(2, 2);
        let sc = scene_from_stats(&s, ShareStyle::Warm, &milestones(&s));
        let svg = render_svg(&sc);
        assert!(svg.contains(r#"width="900" height="1200""#));
        assert!(svg.contains("偶遇 OuYu"));
        assert!(svg.contains("个人记录 · 非社交排名"));
        assert!(svg.contains("给生活"));
        assert!(svg.contains(">2</text>")); // 大数字
        assert!(!svg.contains("polyline")); // 默认无曲线
    }

    #[test]
    fn svg_curve_has_no_dates() {
        let s = stats_with(2, 2);
        let mut sc = scene_from_stats(&s, ShareStyle::Night, &milestones(&s));
        sc.curve = Some(s.weekly.iter().map(|w| w.count).collect());
        let svg = render_svg(&sc);
        assert!(svg.contains("polyline"));
        // 曲线不标具体日期：SVG 里没有任何 YYYY-MM-DD 或 MM/DD 文本。
        assert!(!svg.contains("2026-"));
        assert!(!svg.contains("09/"));
    }

    #[test]
    fn svg_has_no_identity_fields() {
        // 06 节硬要求：无联系人姓名 / 地点 / 具体日期 / 隐藏记录。
        let s = stats_with(3, 3);
        let sc = scene_from_stats(&s, ShareStyle::Warm, &milestones(&s));
        let svg = render_svg(&sc);
        for forbidden in ["朋友0", "朋友1", "林舟", "三里屯", "2026-09"] {
            assert!(!svg.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[test]
    fn svg_escapes_text() {
        assert_eq!(esc("a<&>b"), "a&lt;&amp;&gt;b");
    }

    #[test]
    fn save_writes_file() {
        // 不走 MAKEPAD_HOME 环境变量（并行测试会互相干扰），直接测写盘本体。
        let path = std::env::temp_dir()
            .join(format!("ouyu-share-{}/ouyu/share-card.svg", std::process::id()));
        let s = stats_with(1, 1);
        let sc = scene_from_stats(&s, ShareStyle::Warm, &milestones(&s));
        let svg = render_svg(&sc);
        save_share_card_to(&path, &svg).expect("写盘应成功");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), svg);
        let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }
}
