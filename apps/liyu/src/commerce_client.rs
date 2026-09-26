//! Buyer-only cart and test-order REST client with an explicitly local offline demo.
use serde_json::{json, Value};
use std::time::Duration;

use crate::{
    data::{item, CATALOG},
    profile_client,
};

#[derive(Clone, Default)]
pub struct Friend {
    pub id: i64,
    pub display_name: String,
}

#[derive(Clone, Default)]
pub struct CartItem {
    pub id: i64,
    pub product_id: u16,
    pub recipient_id: i64,
    pub name: String,
    pub price_cents: i64,
}

#[derive(Clone, Default)]
pub struct Order {
    pub id: i64,
    pub total_cents: i64,
    pub status: String,
    pub offline: bool,
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

#[derive(Clone, Default)]
pub struct Commerce {
    pub online: bool,
    pub friends: Vec<Friend>,
    pub items: Vec<CartItem>,
    pub order: Option<Order>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(600))
        .build()
}

fn json_get(path: &str) -> Result<Option<Value>, &'static str> {
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
        Err(ureq::Error::Status(_, _)) => Err("服务器暂时无法读取购物车"),
    }
}

fn json_post(path: &str, body: Value, key: Option<&str>) -> Result<Option<Value>, &'static str> {
    let Some((url, token)) = profile_client::session() else {
        return Ok(None);
    };
    let mut request = agent()
        .post(&format!("{url}/api/v1/{path}"))
        .set("Authorization", &format!("Bearer {token}"));
    if let Some(key) = key {
        request = request.set("Idempotency-Key", key);
    }
    match request.send_json(body) {
        Ok(reply) => reply.into_json().map(Some).map_err(|_| "服务器响应无效"),
        Err(ureq::Error::Transport(_)) => Ok(None),
        Err(ureq::Error::Status(403, _)) => Err("只能送给已确认的好友"),
        Err(ureq::Error::Status(404, _)) => Err("收礼人或订单不存在"),
        Err(ureq::Error::Status(409, _)) => Err("订单状态已改变，请刷新"),
        Err(ureq::Error::Status(_, _)) => Err("服务器未接受操作"),
    }
}

fn json_delete(path: &str) -> Result<Option<Value>, &'static str> {
    let Some((url, token)) = profile_client::session() else {
        return Ok(None);
    };
    match agent()
        .delete(&format!("{url}/api/v1/{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
    {
        Ok(reply) => reply.into_json().map(Some).map_err(|_| "服务器响应无效"),
        Err(ureq::Error::Transport(_)) => Ok(None),
        Err(ureq::Error::Status(_, _)) => Err("服务器未删除购物车商品"),
    }
}

fn parse_items(value: &Value) -> Vec<CartItem> {
    value
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|v| {
            Some(CartItem {
                id: v.get("id")?.as_i64()?,
                product_id: u16::try_from(v.get("product_id")?.as_u64()?).ok()?,
                recipient_id: v.get("recipient_id")?.as_i64()?,
                name: v.get("name")?.as_str()?.into(),
                price_cents: v.get("price_cents")?.as_i64()?,
            })
        })
        .collect()
}

impl Commerce {
    pub fn refresh(&mut self) -> Result<(), &'static str> {
        let friends = json_get("friends")?;
        let cart = json_get("cart")?;
        if let (Some(friends), Some(cart)) = (friends, cart) {
            self.friends = friends
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| {
                    Some(Friend {
                        id: f.get("id")?.as_i64()?,
                        display_name: f.get("display_name")?.as_str()?.into(),
                    })
                })
                .collect();
            let local = self
                .items
                .iter()
                .filter(|x| x.id < 0)
                .cloned()
                .collect::<Vec<_>>();
            self.items = parse_items(&cart);
            self.items.extend(local);
            self.online = true;
        } else {
            self.online = false;
            if self.friends.is_empty() {
                self.friends = vec![
                    Friend {
                        id: 2,
                        display_name: "林舟（演示）".into(),
                    },
                    Friend {
                        id: 3,
                        display_name: "陈晓（演示）".into(),
                    },
                ];
            }
        }
        Ok(())
    }

    pub fn add(&mut self, product_id: u16, recipient_id: i64) -> Result<(), &'static str> {
        if product_id as usize >= CATALOG.len() {
            return Err("商品不存在");
        }
        if !self.friends.iter().any(|f| f.id == recipient_id) {
            return Err("请先选一位熟人");
        }
        let response = json_post(
            "cart/items",
            json!({"product_id":product_id,"recipient_id":recipient_id}),
            None,
        )?;
        if let Some(response) = response {
            let id = response
                .get("id")
                .and_then(Value::as_i64)
                .ok_or("服务器响应无效")?;
            self.online = true;
            self.items.push(CartItem {
                id,
                product_id,
                recipient_id,
                name: item(product_id).name.into(),
                price_cents: item(product_id).price,
            });
        } else {
            self.online = false;
            let id = self
                .items
                .iter()
                .map(|x| x.id)
                .filter(|id| *id < 0)
                .min()
                .unwrap_or(0)
                - 1;
            self.items.push(CartItem {
                id,
                product_id,
                recipient_id,
                name: item(product_id).name.into(),
                price_cents: item(product_id).price,
            });
        }
        Ok(())
    }

    pub fn remove(&mut self, id: i64) -> Result<(), &'static str> {
        if id >= 0 && json_delete(&format!("cart/items/{id}"))?.is_none() {
            return Err("服务器暂时连不上，在线商品尚未移除");
        }
        self.items.retain(|x| x.id != id);
        Ok(())
    }

    pub fn total(&self) -> i64 {
        self.items.iter().map(|x| x.price_cents).sum()
    }

    pub fn create_order(&mut self) -> Result<Order, &'static str> {
        if self.items.is_empty() {
            return Err("购物车是空的");
        }
        if self.items.iter().all(|x| x.id < 0) {
            let order = Order {
                id: -1,
                total_cents: self.total(),
                status: "pending_demo".into(),
                offline: true,
            };
            self.order = Some(order.clone());
            return Ok(order);
        }
        if self.items.iter().any(|x| x.id < 0) {
            return Err("离线商品和服务器商品不能合并结算，请恢复连接后刷新");
        }
        let Some(_) = json_post("orders/quote", json!({}), None)? else {
            return Err("服务器暂时连不上，在线购物车尚未下单");
        };
        let key = format!(
            "liyu-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let Some(value) = json_post("orders", json!({}), Some(&key))? else {
            return Err("下单时服务器断开，请刷新购物车后重试");
        };
        let order = Order {
            id: value
                .get("id")
                .and_then(Value::as_i64)
                .ok_or("订单响应无效")?,
            total_cents: value
                .get("total_cents")
                .and_then(Value::as_i64)
                .ok_or("订单响应无效")?,
            status: value
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("pending")
                .into(),
            offline: false,
        };
        self.items.clear();
        self.order = Some(order.clone());
        Ok(order)
    }

    pub fn pay_test(&mut self) -> Result<Order, &'static str> {
        let Some(mut order) = self.order.clone() else {
            return Err("还没有订单");
        };
        if order.offline {
            order.status = "paid_demo".into();
            self.order = Some(order.clone());
            self.items.clear();
            return Ok(order);
        }
        let Some(value) = json_post(&format!("orders/{}/pay-test", order.id), json!({}), None)?
        else {
            return Err("支付时服务器断开，订单仍待支付，请稍后重试");
        };
        order.status = value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("paid_test")
            .into();
        self.order = Some(order.clone());
        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_cart_ids_and_totals_are_distinct() {
        let mut c = Commerce {
            friends: vec![Friend {
                id: 2,
                display_name: "演示熟人".into(),
            }],
            ..Default::default()
        };
        c.add(0, 2).unwrap();
        c.add(1, 2).unwrap();
        assert_ne!(c.items[0].id, c.items[1].id);
        assert_eq!(c.total(), item(0).price + item(1).price);
        assert!(c.create_order().unwrap().offline);
        assert_eq!(c.pay_test().unwrap().status, "paid_demo");
    }
}
