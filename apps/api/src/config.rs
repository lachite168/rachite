use anyhow::{Context, Result, ensure};
use reqwest::Url;
use std::env;
use uuid::Uuid;

pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub traq_origin: Url,
    pub command_mention: String,
    pub bot: Option<BotConfig>,
}

pub struct BotConfig {
    pub id: Uuid,
    pub token: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let traq_origin = env::var("TRAQ_ORIGIN")
            .unwrap_or_else(|_| "https://q.trap.jp".into())
            .parse::<Url>()
            .context("Invalid TRAQ_ORIGIN")?;
        ensure!(
            matches!(traq_origin.scheme(), "http" | "https")
                && traq_origin.host().is_some()
                && traq_origin.path() == "/"
                && traq_origin.username().is_empty()
                && traq_origin.password().is_none()
                && traq_origin.query().is_none()
                && traq_origin.fragment().is_none(),
            "TRAQ_ORIGIN must be an HTTP(S) origin without a path or credentials"
        );

        let bot_name = env::var("TRAQ_BOT_NAME").unwrap_or_else(|_| "BOT_lachite".into());
        ensure!(
            !bot_name.is_empty()
                && bot_name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')),
            "Invalid TRAQ_BOT_NAME"
        );

        let token = env::var("TRAQ_BOT_TOKEN").ok().filter(|s| !s.is_empty());
        let bot_id = env::var("TRAQ_BOT_ID").ok().filter(|s| !s.is_empty());
        let bot = match (token, bot_id) {
            (None, None) => None,
            (Some(token), Some(id)) => Some(BotConfig {
                id: id.parse().context("Invalid TRAQ_BOT_ID")?,
                token,
            }),
            _ => anyhow::bail!("Set both TRAQ_BOT_TOKEN and TRAQ_BOT_ID to enable the bot"),
        };

        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .context("Invalid PORT")?,
            traq_origin,
            command_mention: format!("@{bot_name}"),
            bot,
        })
    }
}
