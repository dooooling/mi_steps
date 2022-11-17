#[path = "mod.rs"]
mod mods;

use std::env;
use chrono::Timelike;
use crate::mods::bark::{Bark, BarkMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let accounts = env::var("ACCOUNTS")?;
    let bark_server = env::var("BARK_SERVER")?;
    let bark_key = env::var("BARK_KEY")?;
    let full_time = env::var("FULL_TIME").unwrap_or("17".to_string()).parse::<u32>()?;
    let max_steps = env::var("MAX_STEPS").unwrap_or("100000".to_string()).parse::<u32>()?;

    let bark = if !(bark_key.is_empty() || bark_server.is_empty()) {
        Some(Bark::new(bark_server, bark_key))
    } else { None };

    for account in accounts.lines() {
        let (account, password) = account.split_once("#").unwrap();
        let steps = gen_steps(full_time, max_steps);
        mods::mi::update_steps(account, password, steps).await?;
        if let Some(bark) = &bark {
            let mut message = BarkMessage::default();
            message.body = format!("🏃‍[{}]更新步数成功->{}。", account, steps);
            message.title = Some("✔小米运动同步🐾".to_string());
            message.group = Some("小米运动同步记录".to_string());
            bark.send_message(message).await?;
        }
    }
    println!("同步成功!!!!!!!!!!!!!!!");
    Ok(())
}


fn gen_steps(full_time: u32, max_steps: u32) -> u32 {
    let hour = chrono::Local::now().hour();
    return if hour < full_time {
        let rate = hour as f64 / full_time as f64;
        let step = rate * max_steps as f64;
        step as u32
    } else { max_steps };
}