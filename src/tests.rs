use color_eyre::eyre::Result;
use irc::client::prelude::*;
use std::time::Duration;

#[cfg(test)]
pub mod tests {
    use super::*;
    use futures::Stream;

    #[tokio::test]
    async fn tst_privmsg() -> Result<()> {
        color_eyre::install()?;
        let config = Config {
            nickname: Some("wctxt".to_string()),
            server: Some("127.0.0.1".to_string()),
            use_tls: Some(false),
            username: Some("wctxt".to_string()),
            ..Config::default()
        };
        let mut client = Client::from_config(config).await?;
        client.identify()?;
        let sender = client.sender();
        let stream = client.stream()?;
        let target = "Jett-Rusty";
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            sender.send_privmsg(target, "hello!")?;
        }
        Ok(())
    }
}
