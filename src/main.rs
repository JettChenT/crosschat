mod util;
mod wechat;

use color_eyre::eyre::Result;
use futures::prelude::*;
use irc::client::prelude::*;

use crate::wechat::{fetch_chatlog, send_msg};

use crate::util::get_timestamp;
use console::Term;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use irc::client::ClientStream;
use std::thread;
use std::time;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let all_contacts = wechat::retrieve_contacts(None).await?;
    // You can spawn multiple clients here
    let target_username: String = Input::new()
        .with_prompt("Please input the username of the irc user you'll be logging in to")
        .interact_text()?;

    let mut threads = Vec::new();
    loop {
        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Please select the contact you'd like to chat with")
            .items(&all_contacts)
            .default(0)
            .interact_on_opt(&Term::stderr())?;

        match selection {
            Some(index) => {
                let display_name = Input::new()
                    .with_prompt("Please input the display name you'd like to use")
                    .interact_text()?;
                let wxid = all_contacts[index].arg.clone();
                let tu_clone = target_username.clone();
                println!("Spawning thread for {} with wxid {}", display_name, wxid);
                let t = tokio::spawn(async move {
                    spawn_client(display_name, wxid, tu_clone).await;
                });
                threads.push(t);
            }
            None => {break;}
        }
    }



    for join_handle in threads {
        join_handle.await?;
    }
    Ok(())
}

struct SendMsg {
    pub channel: String,
    pub msg: String,
}

async fn spawn_client(
    nickname: String,
    target_contact_id: String,
    target_username: String,
) -> Result<()> {
    let config = Config {
        nickname: Some(nickname),
        server: Some("127.0.0.1".to_owned()),
        use_tls: Some(false),
        ..Config::default()
    };

    let (tx, mut rx): (Sender<SendMsg>, Receiver<SendMsg>) = mpsc::channel(32);
    let (itx, mut irx): (Sender<SendMsg>, Receiver<SendMsg>) = mpsc::channel(32);
    let mut client = Client::from_config(config).await?;
    client.identify()?;
    let stream = client.stream()?;
    let tuser = target_username.clone();
    eprintln!("preparing to spawn for receiver...");
    let _irc_watch = tokio::spawn(async move {
        spawn_receiver(stream, tuser, itx).await;
    });

    let tuser = target_username.clone();
    let tcontact = target_contact_id.clone();
    eprintln!("preparing to spawn for watcher...");
    let _wechat_watch = tokio::spawn(async move {
        spawn_watcher(tcontact, tuser, tx).await;
    });

    let reqw_client = reqwest::Client::new();

    loop {
        // println!("looping");
        if let Ok(msg) = irx.try_recv() {
            println!("sending {} to {} in wechat", msg.msg, msg.channel);
            send_msg(
                target_contact_id.clone(),
                msg.msg,
                Some(reqw_client.clone()),
            )
            .await?;
        }

        if let Ok(msg) = rx.try_recv() {
            println!("sending {} to {} in irc", msg.msg, msg.channel);
            client.send_privmsg(&msg.channel, &msg.msg).unwrap();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

async fn spawn_receiver(
    mut stream: ClientStream,
    watch_sender: String,
    irc_sender: Sender<SendMsg>,
    ) -> Result<()> {
    eprintln!("spawn_receiver");
    while let Some(message) = stream.next().await.transpose()? {
        if let Command::PRIVMSG(channel, msg) = message.clone().command {
            let from = message.source_nickname().unwrap();
            println!("{} <=> {}", from, watch_sender);
            if from == watch_sender {
                irc_sender.send(SendMsg { channel, msg }).await?;
            }
        }
    }
    Ok(())
}

async fn spawn_watcher(
    watch_wxid: String,
    target_channel: String,
    sender: Sender<SendMsg>,
) -> Result<()> {
    println!("watching {}... {}", watch_wxid, target_channel);
    let req_client = reqwest::Client::new();
    let mut last_timestamp = get_timestamp();
    sender
        .send(SendMsg {
            channel: target_channel.clone(),
            msg: format!(
                "=== Initializing Wechat Link with {} ===",
                watch_wxid.clone()
            ),
        })
        .await?;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        let chat_logs =
            fetch_chatlog(watch_wxid.clone(), Some(5), Some(req_client.clone())).await?;
        for log in chat_logs.chat_logs {
            if !log.is_sent_from_self && log.create_time >= last_timestamp {
                println!("{}: {}", log.create_time, log.content);
                let msg = log.content.to_string();
                sender
                    .send(SendMsg {
                        channel: target_channel.clone(),
                        msg,
                    })
                    .await?;
                println!("sent");
            }
        }
        last_timestamp = get_timestamp();
    }
}
