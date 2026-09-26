//! Small account-profile client. A local copy keeps the demo usable offline.
//!
//! 会话(token)持久化在 `$MAKEPAD_HOME/liyu/session.json` —— 与资料 JSON 分开,
//! 密码永远不落盘。401 / token 失效时只标记过期、回到认证界面,绝不静默重登。
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// 账号怎么用上的:真实登录(有 token)、离线演示、还是没选过(首次启动)。
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum AuthMode {
    /// 启动后还没登录也没选演示。
    #[default]
    None,
    /// 服务器登录成功,持有 token。
    Account,
    /// 用户明确点了「离线演示」。
    Demo,
}

#[derive(Default)]
struct AuthState {
    identifier: String,
    token: Option<String>,
    mode: AuthMode,
    /// 会话被服务器拒绝过(401):不再用旧资料装在线,必须重新登录。
    expired: bool,
}
static AUTH: OnceLock<Mutex<AuthState>> = OnceLock::new();

fn auth() -> &'static Mutex<AuthState> {
    AUTH.get_or_init(|| Mutex::new(AuthState::default()))
}

pub fn active_identifier() -> String {
    let state = auth().lock().unwrap();
    if state.mode != AuthMode::None {
        state.identifier.clone()
    } else {
        std::env::var("LIYU_IDENTIFIER").unwrap_or_else(|_| "demo@liyu.test".into())
    }
}

pub fn mode() -> AuthMode {
    auth().lock().map(|s| s.mode).unwrap_or_default()
}

/// 已经有过一次明确的身份选择(登录或离线演示):false = 首次启动该显示认证界面。
pub fn has_choice() -> bool {
    auth().lock().map(|s| s.mode != AuthMode::None).unwrap_or(false)
}

/// 真实账号会话还有效(有 token 且没被 401 否掉)。
pub fn is_online() -> bool {
    auth()
        .lock()
        .map(|s| s.mode == AuthMode::Account && s.token.is_some() && !s.expired)
        .unwrap_or(false)
}

/// token 被服务器拒绝过:界面据此弹回认证,而不是继续显示伪在线资料。
pub fn take_expired() -> bool {
    match auth().lock() {
        Ok(mut s) => {
            let was = s.expired;
            s.expired = false;
            was
        }
        Err(_) => false,
    }
}

/// 测试服务器(契约文档:演示密码与验证码固定 123456)才显示测试提示。
pub fn is_test_server() -> bool {
    std::env::var("LIYU_API_URL")
        .map(|u| u.contains("test") || u.contains("localhost") || u.contains("127.0.0.1"))
        .unwrap_or(false)
}

// ---- 会话持久化:独立的 session.json,不混进资料 JSON ----

fn session_path() -> Option<std::path::PathBuf> {
    Some(std::path::PathBuf::from(std::env::var_os("MAKEPAD_HOME")?).join("liyu/session.json"))
}

fn session_save() {
    let Ok(state) = auth().lock() else { return };
    let Some(path) = session_path() else { return };
    if state.mode == AuthMode::Account {
        if let Some(token) = &state.token {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let value = json!({"identifier": state.identifier, "token": token});
            let _ = std::fs::write(path, value.to_string());
            return;
        }
    }
    // 演示 / 登出:本机不留凭据。
    let _ = std::fs::remove_file(path);
}

/// 启动时恢复上次登录的会话。有凭据就回到 Account,没有就是 None(首次启动)。
pub fn restore_session() {
    let Some(path) = session_path() else { return };
    let Ok(bytes) = std::fs::read(&path) else { return };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else { return };
    let Some(identifier) = value.get("identifier").and_then(Value::as_str) else {
        return;
    };
    let Some(token) = value.get("token").and_then(Value::as_str) else {
        return;
    };
    if identifier.is_empty() || token.is_empty() {
        return;
    }
    if let Ok(mut state) = auth().lock() {
        state.identifier = identifier.into();
        state.token = Some(token.into());
        state.mode = AuthMode::Account;
        state.expired = false;
    }
}

/// 登出:清内存会话 + 删本机凭据文件。调用方负责重置按账号隔离的本地缓存。
pub fn logout() {
    if let Ok(mut state) = auth().lock() {
        state.identifier.clear();
        state.token = None;
        state.mode = AuthMode::None;
        state.expired = false;
    }
    let Some(path) = session_path() else { return };
    let _ = std::fs::remove_file(path);
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

/// 按当前账号标识派生的本机资料文件:切账号自然换一份,不串号。
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

/// 登出 / 换号时清掉指定账号的本机资料缓存(礼物等业务数据在 data.rs 侧同样按账号隔离)。
pub fn clear_local_cache() {
    let Some(path) = local_path() else { return };
    let _ = std::fs::remove_file(path);
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

/// 取一个可用会话。不会再去用固定密码重登:没有 token 就没有会话,
/// 让界面回到认证入口,而不是装作还在线。
fn api() -> Option<(String, String)> {
    let url = std::env::var("LIYU_API_URL")
        .ok()?
        .trim_end_matches('/')
        .to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    let state = auth().lock().ok()?;
    if state.mode != AuthMode::Account || state.expired {
        return None;
    }
    let token = state.token.clone()?;
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
    if password.is_empty() || password.len() > 128 {
        return Err("请输入密码");
    }
    // 固定验证码说明只在测试服务器上提示;真实服务器按服务端校验来。
    if is_test_server() {
        if password != "123456" {
            return Err("测试密码是 123456");
        }
        if register && code != "123456" {
            return Err("测试验证码是 123456");
        }
    } else if register && code.is_empty() {
        return Err("请输入验证码");
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
    {
        let mut state = auth().lock().map_err(|_| "登录状态暂不可用")?;
        state.identifier = identifier;
        state.token = Some(token.into());
        state.mode = AuthMode::Account;
        state.expired = false;
    }
    // token 落盘,重启可恢复;密码只在这次请求里用过,不保存。
    session_save();
    Ok(())
}

/// 进入离线演示:本机身份,不发请求、不保存凭据。与真实账号严格区隔。
pub fn use_demo() {
    if let Ok(mut state) = auth().lock() {
        state.identifier = "demo@liyu.test".into();
        state.token = None;
        state.mode = AuthMode::Demo;
        state.expired = false;
    }
    // 演示不持有凭据:任何遗留的 session.json 都不该在演示名下生效。
    let Some(path) = session_path() else { return };
    let _ = std::fs::remove_file(path);
}

/// 401 / token 失效:只标记过期并丢弃内存里的 token。绝不在这里重登 ——
/// 重登是用户在认证界面里点出来的,不能藏在一次资料读取背后。
fn mark_expired() {
    if let Ok(mut state) = auth().lock() {
        state.expired = true;
        state.token = None;
    }
    let Some(path) = session_path() else { return };
    let _ = std::fs::remove_file(path);
}

// ---- 头像:二进制上传 / 删除 + 本地缓存与待上传状态 ----

/// 上传成功后服务端返回的标识/URL 存进 profile.avatar_url;
/// 断网待上传时 avatar_url 置 "pending:<内容指纹>" 占位并保留本地字节,明示未同步。
/// 待上传头像字节缓存文件名(同一账号目录下,与 profile-*.json 同目录)。
/// 公开给 UI:重启后读回本地字节做预览,以及重试上传时取字节。
pub fn avatar_pending_path() -> Option<std::path::PathBuf> {
    let path = local_path()?;
    Some(path.with_extension("avatar-pending"))
}

/// 断网删除头像后持久记录的「待删除」意图路径,联网后据此重试同步。
pub fn avatar_delete_pending_path() -> Option<std::path::PathBuf> {
    let path = local_path()?;
    Some(path.with_extension("avatar-delete-pending"))
}

/// 是否存在未同步的删除意图(断网删除后联网需重试)。
pub fn avatar_delete_pending() -> bool {
    avatar_delete_pending_path().is_some_and(|p| p.exists())
}

/// 联网且已登录时,若存在待删除意图则重试服务端删除;成功(或 404)清除意图。
/// 返回 true 表示无遗留待删除(本就没有,或本次同步成功)。
pub fn retry_delete_avatar() -> bool {
    if !avatar_delete_pending() {
        return true;
    }
    let Some((url, token)) = api() else {
        return false;
    };
    let ok = matches!(
        agent()
            .delete(&format!("{url}/api/v1/me/avatar"))
            .set("Authorization", &format!("Bearer {token}"))
            .call(),
        Ok(_) | Err(ureq::Error::Status(404, _))
    );
    if ok {
        if let Some(path) = avatar_delete_pending_path() {
            let _ = std::fs::remove_file(path);
        }
    }
    ok
}

/// 头像字节指纹(FNV-1a,与 local_path 同一套 hash),用于"pending:"占位标识。
fn avatar_fingerprint(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for &b in bytes {
        hash = (hash ^ u64::from(b)).wrapping_mul(0x100000001b3);
    }
    hash
}

/// 解析头像上传响应里的头像标识。服务端契约未给出字段名(沙箱读不到
/// liyu-server 源码),按 RESTful 惯例依次尝试 avatar_url / avatar / url / id。
/// 服务端契约:成功 200 JSON `{"avatar_url":"/api/v1/media/avatars/<32hex>",...}`。
/// 只认 `avatar_url` 且必须是该媒体路径;其它字段名/纯字符串/缺失一律视为无效,
/// 由调用方按失败处理(不清缓存、不报成功)。
fn parse_avatar_reply(value: &Value) -> Option<String> {
    let s = value.get("avatar_url").and_then(Value::as_str)?;
    let s = s.trim();
    // 必须形如 /api/v1/media/avatars/<32hex>。
    let rest = s.strip_prefix("/api/v1/media/avatars/")?;
    if rest.len() == 32 && rest.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(s.to_string())
    } else {
        None
    }
}

/// 上传头像:POST /api/v1/me/avatar,body=图像字节,Content-Type=编码对应类型。
/// 成功返回服务端头像标识并清掉待上传缓存;401 标记过期;断网时把字节留在
/// 本地待上传缓存,profile.avatar_url 置 "pending:<指纹>",返回明确错误。
pub fn upload_avatar(
    profile: &mut Profile,
    bytes: &[u8],
    content_type: &str,
) -> Result<String, &'static str> {
    if bytes.is_empty() {
        return Err("头像内容为空");
    }
    if !matches!(content_type, "image/jpeg" | "image/png" | "image/webp") {
        return Err("头像编码类型不支持");
    }
    let Some((url, token)) = api() else {
        return pending_avatar(profile, bytes);
    };
    match agent()
        .post(&format!("{url}/api/v1/me/avatar"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", content_type)
        .send_bytes(bytes)
    {
        Ok(reply) => {
            // 解析失败/字段不符 = 上传未被服务端确认:不得清缓存、不得报成功。
            let ident = reply
                .into_json::<Value>()
                .ok()
                .and_then(|v| parse_avatar_reply(&v));
            let Some(ident) = ident else {
                return Err("服务器响应格式不符，头像上传未被确认，请重试");
            };
            profile.avatar_url = ident.clone();
            profile.online = true;
            let _ = std::fs::remove_file(avatar_pending_path().unwrap_or_default());
            local_write(profile);
            Ok(ident)
        }
        Err(ureq::Error::Status(401, _)) => {
            mark_expired();
            Err("登录已过期，请重新登录")
        }
        Err(ureq::Error::Transport(_)) => pending_avatar(profile, bytes),
        Err(ureq::Error::Status(413, _)) => Err("头像文件过大，服务器未接受"),
        Err(ureq::Error::Status(415, _)) => Err("头像格式不被服务器接受"),
        Err(ureq::Error::Status(_, _)) => Err("服务器未接受头像，请稍后重试"),
    }
}

/// 断网 / 未登录时:字节留在本地待上传缓存,资料里明示未同步。
fn pending_avatar(profile: &mut Profile, bytes: &[u8]) -> Result<String, &'static str> {
    let fingerprint = avatar_fingerprint(bytes);
    // 只有真的把字节写到本机缓存,才记录待上传;写失败不得声称已保存。
    let Some(path) = avatar_pending_path() else {
        return Err("网络不可用，且本机无法定位待上传缓存目录，头像未保存");
    };
    if let Some(dir) = path.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return Err("网络不可用，且本机缓存目录创建失败，头像未保存");
        }
    }
    if std::fs::write(&path, bytes).is_err() {
        return Err("网络不可用，且头像字节写入本机失败，头像未保存");
    }
    profile.avatar_url = format!("pending:{fingerprint:016x}");
    profile.online = false;
    local_write(profile);
    Err("网络不可用，头像已保存在本机，联网后重试")
}

/// 删除头像:DELETE /api/v1/me/avatar,成功回到默认占位。断网时本地先清,
/// 待上传缓存一并删掉(删除意图优先于未同步的上传)。
pub fn delete_avatar(profile: &mut Profile) -> Result<(), &'static str> {
    let mut synced = false;
    if let Some((url, token)) = api() {
        match agent()
            .delete(&format!("{url}/api/v1/me/avatar"))
            .set("Authorization", &format!("Bearer {token}"))
            .call()
        {
            Ok(_) => synced = true,
            Err(ureq::Error::Status(401, _)) => {
                mark_expired();
                return Err("登录已过期，请重新登录");
            }
            Err(ureq::Error::Transport(_)) => synced = false,
            Err(ureq::Error::Status(404, _)) => synced = true, // 服务端本来就没有头像
            Err(ureq::Error::Status(_, _)) => return Err("服务器未删除头像，请稍后重试"),
        }
    }
    // 删掉本地待上传缓存(删除意图优先于未同步的上传)。
    if let Some(path) = avatar_pending_path() {
        let _ = std::fs::remove_file(path);
    }
    if synced {
        // 服务端已确认删除:清掉任何持久删除意图,回到默认占位。
        profile.avatar_url.clear();
        profile.online = true;
        let _ = std::fs::remove_file(avatar_delete_pending_path().unwrap_or_default());
        local_write(profile);
        return Ok(());
    }
    // 断网/未登录:本地先清,但必须持久记录「待删除」意图供联网后重试——
    // 不能声称会自动同步,也不能让重启/服务端把旧头像恢复。
    profile.avatar_url.clear();
    profile.online = false;
    if let Some(path) = avatar_delete_pending_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::write(&path, b"delete").is_err() {
            local_write(profile);
            return Err("网络不可用，头像已在本机删除，但待删除记录写入失败，联网后请手动重试删除");
        }
    }
    local_write(profile);
    Err("网络不可用，头像已在本机删除，联网后需重试同步（已记录待删除）")
}

/// 是否存在本地待上传的头像(资料里 avatar_url 以 "pending:" 开头)。
pub fn avatar_pending(profile: &Profile) -> bool {
    profile.avatar_url.starts_with("pending:")
}

fn get(path: &str) -> Option<Value> {
    let (url, token) = api()?;
    match agent()
        .get(&format!("{url}/api/v1/{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
    {
        Ok(reply) => reply.into_json().ok(),
        Err(ureq::Error::Status(401, _)) => {
            mark_expired();
            None
        }
        Err(_) => None,
    }
}

/// 已上传头像的图像字节回显:GET 服务端返回的 avatar_url 路径(如
/// /api/v1/media/avatars/<32hex>),取原始字节供界面解码显示。断网/401/无会话返回 None。
pub fn fetch_avatar_bytes(avatar_url: &str) -> Option<Vec<u8>> {
    // 只接受服务端契约的媒体路径,拒绝把 pending:/外部 URL 当可 GET 路径。
    let path = avatar_url.strip_prefix("/api/v1/")?;
    let (url, token) = api()?;
    match agent()
        .get(&format!("{url}/api/v1/{path}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
    {
        Ok(reply) => {
            use std::io::Read;
            let mut buf = Vec::new();
            reply
                .into_reader()
                .read_to_end(&mut buf)
                .ok()
                .filter(|_| !buf.is_empty())
                .map(|_| buf)
        }
        Err(ureq::Error::Status(401, _)) => {
            mark_expired();
            None
        }
        Err(_) => None,
    }
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
        Err(ureq::Error::Status(401, _)) => {
            mark_expired();
            Err("登录已过期，请重新登录")
        }
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
    let _ = avatar_url; // 头像不再经保存资料 PATCH;保留签名避免牵动调用点。
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 50 {
        return Err("名字须为 1–50 字");
    }
    // 头像走二进制上传(upload_avatar)/删除(delete_avatar)的专用契约,保存资料只
    // 更新显示名——不把 avatar_url(尤其 "pending:<指纹>" 占位)写进 PATCH,否则会把
    // 服务端真实头像覆盖成占位串或旧值。
    let reply = put("me/profile", json!({"display_name":name}), true)?;
    profile.display_name = name.into();
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
    fn avatar_content_type_is_validated_before_any_network() {
        // 不支持的编码类型在发请求前就被拒(无服务器也能走到)。
        let mut profile = Profile::default();
        assert_eq!(
            upload_avatar(&mut profile, b"x", "image/gif"),
            Err("头像编码类型不支持")
        );
        assert_eq!(
            upload_avatar(&mut profile, &[], "image/png"),
            Err("头像内容为空")
        );
    }

    #[test]
    fn avatar_fingerprint_is_stable_and_content_sensitive() {
        let a = avatar_fingerprint(b"avatar-bytes");
        assert_eq!(a, avatar_fingerprint(b"avatar-bytes"));
        assert_ne!(a, avatar_fingerprint(b"avatar-bytes!"));
        assert_ne!(a, avatar_fingerprint(b""));
    }

    #[test]
    fn avatar_reply_parsing_strict_contract() {
        // 只认 avatar_url 且必须是 /api/v1/media/avatars/<32hex>;其它一律无效。
        assert_eq!(
            parse_avatar_reply(
                &json!({"avatar_url":"/api/v1/media/avatars/0123456789abcdef0123456789abcdef"})
            ),
            Some("/api/v1/media/avatars/0123456789abcdef0123456789abcdef".into())
        );
        // 非契约路径、其它字段名、纯字符串、缺字段、空串、非 32hex 全部拒绝。
        assert_eq!(parse_avatar_reply(&json!({"avatar_url":"https://cdn/x.png"})), None);
        assert_eq!(parse_avatar_reply(&json!({"avatar":"id-42","url":"ignored"})), None);
        assert_eq!(parse_avatar_reply(&json!("plain-id")), None);
        assert_eq!(parse_avatar_reply(&json!({"other":1})), None);
        assert_eq!(parse_avatar_reply(&json!({"avatar_url":""})), None);
        assert_eq!(
            parse_avatar_reply(&json!({"avatar_url":"/api/v1/media/avatars/xyz"})),
            None
        );
    }

    #[test]
    fn avatar_pending_state_is_marked_and_detected() {
        // 断网(无 LIYU_API_URL / 无 token)时进入待上传:avatar_url 带 pending: 前缀。
        let mut profile = Profile::default();
        let result = upload_avatar(&mut profile, b"img", "image/png");
        if api().is_none() {
            // 无会话时走 pending_avatar:明确错误 + 本地占位(需缓存目录可写)。
            if avatar_pending_path().is_some() {
                assert!(result.is_err());
                assert!(avatar_pending(&profile));
                assert!(!profile.online);
            } else {
                assert!(result.is_err());
            }
        }
        // 删除后回到默认占位,pending 状态随之清除。
        let _ = delete_avatar(&mut profile);
        assert!(!avatar_pending(&profile));
        assert!(profile.avatar_url.is_empty());
        // 断网删除会记录待删除意图(有缓存目录时),联网后可 retry_delete_avatar 同步。
        if api().is_none() && avatar_delete_pending_path().is_some() {
            assert!(avatar_delete_pending());
        }
    }

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
