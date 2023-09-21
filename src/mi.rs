use std::error::Error;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Url;
use serde_json::{Value};

lazy_static! {
        static ref EMAIL_REG : Regex = Regex::new(r"^\w+([-+.]\w+)*@\w+([-.]\w+)*\.\w+([-.]\w+)*$").unwrap();
    }

pub async fn update_steps(account: &str, password: &str, steps: u32) -> Result<(), Box<dyn Error>> {
    let access = login(account, password).await?;
    let (login_token, user_id) = get_token(account, access.as_str()).await?;
    let app_token = get_app_token(login_token.as_str()).await?;
    sync_steps(app_token.as_str(), user_id.as_str(), steps).await?;
    Ok(())
}

async fn login(account: &str, password: &str) -> Result<String, Box<dyn Error>> {
    let params = [
        ("client_id", "HuaMi"),
        ("redirect_uri", "https://s3-us-west-2.amazonaws.com/hm-registration/successsignin.html"),
        ("password", password),
        ("token", "access"),
    ];
    let path_val = if is_email(account) {
        account.to_string()
    } else {
        format!("+86{account}")
    };

    let resp = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?.post(format!("https://api-user.huami.com/registrations/{path_val}/tokens"))
        .form(&params)
        .headers(get_headers())
        .send()
        .await?;
    let location = resp.headers().get("location").unwrap();
    get_code(String::from_utf8_lossy(location.as_bytes()).trim())
}


async fn get_token<'a>(account: &'a str, access: &'a str) -> Result<(String, String), Box<dyn Error>> {
    let params = if is_email(account) {
        [
            ("app_name", "com.xiaomi.hm.health"),
            ("app_version", "4.6.0"),
            ("code", access),
            ("country_code", "CN"),
            ("device_id", "2C8B4939-0CCD-4E94-8CBA-CB8EA6E613A1"),
            ("device_model", "phone"),
            ("grant_type", "access_token"),
            ("third_name", "huami"),
            ("allow_registration", "false")
        ]
    } else {
        [
            ("app_name", "com.xiaomi.hm.health"),
            ("app_version", "4.6.0"),
            ("code", access),
            ("country_code", "CN"),
            ("device_id", "2C8B4939-0CCD-4E94-8CBA-CB8EA6E613A1"),
            ("device_model", "phone"),
            ("grant_type", "access_token"),
            ("third_name", "huami_phone"),
            ("allow_registration", "false")
        ]
    };

    let resp = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?.post(format!("https://account.huami.com/v2/client/login"))
        .form(&params)
        .headers(get_headers())
        .send()
        .await?;
    let mut json_value = resp.json::<Value>().await?;
    let error_code = json_value["error_code"].as_str();
    if let Some(code) = error_code {
        return Err(format!("获取login_token错误：{code}").into());
    }
    let token_info = &mut json_value["token_info"];
    let login_token = token_info["login_token"].take();
    let user_id = token_info["user_id"].take();
    let x = login_token.as_str().unwrap().to_string();
    let y = user_id.as_str().unwrap().to_string();
    Ok((x, y))
}

async fn get_app_token(login_token: &str) -> Result<String, Box<dyn Error>> {
    let url = format!("https://account-cn.huami.com/v1/client/app_tokens?app_name=com.xiaomi.hm.health&dn=api-user.huami.com%2Capi-mifit.huami.com%2Capp-analytics.huami.com&login_token={login_token}");
    let resp = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?.get(url)
        .headers(get_headers())
        .send()
        .await?;
    let json_value = resp.json::<Value>().await?;
    let error_code = json_value["error_code"].as_str();
    if let Some(code) = error_code {
        return Err(format!("获取app_token错误：{code}").into());
    }
    Ok(json_value["token_info"]["app_token"].as_str().unwrap().to_string())
}

async fn sync_steps(app_token: &str, user_id: &str, steps: u32) -> Result<(), Box<dyn Error>> {
    let time = chrono::Utc::now().timestamp_millis();
    let url = format!("https://api-mifit-cn.huami.com/v1/data/band_data.json?&t={time}");
    let step_data = get_step_data(steps);
    let params = [
        ("data_json", step_data.as_str()),
        ("userid", user_id),
        ("device_type", "0"),
        ("last_sync_data_time", "1597306380"),
        ("last_deviceid", "DA932FFFFE8816E7")
    ];

    let mut headers = get_headers();
    headers.insert("apptoken", HeaderValue::from_str(app_token).unwrap());
    let resp = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .post(url)
        .headers(headers)
        .form(&params)
        .send()
        .await?;

    let json_value = resp.json::<Value>().await?;
    let code = json_value["code"].as_u64();
    return match code {
        None => {
            Err(format!("更新步数错误").into())
        }
        Some(code) => {
            if code == 1 {
                Ok(())
            } else {
                Err(format!("更新步数错误").into())
            }
        }
    };
}

fn get_code(location: &str) -> Result<String, Box<dyn Error>> {
    let url = Url::parse(location).unwrap();
    let mut parse = url.query_pairs();
    let option = parse.find(|(key, _)| {
        key.eq("access")
    });
    if let Some((_, val)) = option {
        Ok(val.to_string())
    } else {
        Err("登陆失败".into())
    }
}


fn is_email(str: &str) -> bool {
    EMAIL_REG.is_match(str)
}

fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_bytes(b"Mozilla/5.0 (iPhone; CPU iPhone OS 14_7_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.1.2").unwrap());
    headers.insert("Content-Type", HeaderValue::from_bytes(b"application/x-www-form-urlencoded;charset=UTF-8").unwrap());
    headers
}

fn get_step_data(steps: u32) -> String {
    let s = "[{\"data_hr\":\"\\/\\/\\/\\/\\/\\/9L\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/Vv\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/0v\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/9e\\/\\/\\/\\/\\/0n\\/a\\/\\/\\/S\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/0b\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/1FK\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/R\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/9PTFFpaf9L\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/R\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/0j\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/9K\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/Ov\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/zf\\/\\/\\/86\\/zr\\/Ov88\\/zf\\/Pf\\/\\/\\/0v\\/S\\/8\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/Sf\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/z3\\/\\/\\/\\/\\/\\/0r\\/Ov\\/\\/\\/\\/\\/\\/S\\/9L\\/zb\\/Sf9K\\/0v\\/Rf9H\\/zj\\/Sf9K\\/0\\/\\/N\\/\\/\\/\\/0D\\/Sf83\\/zr\\/Pf9M\\/0v\\/Ov9e\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/S\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/zv\\/\\/z7\\/O\\/83\\/zv\\/N\\/83\\/zr\\/N\\/86\\/z\\/\\/Nv83\\/zn\\/Xv84\\/zr\\/PP84\\/zj\\/N\\/9e\\/zr\\/N\\/89\\/03\\/P\\/89\\/z3\\/Q\\/9N\\/0v\\/Tv9C\\/0H\\/Of9D\\/zz\\/Of88\\/z\\/\\/PP9A\\/zr\\/N\\/86\\/zz\\/Nv87\\/0D\\/Ov84\\/0v\\/O\\/84\\/zf\\/MP83\\/zH\\/Nv83\\/zf\\/N\\/84\\/zf\\/Of82\\/zf\\/OP83\\/zb\\/Mv81\\/zX\\/R\\/9L\\/0v\\/O\\/9I\\/0T\\/S\\/9A\\/zn\\/Pf89\\/zn\\/Nf9K\\/07\\/N\\/83\\/zn\\/Nv83\\/zv\\/O\\/9A\\/0H\\/Of8\\/\\/zj\\/PP83\\/zj\\/S\\/87\\/zj\\/Nv84\\/zf\\/Of83\\/zf\\/Of83\\/zb\\/Nv9L\\/zj\\/Nv82\\/zb\\/N\\/85\\/zf\\/N\\/9J\\/zf\\/Nv83\\/zj\\/Nv84\\/0r\\/Sv83\\/zf\\/MP\\/\\/\\/zb\\/Mv82\\/zb\\/Of85\\/z7\\/Nv8\\/\\/0r\\/S\\/85\\/0H\\/QP9B\\/0D\\/Nf89\\/zj\\/Ov83\\/zv\\/Nv8\\/\\/0f\\/Sv9O\\/0ZeXv\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/1X\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/9B\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/TP\\/\\/\\/1b\\/\\/\\/\\/\\/\\/0\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/9N\\/\\/\\/\\/\\/\\/\\/\\/\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\\/v7+\",\"date\":\"1-2-3\",\"data\":[{\"start\":0,\"stop\":1439,\"value\":\"UA8AUBQAUAwAUBoAUAEAYCcAUBkAUB4AUBgAUCAAUAEAUBkAUAwAYAsAYB8AYB0AYBgAYCoAYBgAYB4AUCcAUBsAUB8AUBwAUBIAYBkAYB8AUBoAUBMAUCEAUCIAYBYAUBwAUCAAUBgAUCAAUBcAYBsAYCUAATIPYD0KECQAYDMAYB0AYAsAYCAAYDwAYCIAYB0AYBcAYCQAYB0AYBAAYCMAYAoAYCIAYCEAYCYAYBsAYBUAYAYAYCIAYCMAUB0AUCAAUBYAUCoAUBEAUC8AUB0AUBYAUDMAUDoAUBkAUC0AUBQAUBwAUA0AUBsAUAoAUCEAUBYAUAwAUB4AUAwAUCcAUCYAUCwKYDUAAUUlEC8IYEMAYEgAYDoAYBAAUAMAUBkAWgAAWgAAWgAAWgAAWgAAUAgAWgAAUBAAUAQAUA4AUA8AUAkAUAIAUAYAUAcAUAIAWgAAUAQAUAkAUAEAUBkAUCUAWgAAUAYAUBEAWgAAUBYAWgAAUAYAWgAAWgAAWgAAWgAAUBcAUAcAWgAAUBUAUAoAUAIAWgAAUAQAUAYAUCgAWgAAUAgAWgAAWgAAUAwAWwAAXCMAUBQAWwAAUAIAWgAAWgAAWgAAWgAAWgAAWgAAWgAAWgAAWREAWQIAUAMAWSEAUDoAUDIAUB8AUCEAUC4AXB4AUA4AWgAAUBIAUA8AUBAAUCUAUCIAUAMAUAEAUAsAUAMAUCwAUBYAWgAAWgAAWgAAWgAAWgAAWgAAUAYAWgAAWgAAWgAAUAYAWwAAWgAAUAYAXAQAUAMAUBsAUBcAUCAAWwAAWgAAWgAAWgAAWgAAUBgAUB4AWgAAUAcAUAwAWQIAWQkAUAEAUAIAWgAAUAoAWgAAUAYAUB0AWgAAWgAAUAkAWgAAWSwAUBIAWgAAUC4AWSYAWgAAUAYAUAoAUAkAUAIAUAcAWgAAUAEAUBEAUBgAUBcAWRYAUA0AWSgAUB4AUDQAUBoAXA4AUA8AUBwAUA8AUA4AUA4AWgAAUAIAUCMAWgAAUCwAUBgAUAYAUAAAUAAAUAAAUAAAUAAAUAAAUAAAUAAAUAAAWwAAUAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAeSEAeQ8AcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcBcAcAAAcAAAcCYOcBUAUAAAUAAAUAAAUAAAUAUAUAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcCgAeQAAcAAAcAAAcAAAcAAAcAAAcAYAcAAAcBgAeQAAcAAAcAAAegAAegAAcAAAcAcAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcCkAeQAAcAcAcAAAcAAAcAwAcAAAcAAAcAIAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcCIAeQAAcAAAcAAAcAAAcAAAcAAAeRwAeQAAWgAAUAAAUAAAUAAAUAAAUAAAcAAAcAAAcBoAeScAeQAAegAAcBkAeQAAUAAAUAAAUAAAUAAAUAAAUAAAcAAAcAAAcAAAcAAAcAAAcAAAegAAegAAcAAAcAAAcBgAeQAAcAAAcAAAcAAAcAAAcAAAcAkAegAAegAAcAcAcAAAcAcAcAAAcAAAcAAAcAAAcA8AeQAAcAAAcAAAeRQAcAwAUAAAUAAAUAAAUAAAUAAAUAAAcAAAcBEAcA0AcAAAWQsAUAAAUAAAUAAAUAAAUAAAcAAAcAoAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAYAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcBYAegAAcAAAcAAAegAAcAcAcAAAcAAAcAAAcAAAcAAAeRkAegAAegAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAEAcAAAcAAAcAAAcAUAcAQAcAAAcBIAeQAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcBsAcAAAcAAAcBcAeQAAUAAAUAAAUAAAUAAAUAAAUBQAcBYAUAAAUAAAUAoAWRYAWTQAWQAAUAAAUAAAUAAAcAAAcAAAcAAAcAAAcAAAcAMAcAAAcAQAcAAAcAAAcAAAcDMAeSIAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcAAAcBQAeQwAcAAAcAAAcAAAcAMAcAAAeSoAcA8AcDMAcAYAeQoAcAwAcFQAcEMAeVIAaTYAbBcNYAsAYBIAYAIAYAIAYBUAYCwAYBMAYDYAYCkAYDcAUCoAUCcAUAUAUBAAWgAAYBoAYBcAYCgAUAMAUAYAUBYAUA4AUBgAUAgAUAgAUAsAUAsAUA4AUAMAUAYAUAQAUBIAASsSUDAAUDAAUBAAYAYAUBAAUAUAUCAAUBoAUCAAUBAAUAoAYAIAUAQAUAgAUCcAUAsAUCIAUCUAUAoAUA4AUB8AUBkAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAAfgAA\",\"tz\":32,\"did\":\"DA932FFFFE8816E7\",\"src\":24}],\"summary\":\"{\\\"v\\\":6,\\\"slp\\\":{\\\"st\\\":1628296479,\\\"ed\\\":1628296479,\\\"dp\\\":0,\\\"lt\\\":0,\\\"wk\\\":0,\\\"usrSt\\\":-1440,\\\"usrEd\\\":-1440,\\\"wc\\\":0,\\\"is\\\":0,\\\"lb\\\":0,\\\"to\\\":0,\\\"dt\\\":0,\\\"rhr\\\":0,\\\"ss\\\":0},\\\"stp\\\":{\\\"ttl\\\":989898,\\\"dis\\\":10627,\\\"cal\\\":510,\\\"wk\\\":41,\\\"rn\\\":50,\\\"runDist\\\":7654,\\\"runCal\\\":397,\\\"stage\\\":[{\\\"start\\\":327,\\\"stop\\\":341,\\\"mode\\\":1,\\\"dis\\\":481,\\\"cal\\\":13,\\\"step\\\":680},{\\\"start\\\":342,\\\"stop\\\":367,\\\"mode\\\":3,\\\"dis\\\":2295,\\\"cal\\\":95,\\\"step\\\":2874},{\\\"start\\\":368,\\\"stop\\\":377,\\\"mode\\\":4,\\\"dis\\\":1592,\\\"cal\\\":88,\\\"step\\\":1664},{\\\"start\\\":378,\\\"stop\\\":386,\\\"mode\\\":3,\\\"dis\\\":1072,\\\"cal\\\":51,\\\"step\\\":1245},{\\\"start\\\":387,\\\"stop\\\":393,\\\"mode\\\":4,\\\"dis\\\":1036,\\\"cal\\\":57,\\\"step\\\":1124},{\\\"start\\\":394,\\\"stop\\\":398,\\\"mode\\\":3,\\\"dis\\\":488,\\\"cal\\\":19,\\\"step\\\":607},{\\\"start\\\":399,\\\"stop\\\":414,\\\"mode\\\":4,\\\"dis\\\":2220,\\\"cal\\\":120,\\\"step\\\":2371},{\\\"start\\\":415,\\\"stop\\\":427,\\\"mode\\\":3,\\\"dis\\\":1268,\\\"cal\\\":59,\\\"step\\\":1489},{\\\"start\\\":428,\\\"stop\\\":433,\\\"mode\\\":1,\\\"dis\\\":152,\\\"cal\\\":4,\\\"step\\\":238},{\\\"start\\\":434,\\\"stop\\\":444,\\\"mode\\\":3,\\\"dis\\\":2295,\\\"cal\\\":95,\\\"step\\\":2874},{\\\"start\\\":445,\\\"stop\\\":455,\\\"mode\\\":4,\\\"dis\\\":1592,\\\"cal\\\":88,\\\"step\\\":1664},{\\\"start\\\":456,\\\"stop\\\":466,\\\"mode\\\":3,\\\"dis\\\":1072,\\\"cal\\\":51,\\\"step\\\":1245},{\\\"start\\\":467,\\\"stop\\\":477,\\\"mode\\\":4,\\\"dis\\\":1036,\\\"cal\\\":57,\\\"step\\\":1124},{\\\"start\\\":478,\\\"stop\\\":488,\\\"mode\\\":3,\\\"dis\\\":488,\\\"cal\\\":19,\\\"step\\\":607},{\\\"start\\\":489,\\\"stop\\\":499,\\\"mode\\\":4,\\\"dis\\\":2220,\\\"cal\\\":120,\\\"step\\\":2371},{\\\"start\\\":500,\\\"stop\\\":511,\\\"mode\\\":3,\\\"dis\\\":1268,\\\"cal\\\":59,\\\"step\\\":1489},{\\\"start\\\":512,\\\"stop\\\":522,\\\"mode\\\":1,\\\"dis\\\":152,\\\"cal\\\":4,\\\"step\\\":238}]},\\\"goal\\\":8000,\\\"tz\\\":\\\"28800\\\"}\",\"source\":24,\"type\":0}]";
    s.replace("1-2-3", chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string().as_str())
        .replace("989898", steps.to_string().as_str())
}
