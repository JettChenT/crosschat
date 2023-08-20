use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use color_eyre::eyre::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatLogResponse{
    #[serde(rename = "hasMore")]
    has_more: i8,

    #[serde(rename = "chatLogs")]
    pub chat_logs: Vec<ChatLogItem>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatLogItem{
    #[serde(rename = "fromUser")]
    pub from_user: String,
    #[serde(rename = "toUser")]
    pub to_user: String,
    pub content: String,
    #[serde(rename = "createTime")]
    pub create_time: i64,
    #[serde(rename = "isSentFromSelf")]
    pub is_sent_from_self: bool
}

pub async fn fetch_chatlog(user_id: String, count: Option<i64>, client: Option<reqwest::Client>) -> Result<ChatLogResponse>{
    let r_client = match client {
        Some(c) => c,
        None => reqwest::Client::new()
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

pub async fn send_msg(user_id: String, message: String, client: Option<reqwest::Client>) -> Result<()>{
    let r_client = match client {
        Some(c) => c,
        None => reqwest::Client::new()
    };
    let res = r_client.post("http://localhost:48065/wechat/send")
        .query(&[("userId", user_id), ("content", message)])
        .send()
        .await?
        .json::<HashMap<String, i32>>()
        .await?;
    if res.get("sent") == Some(&1){
        Ok(())
    }else{
        Err(color_eyre::eyre::eyre!("Failed to send message"))
    }
}