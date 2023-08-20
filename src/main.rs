mod wechat;
mod util;

use irc::client::prelude::*;
use futures::prelude::*;
use color_eyre::eyre::Result;
use std::env;
use crate::wechat::{fetch_chatlog, send_msg};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::util::get_timestamp;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::time;
use irc::client::ClientStream;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // You can spawn multiple clients here
    let target_username = "jett".to_owned();
    tokio::try_join!(
        spawn_client("Sam".to_owned(), "wxid_asdfasdf".to_owned(), target_username.clone()),
        // Add more as needed
    )?;

    Ok(())
}

struct SendMsg{
    pub channel: String,
    pub msg: String
}

async fn spawn_client(nickname: String, target_contact_id: String, target_username: String) -> Result<()> {
    let config = Config {
        nickname: Some(nickname),
        server: Some("127.0.0.1".to_owned()),
        use_tls: Some(false),
        ..Config::default()
    };

    let (tx, mut rx): (Sender<SendMsg>, Receiver<SendMsg>) = mpsc::channel(32);
    let (itx, mut irx): (Sender<SendMsg>, Receiver<SendMsg>) = mpsc::channel(32);

    let tuser = target_username.clone();
    let tcontact = target_contact_id.clone();
    let wechat_watch = tokio::spawn(async move {
        spawn_watcher(tcontact, tuser, tx).await;
    });


    let mut client = Client::from_config(config).await?;
    client.identify()?;
    let mut stream = client.stream()?;
    let tuser = target_username.clone();
    let irc_watch = tokio::spawn(async move {
        spawn_receiver(stream, tuser, itx).await;
    });

    let reqw_client = reqwest::Client::new();

    loop {
        // println!("looping");
        if let Some(msg) = irx.try_recv().ok() {
            println!("sending {} to {} in wechat", msg.msg, msg.channel);
            send_msg(target_contact_id.clone(), msg.msg, Some(reqw_client.clone())).await?;
        }

        if let Some(msg) = rx.try_recv().ok() {
            println!("sending {} to {} in irc", msg.msg, msg.channel);
            client.send_privmsg(&msg.channel, &msg.msg).unwrap();
        }
        thread::sleep(time::Duration::from_millis(100));
    }
}

async fn spawn_receiver(mut stream: ClientStream, watch_sender: String, sender: Sender<SendMsg>) -> Result<()>{
    println!("spawn_receiver");
    while let Some(message) = stream.next().await.transpose()? {
        if let Command::PRIVMSG(channel, msg) = message.clone().command {
            let from = message.source_nickname().unwrap();
            println!("{} <=> {}", from, watch_sender);
            if from == watch_sender {
                sender.send(SendMsg{channel: channel, msg: msg}).await?;
            }
        }
    }
    Ok(())
}

async fn spawn_watcher(watch_wxid: String, target_channel: String, sender: Sender<SendMsg>) -> Result<()>{
    let req_client = reqwest::Client::new();
    let mut last_timestamp = get_timestamp();
    println!("watching {}... {}", watch_wxid, target_channel);
    loop {
        thread::sleep(time::Duration::from_millis(1000));
        let chat_logs = fetch_chatlog(watch_wxid.clone(), Some(5), Some(req_client.clone())).await?;
        for log in chat_logs.chat_logs {
            // println!("{}: {} -- {}", log.create_time, log.content, last_timestamp);
            if !log.is_sent_from_self && log.create_time >= last_timestamp {
                println!("{}: {}", log.create_time, log.content);
                let msg = format!("{}", log.content);
                sender.send(SendMsg{channel: target_channel.clone(), msg: msg}).await?;
                println!("sent");
            }
        }
        last_timestamp = get_timestamp();
    }
}