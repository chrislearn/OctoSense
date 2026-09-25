//! 偶遇在桌面 AI bus 上的服务清单与匿名工具（02 H 节）。
//!
//! Phase 1 删除了具名工具 who_is_free / suggest_meetup / draft_invite /
//! send_invite；Phase 2 按 H 节目标以匿名聚合 / 通用建议形态重新加入：
//! - `get_area_opportunities`：当前是否有匿名机会（发布状态 × 草图场景模拟）、
//!   粗区域、时段桶、通用意愿列表。输出结构上就没有姓名 / 人数 / 联系方式字段。
//! - `suggest_activity`：一条通用活动建议 / 城市小签，不依赖他人行程。
//! 发布、互认、领奖始终由本人操作，AI 不能代签；隐藏记录不进入 AI（F 节）。

use crate::data::{ECHOES, INTENTS, OuyuState, SIGNS, SLOTS, day_label, minutes_of_day, today_days};
use makepad_app_module::makepad_ai_services::wire::{Risk, ServiceCall, ServiceManifest, ToolDef, ToolResult};

/// 本轮注册的工具列表。
pub const TOOL_NAMES: [&str; 2] = ["get_area_opportunities", "suggest_activity"];

const NO_ARGS: &str = r#"{"type":"object","properties":{}}"#;

pub fn manifest() -> ServiceManifest {
    ServiceManifest::new(
        "ouyu",
        "偶遇 OuYu",
        "偶遇演示应用：匿名机会 + 现场互认 + 私人回忆。只提供匿名聚合的机会查询与通用活动建议；输出永远没有姓名、人数或联系方式。发布、互认、领奖均由本人操作，不能代签；隐藏回忆不进入 AI。",
    )
    .with_tool(ToolDef::new(
        TOOL_NAMES[0],
        "当前匿名区域机会（聚合）：是否有机会、粗区域、时段桶、通用意愿列表。不含姓名、人数、联系方式。",
        NO_ARGS,
        Risk::Read,
    ))
    .with_tool(ToolDef::new(
        TOOL_NAMES[1],
        "一条通用活动建议 / 城市小签，不依赖他人行程。",
        NO_ARGS,
        Risk::Read,
    ))
}

/// 匿名机会快照：只有粗区域 / 时段桶 / 通用意愿下标。
/// 注意结构上没有姓名、人数、联系方式字段——H 节的硬要求由类型保证。
#[derive(Clone, Debug, Default)]
pub struct OpportunitySnapshot {
    /// 本人当前是否有还没到期的行踪。几条同时有效时，快照里只放最先结束的那条。
    pub published: bool,
    /// 草图场景模拟的机会条件是否满足（非稀疏场景）。
    pub opportunity: bool,
    pub day: Option<usize>,
    pub slot: Option<usize>,
    pub area: Option<u16>,
    pub intent: Option<usize>,
    pub echo: Option<usize>,
}

impl OpportunitySnapshot {
    /// 从应用状态取快照：只取行踪与回声，联系人 / 回忆根本不会进快照。
    pub fn from_state(state: &OuyuState, opportunity: bool) -> Self {
        let today = today_days();
        let active = state.active_publishes(today, minutes_of_day());
        let p = active.first().copied();
        Self {
            published: p.is_some(),
            opportunity,
            day: p.and_then(|p| p.day_at(today)),
            slot: p.map(|p| p.slot),
            area: p.map(|p| p.area),
            intent: p.map(|p| p.intent),
            echo: state.echo,
        }
    }
}

pub fn answer(snap: &OpportunitySnapshot, call: &ServiceCall) -> ToolResult {
    match call.tool.as_str() {
        "get_area_opportunities" => ToolResult::ok(
            &call.call_id,
            opportunities_json(snap),
            "匿名聚合输出：没有姓名、人数或联系方式字段",
        ),
        "suggest_activity" => ToolResult::ok(
            &call.call_id,
            suggestion_json(snap),
            "通用建议，不依赖他人行程",
        ),
        other => ToolResult::refused(
            &call.call_id,
            format!("ouyu 没有 `{other}` 工具；只有匿名机会查询与通用活动建议"),
        ),
    }
}

/// 手拼 JSON（字段固定、值来自常量表，无需序列化库）。
fn opportunities_json(snap: &OpportunitySnapshot) -> String {
    // 机会 = 草图场景条件满足（演示模拟；生产由匹配服务聚合 ≥5 位候选）。
    let has = snap.opportunity;
    let area = snap
        .published
        .then(|| snap.area.map(crate::areas::area_name))
        .flatten();
    let bucket = snap
        .published
        .then(|| {
            snap.day
                .zip(snap.slot)
                .map(|(d, s)| format!("{}{}", day_label(d), SLOTS[s.min(SLOTS.len() - 1)]))
        })
        .flatten();
    // 通用意愿：回声优先（本人当下想做的事），其次是发布意愿；都没有给空列表。
    let intents: Vec<&str> = if let Some(e) = snap.echo {
        vec![ECHOES[e.min(ECHOES.len() - 1)]]
    } else if let Some(i) = snap.intent.filter(|_| snap.published) {
        vec![INTENTS[i.min(INTENTS.len() - 1)]]
    } else {
        vec![]
    };
    let opt = |v: Option<String>| match v {
        Some(v) => format!("\"{v}\""),
        None => "null".into(),
    };
    let intents_json = intents
        .iter()
        .map(|i| format!("\"{i}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"has_opportunity\":{has},\"area\":{},\"time_bucket\":{},\"intents\":[{intents_json}],\"note\":\"匿名聚合：不包含姓名、人数、联系方式\"}}",
        opt(area.map(|s| s.to_string())),
        opt(bucket),
    )
}

fn suggestion_json(snap: &OpportunitySnapshot) -> String {
    // 确定性取一条城市小签：有发布按日期下标，否则第一条。
    let sign = SIGNS[snap.day.unwrap_or(0).min(SIGNS.len() - 1)];
    format!(
        "{{\"suggestion\":\"{sign}\",\"note\":\"通用活动建议，不依赖他人行程\"}}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::MemoryChoice;
    use makepad_app_module::makepad_ai_services::wire::ToolOutcome;

    fn call(tool: &str) -> ServiceCall {
        ServiceCall {
            call_id: "t1".into(),
            tool: tool.into(),
            args: "{}".into(),
        }
    }

    #[test]
    fn manifest_validates_with_two_read_tools() {
        let m = manifest();
        m.validate().unwrap();
        assert_eq!(m.tools.len(), 2);
        for name in TOOL_NAMES {
            let t = m.tool(name).expect("工具应在清单里");
            assert_eq!(t.risk, Risk::Read);
        }
    }

    #[test]
    fn unknown_tool_is_refused() {
        let snap = OpportunitySnapshot::default();
        let r = answer(&snap, &call("who_is_free"));
        assert_eq!(r.outcome, ToolOutcome::Refused);
    }

    #[test]
    fn opportunities_reflect_publish_and_scene() {
        let mut s = OuyuState::for_tests();
        s.publish(1, 1, 1, 0);
        s.set_echo(0);
        // 正常场景：有机会 + 发布的粗区域 / 时段桶 / 回声意愿。
        let snap = OpportunitySnapshot::from_state(&s, true);
        let r = answer(&snap, &call("get_area_opportunities"));
        assert_eq!(r.outcome, ToolOutcome::Ok);
        assert!(r.text.contains("\"has_opportunity\":true"), "{}", r.text);
        assert!(r.text.contains("\"area\":\"三里屯一带\""), "{}", r.text);
        assert!(r.text.contains("\"time_bucket\":\"明天下午\""), "{}", r.text);
        assert!(r.text.contains("\"咖啡\""), "{}", r.text);
        // 稀疏场景：条件不满足。
        let snap = OpportunitySnapshot::from_state(&s, false);
        let r = answer(&snap, &call("get_area_opportunities"));
        assert!(r.text.contains("\"has_opportunity\":false"), "{}", r.text);
        // 未发布：没有区域 / 时段。
        let s2 = OuyuState::for_tests();
        let snap = OpportunitySnapshot::from_state(&s2, true);
        let r = answer(&snap, &call("get_area_opportunities"));
        assert!(r.text.contains("\"area\":null"), "{}", r.text);
        assert!(r.text.contains("\"intents\":[]"), "{}", r.text);
    }

    #[test]
    fn suggest_activity_is_generic() {
        let snap = OpportunitySnapshot::default();
        let r = answer(&snap, &call("suggest_activity"));
        assert_eq!(r.outcome, ToolOutcome::Ok);
        assert!(r.text.contains("\"suggestion\":"), "{}", r.text);
        assert!(SIGNS.iter().any(|s| r.text.contains(s)), "{}", r.text);
    }

    #[test]
    fn outputs_never_contain_identity() {
        // H 节硬要求：任何联系人都不能出现在工具输出里（含隐藏回忆的称呼）。
        let mut s = OuyuState::for_tests();
        s.publish(1, 1, 1, 0);
        s.set_echo(1);
        s.push_memory(0, "林舟", MemoryChoice::Hidden);
        let snap = OpportunitySnapshot::from_state(&s, true);
        let mut forbidden: Vec<String> = s.contacts.iter().map(|c| c.label.clone()).collect();
        forbidden.extend(s.directory.iter().cloned());
        forbidden.extend(s.encounters.iter().map(|e| e.label_snapshot.clone()));
        for tool in TOOL_NAMES {
            let r = answer(&snap, &call(tool));
            assert_eq!(r.outcome, ToolOutcome::Ok);
            for name in &forbidden {
                assert!(!r.text.contains(name), "{tool} 输出泄露 {name}");
                assert!(!r.note.contains(name), "{tool} 备注泄露 {name}");
            }
        }
    }
}
