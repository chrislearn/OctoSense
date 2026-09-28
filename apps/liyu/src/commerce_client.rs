//! Buyer-only direct gift checkout. Server failures never simulate success.
use serde_json::{json, Value};
use std::time::Duration;

use crate::{data::CATALOG, profile_client};

#[derive(Clone, Default)]
pub struct Order {
    pub id: i64,
    pub total_cents: i64,
    pub status: String,
}

#[derive(Clone, Default)]
pub struct ProductDetail {
    pub name: String,
    pub brand: String,
    pub description: String,
    pub price_cents: i64,
    pub available: bool,
}

pub fn product_detail(id: u16) -> Option<ProductDetail> {
    let value = json_get(&format!("catalog/{id}")).ok()??;
    Some(ProductDetail {
        name: value.get("name")?.as_str()?.into(),
        brand: value.get("brand")?.as_str()?.into(),
        description: value.get("description")?.as_str()?.into(),
        price_cents: value.get("price_cents")?.as_i64()?,
        available: value.get("available")?.as_bool()?,
    })
}

#[derive(Default)]
pub struct Commerce {
    pub online: bool,
    pub order: Option<Order>,
    request_key: Option<(String, String)>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build()
}

fn json_get(path: &str) -> Result<Option<Value>, &'static str> {
    let Some((url, token)) = profile_client::session() else {
        return Err("请先登录礼遇账号");
    };
    match agent()
        .get(&format!("{url}/api/v1/{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
    {
        Ok(reply) => reply.into_json().map(Some).map_err(|_| "服务器响应无效"),
        Err(ureq::Error::Transport(_)) => Err("无法连接礼遇服务器，请稍后重试"),
        Err(ureq::Error::Status(_, _)) => Err("服务器暂时无法读取数据"),
    }
}

fn json_post(path: &str, body: Value, key: Option<&str>) -> Result<Option<Value>, &'static str> {
    let Some((url, token)) = profile_client::session() else {
        return Err("请先登录礼遇账号");
    };
    let mut request = agent()
        .post(&format!("{url}/api/v1/{path}"))
        .set("Authorization", &format!("Bearer {token}"));
    if let Some(key) = key {
        request = request.set("Idempotency-Key", key);
    }
    match request.send_json(body) {
        Ok(reply) => reply.into_json().map(Some).map_err(|_| "服务器响应无效"),
        Err(ureq::Error::Transport(_)) => Err("无法连接礼遇服务器，请稍后重试"),
        Err(ureq::Error::Status(403, _)) => Err("无权执行此操作"),
        Err(ureq::Error::Status(404, _)) => Err("收礼人或订单不存在"),
        Err(ureq::Error::Status(409, _)) => Err("商品、收件人或订单状态已改变，请检查后重试"),
        Err(ureq::Error::Status(_, _)) => Err("服务器未接受操作"),
    }
}

impl Commerce {
    pub fn create_order(
        &mut self,
        product_id: u16,
        kind: &str,
        value: &str,
        label: &str,
    ) -> Result<Order, &'static str> {
        if product_id as usize >= CATALOG.len() {
            return Err("商品不存在");
        }
        let value = crate::contacts::normalize(kind, value).ok_or("请输入有效收件手机号或邮箱")?;
        let body =
            json!({"product_id":product_id,"recipient":{"kind":kind,"value":value,"label":label}});
        let fingerprint = body.to_string();
        if !self
            .request_key
            .as_ref()
            .is_some_and(|(request, _)| request == &fingerprint)
        {
            let quote = json_post("orders/quote", body.clone(), None)?.ok_or("报价响应无效")?;
            if quote["total_cents"].as_i64().is_none_or(|total| total < 0) {
                return Err("报价响应无效");
            }
        }
        let key = self
            .request_key
            .as_ref()
            .filter(|(request, _)| request == &fingerprint)
            .map(|(_, key)| key.clone())
            .unwrap_or_else(|| {
                format!(
                    "liyu-direct-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                )
            });
        self.request_key = Some((fingerprint, key.clone()));
        let value = json_post("orders", body, Some(&key))?.ok_or("下单响应无效")?;
        let order = Order {
            id: value["id"].as_i64().ok_or("订单响应无效")?,
            total_cents: value["total_cents"].as_i64().ok_or("订单响应无效")?,
            status: value["status"].as_str().unwrap_or("pending").into(),
        };
        self.request_key = None;
        self.online = true;
        self.order = Some(order.clone());
        Ok(order)
    }

    pub fn pay_test(&mut self) -> Result<Order, &'static str> {
        let Some(mut order) = self.order.clone() else {
            return Err("还没有订单");
        };
        let value = json_post(&format!("orders/{}/pay-test", order.id), json!({}), None)?
            .ok_or("支付响应无效")?;
        order.status = value["status"].as_str().unwrap_or("paid_test").into();
        self.order = Some(order.clone());
        Ok(order)
    }
}

#[derive(Clone)]
pub struct Notification {
    pub id: i64,
    pub gift_id: Option<u64>,
    pub title: String,
    pub body: String,
}
pub fn unread_notifications() -> Result<Vec<Notification>, &'static str> {
    let value = json_get("notifications")?.ok_or("通知响应无效")?;
    Ok(value
        .as_array()
        .ok_or("通知响应无效")?
        .iter()
        .filter(|v| v.get("read_at").is_none_or(Value::is_null))
        .filter_map(|v| {
            Some(Notification {
                id: v["id"].as_i64()?,
                gift_id: v["gift_id"].as_u64(),
                title: v["title"].as_str()?.into(),
                body: v["body"].as_str()?.into(),
            })
        })
        .collect())
}
pub fn read_notification(id: i64) -> Result<(), &'static str> {
    json_post(&format!("notifications/{id}/read"), json!({}), None)?.ok_or("通知响应无效")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_order_requires_login_and_valid_recipient() {
        let mut c = Commerce::default();
        assert!(c.create_order(0, "email", "bad", "朋友").is_err());
        assert!(c
            .create_order(0, "email", "friend@example.test", "朋友")
            .is_err());
        assert!(c.pay_test().is_err());
        assert!(c.order.is_none());
    }
}
