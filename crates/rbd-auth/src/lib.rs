//! # rbd-auth
//!
//! 鉴权层: 多 profile SESSDATA + keyring 系统 keychain + QR 登录 (WEB/TV) + buvid 主动取.
//!
//! **算法来源**:
//! - QR 登录端点: `https://passport.bilibili.com/x/passport-login/web/qrcode/generate`
//!   + `poll` (`BBDownLoginUtil`, Yutto login.py)
//! - 多 profile 设计: Yutto auth.py `AuthFileModel.profiles` dict
//! - 系统 keychain: `keyring` crate v3 (apple-native / windows-native / linux-native)

#![warn(missing_docs)]

/// buvid 主动获取 (从 www.bilibili.com 拿 cookie).
pub mod buvid;
/// Cookie 序列化 (curl 风格 + Set-Cookie 解析).
pub mod cookie;
/// 系统 keychain 持久化 (基于 `keyring` crate).
pub mod keyring_store;
/// 多 profile 鉴权数据模型 + TOML 序列化.
pub mod profile;
/// 登录态刷新 (SESSDATA 过期检测).
pub mod refresh;
/// TV 扫码登录.
pub mod tv_qr;
/// WEB 扫码登录.
pub mod web_qr;

pub use buvid::fetch_buvid;
pub use cookie::{merge_cookies, parse_cookie_string, parse_set_cookie, to_cookie_header};
pub use keyring_store::{
    delete as delete_profile, list as list_profiles, load as load_profile, save as save_profile,
};
pub use profile::AuthProfile;
pub use refresh::is_session_valid;
pub use web_qr::{QrGenerateResponse, QrPollResponse, QrStatus};
