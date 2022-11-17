use std::error::Error;
use reqwest::Url;
use serde_json::Value;


pub struct Bark {
    server: String,
    key: String,
}

#[allow(dead_code)]
impl Bark {
    pub fn new(server: String, key: String) -> Self {
        Bark {
            server,
            key,
        }
    }
    pub async fn send(&self, msg: &str) -> Result<(), Box<dyn Error>> {
        let mut message = BarkMessage::default();
        message.body = msg.to_string();
        self.send_message(message).await
    }

    pub async fn send_message(&self, message: BarkMessage) -> Result<(), Box<dyn Error>> {
        let url = Self::build_url(&self.server, self.key.as_str(), message)?;
        let resp = reqwest::Client::builder()
            .build()?
            .get(url)
            .send().await?;

        let json_value = resp.json::<Value>().await?;
        let code = json_value["code"].as_u64();
        if let Some(code) = code {
            if code != 200u64 {
                return Err(format!("bark同步失败：{code}").into());
            }
        }
        Ok(())
    }


    fn build_url(server: &str, key: &str, message: BarkMessage) -> Result<String, Box<dyn Error>> {
        let mut url = Url::parse(server)?;
        {
            let mut binding = url.path_segments_mut().unwrap();
            let segments = binding.clear();
            segments.push(key);
            if let Some(title) = message.title {
                segments.push(&title);
            }
            segments.push(&message.body);
        }
        {
            let mut query = url.query_pairs_mut();
            let query = query.clear();

            if let Some(copy) = message.copy {
                query.append_pair("copy", copy.as_str());
            };
            if let Some(url) = message.url {
                query.append_pair("url", url.as_str());
            };
            if let Some(is_archive) = message.is_archive {
                query.append_pair("isArchive", is_archive.to_string().as_str());
            };
            if let Some(automatically_copy) = message.automatically_copy {
                query.append_pair("automaticallyCopy", automatically_copy.to_string().as_str());
            }
            if let Some(icon) = message.icon {
                query.append_pair("icon", icon.as_str());
            }
            if let Some(group) = message.group {
                query.append_pair("group", group.as_str());
            }

            match message.level {
                Level::Active => { query.append_pair("level", "active"); }
                Level::TimeSensitive => { query.append_pair("level", "timeSensitive"); }
                Level::Passive => { query.append_pair("passive", "passive"); }
            }
        }
        Ok(url.as_str().to_string())
    }
}

pub struct BarkMessage {
    pub title: Option<String>,
    pub body: String,
    pub automatically_copy: Option<u32>,
    pub copy: Option<String>,
    pub url: Option<String>,
    pub is_archive: Option<u32>,
    pub group: Option<String>,
    pub icon: Option<String>,
    pub level: Level,
}

#[allow(dead_code)]
pub enum Level {
    Active,
    TimeSensitive,
    Passive,
}

impl Default for BarkMessage {
    fn default() -> Self {
        BarkMessage {
            title: None,
            body: "".to_string(),
            automatically_copy: None,
            copy: None,
            url: None,
            is_archive: None,
            group: None,
            icon: None,
            level: Level::Active,
        }
    }
}