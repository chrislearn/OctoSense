//! 「偶遇 OuYu」的独立窗口入口（模块形态由 lib.rs 的 OuyuModule 承载）。
//! Phase 2 起注册两个匿名 Read 工具（get_area_opportunities / suggest_activity）：
//! 应答经 OuyuView::ai_answer 取匿名快照，结构上不含姓名 / 人数 / 联系方式。
pub use makepad_widgets;
use makepad_app_module::makepad_ai_services::port::{AiServicePort, PortEvent};
use makepad_app_module::makepad_ai_services::wire::{HostedDown, ServiceDown, ToolResult};
use makepad_widgets::*;
use octosense_ouyu::theme::ThemeMode;
use octosense_ouyu::OuyuView;

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
                window.title: "偶遇 OuYu"
                window.inner_size: vec2(1280, 800)
                // 独立窗口时 pass 与标题栏也用偶遇的底色，免得露出默认灰。
                pass +: { clear_color: mod.ouyu.bg }
                caption_bar.draw_bg.color: mod.ouyu.bg
                body := OuyuView {}
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
    /// OuyuView 里，所以设置页换主题之后得让整个 App 重新 bake 一次。
    #[rust]
    seen_theme: Option<ThemeMode>,
}

impl App {
    fn answer_ai(&self, cx: &mut Cx, call: &makepad_app_module::makepad_ai_services::wire::ServiceCall) -> ToolResult {
        // ui 是 Root，OuyuView 在 main_window.body——直接 borrow Root 永远落空。
        self.ui
            .widget(cx, ids!(main_window.body))
            .borrow::<OuyuView>()
            .map(|view| view.ai_answer(call))
            .unwrap_or_else(|| ToolResult::unavailable(&call.call_id, "偶遇窗口还没准备好"))
    }

    fn ensure_ai_port(&mut self, cx: &mut Cx) {
        if self.ai_port.is_none() {
            self.ai_port = AiServicePort::open(cx, octosense_ouyu::ai::manifest());
            self.register_retry = cx.start_timeout(2.0);
        }
    }

    fn drain_ai_port(&mut self, cx: &mut Cx, event: &Event) {
        if self.register_retry.is_event(event).is_some() {
            if let Some(port) = self.ai_port.as_ref() {
                // 只在托管进程重试; standalone 的 in-process 停靠口永远没 endpoint,
                // 重报只会多停一份 link。
                if cx.in_makepad_studio() && port.endpoint().is_none() {
                    log!("ouyu: no Registered ack after 2s, re-announcing AI service");
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
                        log!("ouyu: answering {} via endpoint-unknown fallback", call.tool);
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
                    log!("ouyu: AI service registered as {}", endpoint.as_str());
                }
                PortEvent::Call(call) => {
                    log!("ouyu: port call {}", call.tool);
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
        octosense_ouyu::theme::install(vm);
        octosense_ouyu::canvas::script_mod(vm);
        octosense_ouyu::script_mod(vm);
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ensure_ai_port(cx);
        self.drain_ai_port(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
        let mode = octosense_ouyu::theme::mode();
        match self.seen_theme {
            None => self.seen_theme = Some(mode),
            Some(seen) if seen != mode => {
                self.seen_theme = Some(mode);
                // Rebake 会重跑 script_mod，mod.ouyu 已经指向新的一套色，
                // 窗口那两处跟着换；OuyuView 收到 LiveEdit 再把数据铺回去。
                cx.request_live_edit();
            }
            Some(_) => {}
        }
    }
}
