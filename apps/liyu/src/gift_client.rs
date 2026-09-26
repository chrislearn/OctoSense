//! Role-specific online gift views. Never deserialize the legacy whole-state JSON here.
use crate::profile_client;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone, Default)]
pub struct ReceivedGift {
    pub id: i64,
    pub state: String,
    pub unlock_kind: String,
    pub clue: String,
    pub attempts_left: i64,
    pub product_id: Option<u16>,
    pub price_cents: Option<i64>,
    pub physical: bool,
    pub message: String,
    pub contract_text: String,
    pub sender_name: Option<String>,
    pub voucher_code: Option<String>,
}

#[derive(Clone, Default)]
pub struct SentGift {
    pub id: i64,
    pub recipient_name: String,
    pub product_id: u16,
    pub price_cents: i64,
    pub state: String,
    pub carrier_delivered: bool,
    pub recipient_confirmed: bool,
}

#[derive(Clone, Default)]
pub struct Shipment {
    pub carrier: String,
    pub tracking_number: String,
    pub recipient_name: String,
    pub recipient_phone: String,
    pub recipient_address: String,
    pub carrier_delivered: bool,
    pub recipient_confirmed: bool,
    pub events: Vec<String>,
}

#[derive(Default)]
pub struct GiftClient {
    pub online: bool,
    pub inbox: Vec<ReceivedGift>,
    pub outbox: Vec<SentGift>,
    pub received_detail: Option<ReceivedGift>,
    pub sent_detail: Option<SentGift>,
    pub shipment: Option<Shipment>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(600))
        .build()
}

fn get(path: &str) -> Result<Option<Value>, &'static str> {
    let Some((url, token)) = profile_client::session() else {
        return Ok(None);
    };
    match agent()
        .get(&format!("{url}/api/v1/{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
    {
        Ok(reply) => reply.into_json().map(Some).map_err(|_| "服务器响应无效"),
        Err(ureq::Error::Transport(_)) => Ok(None),
        Err(ureq::Error::Status(404, _)) => Err("礼物或物流信息不存在"),
        Err(ureq::Error::Status(_, _)) => Err("服务器暂时无法读取礼盒"),
    }
}

fn post(path: &str, body: Value) -> Result<Value, &'static str> {
    let Some((url, token)) = profile_client::session() else {
        return Err("服务器未连接，不能操作在线礼物");
    };
    match agent()
        .post(&format!("{url}/api/v1/{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .send_json(body)
    {
        Ok(reply) => reply.into_json().map_err(|_| "服务器响应无效"),
        Err(ureq::Error::Transport(_)) => Err("服务器断开，在线礼物未操作；请刷新礼盒"),
        Err(ureq::Error::Status(409, _)) => Err("礼物状态已改变，请刷新"),
        Err(ureq::Error::Status(404, _)) => Err("礼物不存在或无权操作"),
        Err(ureq::Error::Status(_, _)) => Err("服务器未接受操作，请检查输入"),
    }
}

fn received(v: &Value) -> Option<ReceivedGift> {
    Some(ReceivedGift {
        id: v.get("id")?.as_i64()?,
        state: v.get("state")?.as_str()?.into(),
        unlock_kind: v.get("unlock_kind")?.as_str()?.into(),
        clue: v.get("clue")?.as_str()?.into(),
        attempts_left: v.get("attempts_left")?.as_i64()?,
        product_id: v
            .get("product_id")
            .and_then(Value::as_u64)
            .and_then(|x| u16::try_from(x).ok()),
        price_cents: v.get("price_cents").and_then(Value::as_i64),
        physical: v.get("physical").and_then(Value::as_bool).unwrap_or(false),
        message: v
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        contract_text: v
            .get("contract_text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        sender_name: v
            .get("sender")
            .and_then(|x| x.get("display_name"))
            .and_then(Value::as_str)
            .map(Into::into),
        voucher_code: v
            .get("voucher_code")
            .and_then(Value::as_str)
            .map(Into::into),
    })
}

fn sent(v: &Value) -> Option<SentGift> {
    Some(SentGift {
        id: v.get("id")?.as_i64()?,
        recipient_name: v.get("recipient")?.get("display_name")?.as_str()?.into(),
        product_id: u16::try_from(v.get("product_id")?.as_u64()?).ok()?,
        price_cents: v.get("price_cents")?.as_i64()?,
        state: v.get("state")?.as_str()?.into(),
        carrier_delivered: v.get("carrier_delivered")?.as_bool()?,
        recipient_confirmed: v.get("recipient_confirmed")?.as_bool()?,
    })
}

fn shipment(v: &Value) -> Option<Shipment> {
    let recipient = v.get("recipient")?;
    Some(Shipment {
        carrier: v.get("carrier")?.as_str()?.into(),
        tracking_number: v.get("tracking_number")?.as_str()?.into(),
        recipient_name: recipient.get("name")?.as_str()?.into(),
        recipient_phone: recipient.get("phone")?.as_str()?.into(),
        recipient_address: recipient.get("address")?.as_str()?.into(),
        carrier_delivered: v.get("carrier_delivered")?.as_bool()?,
        recipient_confirmed: v.get("recipient_confirmed")?.as_bool()?,
        events: v
            .get("events")?
            .as_array()?
            .iter()
            .filter_map(|e| {
                Some(format!(
                    "{} · {}",
                    e.get("at")?.as_str()?,
                    e.get("description")?.as_str()?
                ))
            })
            .collect(),
    })
}

impl GiftClient {
    pub fn refresh(&mut self) -> Result<(), &'static str> {
        let inbox = match get("gifts/inbox") {
            Ok(value) => value,
            Err(err) => {
                self.clear_online();
                return Err(err);
            }
        };
        let outbox = match get("gifts/outbox") {
            Ok(value) => value,
            Err(err) => {
                self.clear_online();
                return Err(err);
            }
        };
        if let (Some(inbox), Some(outbox)) = (inbox, outbox) {
            self.inbox = inbox
                .as_array()
                .ok_or("服务器礼盒响应无效")?
                .iter()
                .filter_map(received)
                .collect();
            self.outbox = outbox
                .as_array()
                .ok_or("服务器礼盒响应无效")?
                .iter()
                .filter_map(sent)
                .collect();
            self.online = true;
        } else {
            self.clear_online();
        }
        Ok(())
    }

    fn clear_online(&mut self) {
        self.online = false;
        self.inbox.clear();
        self.outbox.clear();
        self.received_detail = None;
        self.sent_detail = None;
        self.shipment = None;
    }

    pub fn detail(&mut self, id: i64, sent_role: bool) -> Result<(), &'static str> {
        let value = match get(&format!("gifts/{id}")) {
            Ok(Some(value)) => value,
            Ok(None) => {
                self.online = false;
                return Err("服务器断开，请返回本地演示礼盒");
            }
            Err(err) => return Err(err),
        };
        self.received_detail = None;
        self.sent_detail = None;
        self.shipment = None;
        if sent_role {
            self.sent_detail = Some(sent(&value).ok_or("送礼详情响应无效")?);
            // This endpoint returns only two booleans. Do not request /shipments as sender.
            let Some(status) = get(&format!("gifts/{id}/delivery-summary"))? else {
                self.online = false;
                return Err("服务器断开，请返回本地演示礼盒");
            };
            if let Some(row) = &mut self.sent_detail {
                row.carrier_delivered = status
                    .get("carrier_delivered")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                row.recipient_confirmed = status
                    .get("recipient_confirmed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            }
        } else {
            let row = received(&value).ok_or("收礼详情响应无效")?;
            if row.state == "accepted" && row.physical {
                let Some(detail) = get(&format!("shipments/{id}"))? else {
                    self.online = false;
                    return Err("服务器断开，请返回本地演示礼盒");
                };
                self.shipment = shipment(&detail);
            }
            self.received_detail = Some(row);
        }
        Ok(())
    }

    pub fn open(&mut self, id: i64) -> Result<(), &'static str> {
        self.call(&format!("gifts/{id}/open"), json!({}))?;
        self.detail(id, false)
    }
    pub fn answer(&mut self, id: i64, answer: &str) -> Result<(), &'static str> {
        if answer.trim().is_empty() {
            return Err("请填写答案");
        }
        self.call(&format!("gifts/{id}/answer"), json!({"answer":answer}))?;
        self.detail(id, false)
    }
    pub fn accept(
        &mut self,
        id: i64,
        agree: bool,
        name: &str,
        phone: &str,
        address: &str,
    ) -> Result<(), &'static str> {
        self.call(
            &format!("gifts/{id}/accept"),
            json!({"agree":agree,"recipient_name":name,
            "recipient_phone":phone,"recipient_address":address}),
        )?;
        self.detail(id, false)
    }
    pub fn confirm(&mut self, id: i64) -> Result<(), &'static str> {
        self.call(&format!("shipments/{id}/confirm-receipt"), json!({}))?;
        self.detail(id, false)
    }
    fn call(&mut self, path: &str, body: Value) -> Result<(), &'static str> {
        match post(path, body) {
            Ok(_) => Ok(()),
            Err(err) => {
                if err.contains("断开") || err.contains("未连接") {
                    self.online = false;
                }
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sealed_inbox_and_sender_projection_keep_roles_apart() {
        let sealed = received(
            &json!({"id":1,"state":"sealed","unlock_kind":"free","clue":"","attempts_left":3}),
        )
        .unwrap();
        assert!(sealed.product_id.is_none());
        assert!(sealed.sender_name.is_none());
        let sender = sent(&json!({"id":1,"recipient":{"display_name":"林舟"},"product_id":4,
            "price_cents":9800,"state":"handled","carrier_delivered":true,"recipient_confirmed":false})).unwrap();
        assert_eq!(sender.state, "handled");
        assert!(sender.carrier_delivered);
    }
}
