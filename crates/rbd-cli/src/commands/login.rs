//! rbd login / logintv / auth / logout — 鉴权命令.
use anyhow::Result;

/// WEB 扫码登录.
pub async fn run_web(profile: Option<String>) -> Result<()> {
    use rbd_auth::keyring_store;
    use rbd_auth::profile::AuthProfile;
    use rbd_auth::web_qr;

    let profile_name = profile.unwrap_or_else(|| "default".to_string());

    tracing::info!("生成 WEB 登录二维码...");
    let qr = web_qr::generate().await?;

    println!("请用 B 站 APP 扫描以下二维码登录:");
    println!("URL: {}", qr.url);

    tracing::info!("等待扫码确认 (180s 超时)...");
    let cookies = web_qr::poll(&qr.qrcode_key).await?;

    let mut auth_profile = AuthProfile::default();
    auth_profile.name = profile_name.clone();
    for (key, value) in &cookies {
        match key.as_str() {
            "SESSDATA" => auth_profile.sessdata = value.clone(),
            "bili_jct" => auth_profile.bili_jct = value.clone(),
            "DedeUserID" => auth_profile.dedeuserid = value.clone(),
            "buvid3" => auth_profile.buvid3 = value.clone(),
            "buvid4" => auth_profile.buvid4 = value.clone(),
            _ => {
                auth_profile.cookies.insert(key.clone(), value.clone());
            }
        }
    }
    keyring_store::save(&auth_profile)?;
    tracing::info!("WEB 登录成功, profile: {}", profile_name);
    println!("登录成功! profile: {profile_name}");
    Ok(())
}

/// TV 扫码登录.
pub async fn run_tv(profile: Option<String>) -> Result<()> {
    use rbd_auth::keyring_store;
    use rbd_auth::profile::AuthProfile;
    use rbd_auth::tv_qr;

    let profile_name = profile.unwrap_or_else(|| "default".to_string());

    tracing::info!("生成 TV 登录二维码...");
    let qr = tv_qr::generate().await?;

    println!("请用 B 站 APP 扫描 TV 二维码:");
    println!("URL: {}", qr.url);
    println!(
        "或者访问 https://account.bilibili.com/h5/device-validate?auth_code={} 输入配对码",
        qr.qrcode_key
    );

    tracing::info!("等待 TV 确认 (180s 超时)...");
    let cookies = tv_qr::poll(&qr.qrcode_key).await?;

    let mut auth_profile = AuthProfile::default();
    auth_profile.name = profile_name.clone();
    for (key, value) in &cookies {
        match key.as_str() {
            "SESSDATA" => auth_profile.sessdata = value.clone(),
            "bili_jct" => auth_profile.bili_jct = value.clone(),
            "DedeUserID" => auth_profile.dedeuserid = value.clone(),
            "buvid3" => auth_profile.buvid3 = value.clone(),
            "buvid4" => auth_profile.buvid4 = value.clone(),
            _ => {
                auth_profile.cookies.insert(key.clone(), value.clone());
            }
        }
    }
    keyring_store::save(&auth_profile)?;
    tracing::info!("TV 登录成功, profile: {}", profile_name);
    println!("TV 登录成功! profile: {profile_name}");
    Ok(())
}

/// 查看鉴权状态.
pub fn status() -> Result<()> {
    use rbd_auth::keyring_store;
    let profiles = keyring_store::list()?;
    if profiles.is_empty() {
        println!("未登录 (无已保存的 profile)");
    } else {
        println!("已配置 profile: {}", profiles.join(", "));
        // 尝试加载每个 profile 检查有效性
        for name in &profiles {
            match keyring_store::load(name) {
                Ok(p) if p.is_logged_in() => {
                    let user = if p.uname.is_empty() {
                        format!("mid={}", p.mid)
                    } else {
                        p.uname.clone()
                    };
                    println!("  {name}: 已登录 ({user})");
                }
                Ok(_) => {
                    println!("  {name}: 未登录");
                }
                Err(e) => {
                    println!("  {name}: 加载失败 ({e})");
                }
            }
        }
    }
    Ok(())
}

/// 登出 (删除本地 cookie).
pub fn logout(profile: Option<String>) -> Result<()> {
    use rbd_auth::keyring_store;
    match profile {
        Some(name) => {
            keyring_store::delete(&name)?;
            println!("已登出 profile: {name}");
        }
        None => {
            let profiles = keyring_store::list()?;
            if profiles.is_empty() {
                println!("没有已保存的 profile, 无需登出");
            } else {
                for p in &profiles {
                    match keyring_store::delete(p) {
                        Ok(()) => println!("已登出: {p}"),
                        Err(e) => tracing::warn!("登出 {p} 失败: {e}"),
                    }
                }
            }
        }
    }
    Ok(())
}
