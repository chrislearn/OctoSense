//! Small account-profile client. A local copy keeps the demo usable offline.
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

#[derive(Default)]
struct AuthState {
    identifier: String,
    token: Option<String>,
    selected: bool,
}
static AUTH: OnceLock<Mutex<AuthState>> = OnceLock::new();

fn auth() -> &'static Mutex<AuthState> {
    AUTH.get_or_init(|| Mutex::new(AuthState::default()))
}

pub fn active_identifier() -> String {
    let state = auth().lock().unwrap();
    if state.selected {
        state.identifier.clone()
    } else {
        std::env::var("LIYU_IDENTIFIER").unwrap_or_else(|_| "demo@liyu.test".into())
    }
}

#[derive(Clone, Default)]
pub struct Profile {
    pub display_name: String,
    pub phone: String,
    pub email: String,
    pub avatar_url: String,
    pub addresses: Vec<Address>,
    pub online: bool,
}

#[derive(Clone, Default)]
pub struct Address {
    pub id: i64,
    pub recipient_name: String,
    pub phone: String,
    pub address: String,
    pub is_default: bool,
}

fn local_path() -> Option<std::path::PathBuf> {
    let mut hash = 0xcbf29ce484222325_u64;
    for b in active_identifier().bytes() {
        hash = (hash ^ u64::from(b)).wrapping_mul(0x100000001b3);
    }
    Some(
        std::path::PathBuf::from(std::env::var_os("MAKEPAD_HOME")?)
            .join(format!("liyu/profile-{hash:016x}.json")),
    )
}

fn local_read() -> Option<Profile> {
    let value: Value = serde_json::from_slice(&std::fs::read(local_path()?).ok()?).ok()?;
    Some(from_json(&value, false))
}

fn local_write(profile: &Profile) {
    let Some(path) = local_path() else { return };
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let value = json!({
        "display_name":profile.display_name,"phone":profile.phone,"email":profile.email,
        "avatar_url":profile.avatar_url,"addresses":profile.addresses.iter().map(|a| json!({
            "id":a.id,"recipient_name":a.recipient_name,"phone":a.phone,
            "address":a.address,"is_default":a.is_default
        })).collect::<Vec<_>>()
    });
    let _ = std::fs::write(path, value.to_string());
}

fn next_local_address_id(profile: &Profile) -> i64 {
    profile
        .addresses
        .iter()
        .map(|a| a.id)
        .filter(|id| *id < 0)
        .min()
        .unwrap_or(0)
        - 1
}

fn from_json(value: &Value, online: bool) -> Profile {
    let field = |key| {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Profile {
        display_name: field("display_name"),
        phone: field("phone"),
        email: field("email"),
        avatar_url: field("avatar_url"),
        addresses: value
            .get("addresses")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|v| Address {
                id: v.get("id").and_then(Value::as_i64).unwrap_or_default(),
                recipient_name: v
                    .get("recipient_name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
                phone: v
                    .get("phone")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
                address: v
                    .get("address")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
                is_default: v
                    .get("is_default")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
            .collect(),
        online,
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(600))
        .build()
}

fn api() -> Option<(String, String)> {
    let url = std::env::var("LIYU_API_URL")
        .ok()?
        .trim_end_matches('/')
        .to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    let (identifier, selected, cached) = {
        let state = auth().lock().ok()?;
        let identifier = if state.selected {
            state.identifier.clone()
        } else {
            std::env::var("LIYU_IDENTIFIER").unwrap_or_else(|_| "demo@liyu.test".into())
        };
        (identifier, state.selected, state.token.clone())
    };
    if let Some(token) = cached {
        return Some((url, token));
    }
    if !selected {
        if let Ok(token) = std::env::var("LIYU_AUTH_TOKEN") {
            if !token.is_empty() {
                return Some((url, token));
            }
        }
    }
    let response: Value = agent()
        .post(&format!("{url}/api/v1/auth/login"))
        .send_json(json!({"identifier":identifier,"password":"123456"}))
        .ok()?
        .into_json()
        .ok()?;
    let token = response.get("token")?.as_str()?.to_string();
    if let Ok(mut state) = auth().lock() {
        state.token = Some(token.clone());
    }
    Some((url, token))
}

pub(crate) fn session() -> Option<(String, String)> { api() }

pub fn sign_in(
    identifier: &str,
    password: &str,
    code: &str,
    register: bool,
) -> Result<(), &'static str> {
    let identifier = identifier.trim().to_lowercase();
    if identifier.is_empty() || identifier.len() > 254 {
        return Err("请输入账号标识");
    }
    if password != "123456" {
        return Err("测试密码是 123456");
    }
    if register && code != "123456" {
        return Err("测试验证码是 123456");
    }
    let Some(url) = std::env::var("LIYU_API_URL")
        .ok()
        .map(|s| s.trim_end_matches('/').to_string())
    else {
        return Err("离线时不能注册或登录，请连接服务器");
    };
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("服务器地址无效");
    }
    let path = if register { "register" } else { "login" };
    let body =
        json!({"identifier":identifier,"password":password,"code":code,"display_name":identifier});
    let response = match agent()
        .post(&format!("{url}/api/v1/auth/{path}"))
        .send_json(body)
    {
        Ok(reply) => reply.into_json::<Value>().map_err(|_| "服务器响应无效")?,
        Err(ureq::Error::Transport(_)) => return Err("服务器暂时连不上，仍可使用离线演示"),
        Err(ureq::Error::Status(409, _)) => return Err("账号已存在，请直接登录"),
        Err(ureq::Error::Status(_, _)) => return Err("账号或密码不正确"),
    };
    let token = response
        .get("token")
        .and_then(Value::as_str)
        .ok_or("服务器响应无效")?;
    let mut state = auth().lock().map_err(|_| "登录状态暂不可用")?;
    state.identifier = identifier;
    state.token = Some(token.into());
    state.selected = true;
    Ok(())
}

pub fn use_demo() {
    if let Ok(mut state) = auth().lock() {
        state.identifier = "demo@liyu.test".into();
        state.token = None;
        state.selected = true;
    }
}

fn get(path: &str) -> Option<Value> {
    let (url, token) = api()?;
    agent()
        .get(&format!("{url}/api/v1/{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .ok()?
        .into_json()
        .ok()
}

fn put(path: &str, value: Value, patch: bool) -> Result<Option<Value>, &'static str> {
    let Some((url, token)) = api() else {
        return Ok(None);
    };
    let request = if patch {
        agent().patch(&format!("{url}/api/v1/{path}"))
    } else {
        agent().put(&format!("{url}/api/v1/{path}"))
    };
    match request
        .set("Authorization", &format!("Bearer {token}"))
        .send_json(value)
    {
        Ok(reply) => Ok(reply.into_json().ok()),
        Err(ureq::Error::Transport(_)) => Ok(None),
        Err(ureq::Error::Status(409, _)) => Err("该手机号或邮箱已被使用"),
        Err(ureq::Error::Status(_, _)) => Err("服务器未接受修改，请检查输入"),
    }
}

pub fn load(default_name: &str) -> Profile {
    let mut local = local_read().unwrap_or_default();
    if local.display_name.is_empty() {
        local.display_name = default_name.into()
    }
    if let Some(remote) = get("me/profile") {
        let mut profile = from_json(&remote, true);
        profile.addresses = get("me/addresses")
            .and_then(|v| v.as_array().cloned())
            .map(|rows| from_json(&json!({"addresses":rows}), true).addresses)
            .unwrap_or_else(|| local.addresses.clone());
        profile
            .addresses
            .extend(local.addresses.into_iter().filter(|a| a.id < 0));
        local_write(&profile);
        profile
    } else {
        local
    }
}

pub fn save_profile(
    profile: &mut Profile,
    name: &str,
    avatar_url: &str,
) -> Result<(), &'static str> {
    let name = name.trim();
    let avatar_url = avatar_url.trim();
    if name.is_empty() || name.chars().count() > 50 {
        return Err("名字须为 1–50 字");
    }
    if !avatar_url.is_empty() && (!avatar_url.starts_with("https://") || avatar_url.len() > 512) {
        return Err("头像需填写 HTTPS 图片地址");
    }
    let reply = put(
        "me/profile",
        json!({"display_name":name,"avatar_url":avatar_url}),
        true,
    )?;
    profile.display_name = name.into();
    profile.avatar_url = avatar_url.into();
    profile.online = reply.is_some();
    local_write(profile);
    Ok(())
}

pub fn bind_contact(
    profile: &mut Profile,
    phone: bool,
    value: &str,
    code: &str,
) -> Result<(), &'static str> {
    let value = value.trim();
    if code != "123456" {
        return Err("测试验证码是 123456");
    }
    if phone && !(value.len() == 11 && value.bytes().all(|b| b.is_ascii_digit())) {
        return Err("手机号须为 11 位数字");
    }
    if !phone && !(value.contains('@') && value.len() <= 254) {
        return Err("邮箱格式不正确");
    }
    let path = if phone { "me/phone" } else { "me/email" };
    let reply = put(path, json!({"value":value,"code":code}), false)?;
    if phone {
        profile.phone = value.into()
    } else {
        profile.email = value.into()
    }
    profile.online = reply.is_some();
    local_write(profile);
    Ok(())
}

pub fn add_address(
    profile: &mut Profile,
    name: &str,
    phone: &str,
    address: &str,
) -> Result<(), &'static str> {
    let (name, phone, address) = (name.trim(), phone.trim(), address.trim());
    if name.is_empty()
        || name.chars().count() > 50
        || !address.is_empty() && address.chars().count() > 500
    {
        return Err("收件人或地址过长");
    }
    if address.is_empty() {
        return Err("请填写收件地址");
    }
    if !(phone.len() == 11 && phone.bytes().all(|b| b.is_ascii_digit())) {
        return Err("手机号须为 11 位数字");
    }
    let body = json!({"recipient_name":name,"phone":phone,"address":address,"is_default":profile.addresses.is_empty()});
    let next_local_id = next_local_address_id(profile);
    let new_id = if let Some((url, token)) = api() {
        match agent()
            .post(&format!("{url}/api/v1/me/addresses"))
            .set("Authorization", &format!("Bearer {token}"))
            .send_json(body)
        {
            Ok(reply) => reply
                .into_json::<Value>()
                .ok()
                .and_then(|v| v.get("id")?.as_i64())
                .unwrap_or(next_local_id),
            Err(ureq::Error::Transport(_)) => next_local_id,
            Err(ureq::Error::Status(_, _)) => return Err("服务器未接受地址"),
        }
    } else {
        next_local_id
    };
    profile.addresses.push(Address {
        id: new_id,
        recipient_name: name.into(),
        phone: phone.into(),
        address: address.into(),
        is_default: profile.addresses.is_empty(),
    });
    profile.online = new_id >= 0;
    local_write(profile);
    Ok(())
}

pub fn update_address(
    profile: &mut Profile,
    id: i64,
    name: &str,
    phone: &str,
    address: &str,
) -> Result<(), &'static str> {
    let (name, phone, address) = (name.trim(), phone.trim(), address.trim());
    if name.is_empty()
        || name.chars().count() > 50
        || address.is_empty()
        || address.chars().count() > 500
    {
        return Err("请填写有效的收件人和地址");
    }
    if !(phone.len() == 11 && phone.bytes().all(|b| b.is_ascii_digit())) {
        return Err("手机号须为 11 位数字");
    }
    let Some(old) = profile.addresses.iter().find(|a| a.id == id) else {
        return Err("地址不存在");
    };
    let is_default = old.is_default;
    if id >= 0 {
        if let Some((url, token)) = api() {
            match agent().put(&format!("{url}/api/v1/me/addresses/{id}"))
                .set("Authorization", &format!("Bearer {token}"))
                .send_json(json!({"recipient_name":name,"phone":phone,"address":address,"is_default":is_default})) {
                Ok(_) | Err(ureq::Error::Transport(_)) => {},
                Err(ureq::Error::Status(_, _)) => return Err("服务器未接受地址修改"),
            }
        }
    }
    if let Some(row) = profile.addresses.iter_mut().find(|a| a.id == id) {
        row.recipient_name = name.into();
        row.phone = phone.into();
        row.address = address.into();
    }
    local_write(profile);
    Ok(())
}

pub fn delete_address(profile: &mut Profile, id: i64) -> Result<(), &'static str> {
    if id >= 0 {
        if let Some((url, token)) = api() {
            match agent()
                .delete(&format!("{url}/api/v1/me/addresses/{id}"))
                .set("Authorization", &format!("Bearer {token}"))
                .call()
            {
                Ok(_) | Err(ureq::Error::Transport(_)) => {}
                Err(ureq::Error::Status(_, _)) => return Err("服务器未删除地址"),
            }
        }
    }
    profile.addresses.retain(|a| a.id != id);
    local_write(profile);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_address_ids_remain_unique_across_additions() {
        let mut profile = Profile::default();
        assert_eq!(next_local_address_id(&profile), -1);
        profile.addresses.push(Address {
            id: -1,
            ..Default::default()
        });
        assert_eq!(next_local_address_id(&profile), -2);
        profile.addresses.push(Address {
            id: -2,
            ..Default::default()
        });
        assert_eq!(next_local_address_id(&profile), -3);
    }
}
