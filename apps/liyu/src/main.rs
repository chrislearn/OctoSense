//! 「礼遇 LiYu」的独立窗口入口（模块形态由 lib.rs 的 LiyuModule 承载）。
//! 注册三个只读 AI 工具（礼物目录 / 预算挑礼 / 礼盒摘要，见 ai.rs）：
//! 应答经 LiyuView::ai_answer 取礼盒的匿名汇总，结构上不含送礼人、答案、暗号与口令。
pub use makepad_widgets;
use makepad_app_module::makepad_ai_services::port::{AiServicePort, PortEvent};
use makepad_app_module::makepad_ai_services::wire::{HostedDown, ServiceDown, ToolResult};
use makepad_widgets::*;
use octosense_liyu::theme::ThemeMode;
use octosense_liyu::LiyuView;

app_main!(
    App,
    font_set: International,
    font_assets: ["makepad_widgets/resources/NotoColorEmoji.ttf"]
);

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    startup() do #(App::script_component(vm)) {
        ui: Root {
            main_window := Window {
                window.title: "礼遇 LiYu"
                window.inner_size: vec2(1280, 800)
                // 独立窗口时 pass 与标题栏也用礼遇的底色，免得露出默认灰。
                pass +: { clear_color: mod.liyu.bg }
                caption_bar.draw_bg.color: mod.liyu.bg
                body := LiyuView {}
            }
        }
    }
}

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    /// 面向宿主助手的 AI bus 端口（standalone 时为 in-process 停靠口）。
    #[rust]
    ai_port: Option<AiServicePort>,
    /// 注册应答（Registered）可能因子进程启动竞态被宿主丢掉: 2s 后没学到 endpoint 就重报一次。
    #[rust]
    register_retry: Timer,
    /// 上一拍看到的深浅。窗口底色和标题栏写在这个 script_mod 里，不在
    /// LiyuView 里，所以设置页换主题之后得让整个 App 重新 bake 一次。
    #[rust]
    seen_theme: Option<ThemeMode>,
}

impl App {
    fn answer_ai(&self, cx: &mut Cx, call: &makepad_app_module::makepad_ai_services::wire::ServiceCall) -> ToolResult {
        // ui 是 Root，LiyuView 在 main_window.body——直接 borrow Root 永远落空。
        self.ui
            .widget(cx, ids!(main_window.body))
            .borrow::<LiyuView>()
            .map(|view| view.ai_answer(call))
            .unwrap_or_else(|| ToolResult::unavailable(&call.call_id, "礼遇窗口还没准备好"))
    }

    fn ensure_ai_port(&mut self, cx: &mut Cx) {
        if self.ai_port.is_none() {
            self.ai_port = AiServicePort::open(cx, octosense_liyu::ai::manifest());
            self.register_retry = cx.start_timeout(2.0);
        }
    }

    fn drain_ai_port(&mut self, cx: &mut Cx, event: &Event) {
        if self.register_retry.is_event(event).is_some() {
            if let Some(port) = self.ai_port.as_ref() {
                // 只在托管进程重试; standalone 的 in-process 停靠口永远没 endpoint,
                // 重报只会多停一份 link。
                if cx.in_makepad_studio() && port.endpoint().is_none() {
                    log!("liyu: no Registered ack after 2s, re-announcing AI service");
                    port.register();
                }
            }
        }
        // OctoSense 宿主收到 Register 只存 manifest、不回 Registered, port 学不到
        // endpoint, 地址过滤会把所有 Call 帧丢掉。endpoint 未知时直接按帧应答;
        // 在会回 Registered 的宿主（makepad WM）下仍走 port 的正常路径。
        if self.ai_port.as_ref().is_some_and(|p| p.endpoint().is_none()) {
            if let Event::Custom(json) = event {
                if let Some(down) = HostedDown::parse(json) {
                    if let ServiceDown::Call(call) = down.msg {
                        log!("liyu: answering {} via endpoint-unknown fallback", call.tool);
                        let result = self.answer_ai(cx, &call);
                        if let Some(port) = self.ai_port.as_ref() {
                            port.reply(result);
                        }
                        return;
                    }
                }
            }
        }
        let events = match self.ai_port.as_mut() {
            Some(port) => port.handle_event(cx, event),
            None => return,
        };
        for event in events {
            match event {
                PortEvent::Registered(endpoint) => {
                    log!("liyu: AI service registered as {}", endpoint.as_str());
                }
                PortEvent::Call(call) => {
                    log!("liyu: port call {}", call.tool);
                    let result = self.answer_ai(cx, &call);
                    if let Some(port) = self.ai_port.as_ref() {
                        port.reply(result);
                    }
                }
                PortEvent::Cancel { .. } => {}
                PortEvent::Subscribe { .. } | PortEvent::Unsubscribe { .. } => {}
                PortEvent::ChatOpen { .. } => {}
            }
        }
    }
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        makepad_widgets::script_mod(vm);
        octosense_liyu::theme::install(vm);
        octosense_liyu::canvas::script_mod(vm);
        octosense_liyu::script_mod(vm);
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ensure_ai_port(cx);
        self.drain_ai_port(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
        let mode = octosense_liyu::theme::mode();
        match self.seen_theme {
            None => self.seen_theme = Some(mode),
            Some(seen) if seen != mode => {
                self.seen_theme = Some(mode);
                // Rebake 会重跑 script_mod，mod.liyu 已经指向新的一套色，
                // 窗口那两处跟着换；LiyuView 收到 LiveEdit 再把数据铺回去。
                cx.request_live_edit();
            }
            Some(_) => {}
        }
    }
}
