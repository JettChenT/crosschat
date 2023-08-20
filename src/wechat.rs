use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatLogResponse {
    #[serde(rename = "hasMore")]
    has_more: i8,

    #[serde(rename = "chatLogs")]
    pub chat_logs: Vec<ChatLogItem>,
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
pub struct ChatLogItem {
    #[serde(rename = "fromUser")]
    pub from_user: String,
    #[serde(rename = "toUser")]
    pub to_user: String,
    pub content: String,
    #[serde(rename = "createTime")]
    pub create_time: i64,
    #[serde(rename = "isSentFromSelf")]
    pub is_sent_from_self: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Contact {
    icon: HashMap<String, Option<String>>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub arg: String,
    pub valid: i32,
}

pub async fn fetch_chatlog(
    user_id: String,
    count: Option<i64>,
    client: Option<reqwest::Client>,
) -> Result<ChatLogResponse> {
    let r_client = match client {
        Some(c) => c,
        None => reqwest::Client::new(),
    };
    let mut url = format!("http://localhost:48065/wechat/chatlog?userId={}", user_id);
    if let Some(c) = count {
        url.push_str(format!("&count={}", c).as_str());
    }
    // println!("url: {}", url);
    let res = r_client.get(url).send().await?;
    // println!("Got res {:?}", res.status());
    let res_json = res.json::<ChatLogResponse>().await?;
    // println!("Got json: {:?}", res_json);
    Ok(res_json)
}

pub async fn send_msg(
    user_id: String,
    message: String,
    client: Option<reqwest::Client>,
) -> Result<()> {
    let r_client = match client {
        Some(c) => c,
        None => reqwest::Client::new(),
    };
    let res = r_client
        .post("http://localhost:48065/wechat/send")
        .query(&[("userId", user_id), ("content", message)])
        .send()
        .await?
        .json::<HashMap<String, i32>>()
        .await?;
    if res.get("sent") == Some(&1) {
        Ok(())
    } else {
        Err(color_eyre::eyre::eyre!("Failed to send message"))
    }
}

pub async fn retrieve_contacts(client: Option<reqwest::Client>) -> Result<Vec<Contact>> {
    let r_client = match client {
        Some(c) => c,
        None => reqwest::Client::new(),
    };
    let res = r_client
        .get("http://localhost:48065/wechat/allcontacts")
        .send()
        .await?
        .json::<Vec<Contact>>()
        .await?;
    Ok(res)
}

impl ToString for Contact {
    fn to_string(&self) -> String {
        match (self.title.clone(), self.subtitle.clone()) {
            (Some(t), Some(s)) => format!("{} | {} | {}", t, s, self.arg),
            (Some(t), None) => format!("{} | {}", t, self.arg),
            (None, Some(s)) => format!("{} | {}", s, self.arg),
            (None, None) => self.arg.clone(),
        }
    }
}
