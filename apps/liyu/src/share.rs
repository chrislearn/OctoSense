//! 神秘礼卡：900×1200 矢量图的纯文本生成（界面上的预览见 canvas.rs 的 LiyuShareCard）。
//!
//! 为什么是 SVG：离屏渲染一棵 widget 树再回读纹理，在应用层没有先例；SVG 零依赖、
//! 纯文本、可单测，浏览器 / 聊天工具都能直接打开。
//!
//! 隐私边界（04-rules 隐私一节）：礼卡会被截图、会被转发，所以场景结构里**只有**玩法和
//! 线索 / 问题 / 暗号提示（外加定文件名用的礼物 id）。礼物名、价格、送礼人、答案、寄语根本不进场景 ——
//! 由类型保证，不靠调用方记得。

use crate::data::{Gift, Unlock, MAX_ATTEMPTS};

/// 礼卡样式：暖杏 / 夜蓝。
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

/// 礼卡上的全部内容。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShareCardScene {
    pub style: ShareStyle,
    /// 「玩法：猜我是谁 · 3 次机会」。
    pub play: String,
    /// 「TA 留下的线索」；直接领取时为空。
    pub prompt_title: String,
    /// 线索 / 问题 / 暗号提示，已按行切好。
    pub prompt_lines: Vec<String>,
    /// 礼物 id，只用来给保存的文件起名，不画在卡上。
    pub id: u64,
}

/// 每行最多几个字（900 宽、40 号字、左右各留 64）。
pub const LINE_CHARS: usize = 16;

/// 按字数切行。线索最多 30 字，所以最多两行。
pub fn wrap_chars(s: &str, n: usize) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    chars.chunks(n.max(1)).map(|c| c.iter().collect()).collect()
}

/// 从一份送出的礼物取礼卡场景。只拷玩法和线索。
pub fn scene_for(g: &Gift, style: ShareStyle) -> ShareCardScene {
    let u = g.unlock();
    let play = match u {
        Unlock::Free => format!("玩法：{} · 打开就能领", u.short()),
        _ => format!("玩法：{} · {} 次机会", u.short(), MAX_ATTEMPTS),
    };
    let prompt = match u {
        Unlock::Free => String::new(),
        Unlock::Passphrase if g.clue.trim().is_empty() => "TA 说：你知道的".to_string(),
        _ => format!("「{}」", g.clue.trim()),
    };
    ShareCardScene {
        style,
        play,
        prompt_title: u.clue_title().to_string(),
        prompt_lines: if prompt.is_empty() { Vec::new() } else { wrap_chars(&prompt, LINE_CHARS) },
        id: g.id,
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn text(x: f64, y: f64, size: u32, color: &str, weight: &str, body: &str) -> String {
    format!(
        r#"<text x="{x}" y="{y}" font-family="system-ui, 'PingFang SC', 'Microsoft YaHei', sans-serif" font-size="{size}" fill="{color}" font-weight="{weight}">{}</text>"#,
        esc(body)
    )
}

/// 生成 900×1200 SVG 礼卡。版式与 canvas.rs 的预览一致。
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
    // 礼盒徽记：两道圆环 + 中间一个问号。
    for r in [170, 120] {
        out.push_str(&format!(
            r#"<circle cx="450" cy="330" r="{r}" fill="none" stroke="{ring}" stroke-width="2"/>"#
        ));
    }
    out.push_str(&format!(
        r#"<text x="450" y="390" text-anchor="middle" font-family="system-ui, sans-serif" font-size="170" fill="{accent}" font-weight="bold">?</text>"#
    ));
    out.push_str(&text(64.0, 92.0, 22, sub, "normal", "LIYU / 神秘礼卡"));
    out.push_str(&text(64.0, 600.0, 52, fg, "bold", "一份神秘礼物 · 等你来拆"));
    out.push_str(&text(64.0, 656.0, 26, sub, "normal", &s.play));
    let mut y = 740.0;
    if !s.prompt_lines.is_empty() {
        out.push_str(&text(64.0, y, 24, sub, "normal", &s.prompt_title));
        y += 58.0;
        for line in &s.prompt_lines {
            out.push_str(&text(64.0, y, 40, fg, "normal", line));
            y += 54.0;
        }
    }
    out.push_str(&text(64.0, 1010.0, 36, accent, "bold", CARD_SECRET));
    out.push_str(&format!(
        r#"<line x1="64" y1="1080" x2="836" y2="1080" stroke="{ring}" stroke-width="1.5"/>"#
    ));
    out.push_str(&text(64.0, 1130.0, 26, fg, "normal", "礼遇 LiYu"));
    out.push_str(&text(530.0, 1130.0, 18, sub, "normal", CARD_FOOT));
    out.push_str("</svg>");
    out
}

/// 礼卡底部那句：拆开之前什么都不透露。
pub const CARD_SECRET: &str = "拆开之前，是什么、谁送的都保密";
/// 右下角的小字。
pub const CARD_FOOT: &str = "礼遇 · 点开链接就能拆";

/// 礼卡保存路径：`<MAKEPAD_HOME>/liyu/cards/gift-<id>.svg`。
pub fn card_file(id: u64) -> Option<std::path::PathBuf> {
    crate::data::LiyuState::data_dir().map(|d| d.join("cards").join(format!("gift-{id}.svg")))
}

/// 写礼卡文件（只落本机，不代发不上传）。没有 MAKEPAD_HOME 时返回 Ok(None)。
pub fn save_card(scene: &ShareCardScene) -> std::io::Result<Option<std::path::PathBuf>> {
    let Some(path) = card_file(scene.id) else {
        return Ok(None);
    };
    save_card_to(&path, scene)?;
    Ok(Some(path))
}

fn save_card_to(path: &std::path::Path, scene: &ShareCardScene) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, render_svg(scene))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::*;

    fn every_sent_card(s: &LiyuState) -> Vec<(Gift, String)> {
        s.gifts
            .iter()
            .map(|g| (g.clone(), render_svg(&scene_for(g, ShareStyle::Warm))))
            .collect()
    }

    #[test]
    fn card_never_leaks_gift_price_sender_or_answer() {
        let mut s = LiyuState::for_tests();
        // 再送几份各种玩法的，覆盖所有分支。
        for (k, u) in Unlock::ALL.into_iter().enumerate() {
            let d = SendDraft {
                item: k as u16 * 3,
                peer: "林舟".into(),
                unlock: u,
                clue: "我们在哪认识的".into(),
                answer: "图书馆".into(),
                contract: Some("周末陪我看一场电影".into()),
                message: "天冷了多穿点".into(),
                use_balance: true,
                ..Default::default()
            };
            s.send_gift(&d, TEST_TODAY).unwrap();
        }
        for (g, svg) in every_sent_card(&s) {
            let name = g.catalog().name;
            assert!(!svg.contains(name), "礼卡泄露礼物名 {name}");
            assert!(!svg.contains(&yuan(g.price)), "礼卡泄露价格");
            assert!(!svg.contains('¥'), "礼卡上不该有金额");
            for alias in split_aliases(&g.peer) {
                assert!(!svg.contains(&alias), "礼卡泄露送礼人 / 收礼人 {alias}");
            }
            if !g.answer.is_empty() {
                assert!(!svg.contains(&g.answer), "礼卡泄露答案 {}", g.answer);
            }
            if !g.message.is_empty() {
                assert!(!svg.contains(&g.message), "礼卡带了寄语");
            }
            if !g.contract.is_empty() {
                assert!(!svg.contains(&g.contract), "礼卡带了契约");
            }
            assert!(svg.contains(CARD_SECRET), "礼卡要有那句保密说明");
        }
    }

    #[test]
    fn card_shows_play_and_clue() {
        let s = LiyuState::for_tests();
        let g = s.gifts.iter().find(|g| g.unlock() == Unlock::Question).unwrap();
        let sc = scene_for(g, ShareStyle::Night);
        assert_eq!(sc.play, "玩法：私密问答 · 3 次机会");
        assert_eq!(sc.prompt_title, "TA 的问题");
        assert_eq!(sc.prompt_lines.concat(), format!("「{}」", g.clue));
        let svg = render_svg(&sc);
        assert!(svg.starts_with("<?xml"));
        assert!(svg.contains(r#"width="900" height="1200""#));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn free_and_hintless_cards() {
        let mut s = LiyuState::for_tests();
        let mut d = SendDraft { item: 1, unlock: Unlock::Passphrase, answer: "芝麻开门".into(), ..Default::default() };
        let id = s.send_gift(&d, TEST_TODAY).unwrap();
        let sc = scene_for(s.gift(id).unwrap(), ShareStyle::Warm);
        assert_eq!(sc.prompt_lines, vec!["TA 说：你知道的".to_string()]);
        d.unlock = Unlock::Free;
        let id = s.send_gift(&d, TEST_TODAY).unwrap();
        let sc = scene_for(s.gift(id).unwrap(), ShareStyle::Warm);
        assert!(sc.prompt_lines.is_empty());
        assert_eq!(sc.play, "玩法：直接领取 · 打开就能领");
    }

    #[test]
    fn long_clue_wraps_into_two_lines() {
        let lines = wrap_chars(&"线".repeat(32), LINE_CHARS);
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|l| l.chars().count() <= LINE_CHARS));
    }

    #[test]
    fn text_is_escaped() {
        let sc = ShareCardScene { prompt_lines: vec!["<b>&".into()], prompt_title: "t".into(), ..Default::default() };
        let svg = render_svg(&sc);
        assert!(svg.contains("&lt;b&gt;&amp;"));
        assert!(!svg.contains("<b>"));
    }

    #[test]
    fn save_writes_svg_file() {
        let dir = std::env::temp_dir().join(format!("liyu-card-{}", std::process::id()));
        let path = dir.join("cards").join("gift-7.svg");
        let sc = ShareCardScene { id: 7, ..Default::default() };
        save_card_to(&path, &sc).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("礼遇 LiYu"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
