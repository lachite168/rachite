mod commands;
mod markov;
#[cfg(test)]
#[path = "../tests/bot/markov.rs"]
mod markov_tests;
mod mogamigawa;
mod stampbomb;

use crate::{
    config::BotConfig,
    text::Dictionary,
    traq::{Event, Message, TraqClient},
    verse::{self, VerseStore},
};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Url;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use utoipa::ToSchema;

pub use commands::{Command, command_catalog};

const STAMP_QUEUE_CAPACITY: usize = 32;
const MARKOV_QUEUE_CAPACITY: usize = 8;
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BotStatus {
    Disabled,
    Disconnected,
    Connected,
}

pub fn start(
    config: BotConfig,
    origin: Url,
    mention: String,
    store: VerseStore,
    status: watch::Sender<BotStatus>,
) -> Result<JoinHandle<()>> {
    let client = TraqClient::new(origin, &config.token, config.id)?;
    let dictionary = Arc::new(Dictionary::sudachidict().context("Could not load SudachiDict")?);
    let (stamp_jobs, stamp_queue) = mpsc::channel(STAMP_QUEUE_CAPACITY);
    let (markov_jobs, markov_queue) = mpsc::channel(MARKOV_QUEUE_CAPACITY);
    let bot = Bot {
        client,
        mention,
        store,
        dictionary,
        stamp_jobs,
        markov_jobs,
    };

    Ok(tokio::spawn(async move {
        tokio::join!(
            bot.listen(status),
            stampbomb::run(&bot.client, stamp_queue),
            markov::run(&bot.client, &bot.dictionary, &bot.mention, markov_queue),
        );
    }))
}

struct Bot {
    client: TraqClient,
    mention: String,
    store: VerseStore,
    dictionary: Arc<Dictionary>,
    stamp_jobs: mpsc::Sender<stampbomb::Job>,
    markov_jobs: mpsc::Sender<markov::Job>,
}

impl Bot {
    async fn listen(&self, status: watch::Sender<BotStatus>) {
        loop {
            if let Err(error) = self.receive_events(&status).await {
                tracing::warn!(%error, "traQ WebSocket session ended with an error");
            }
            status.send_replace(BotStatus::Disconnected);
            tracing::info!(?RECONNECT_DELAY, "Reconnecting to traQ");
            tokio::time::sleep(RECONNECT_DELAY).await;
        }
    }

    async fn receive_events(&self, status: &watch::Sender<BotStatus>) -> Result<()> {
        let mut socket = self.client.connect_events().await?;
        status.send_replace(BotStatus::Connected);
        tracing::info!("Connected to traQ WebSocket");
        while let Some(frame) = socket.next().await {
            let frame = frame?;
            if frame.is_close() {
                break;
            }
            if !frame.is_text() {
                continue;
            }
            match serde_json::from_str::<Event>(frame.to_text()?) {
                Ok(event) => self.handle_event(event).await,
                Err(error) => tracing::warn!(%error, "Could not decode a traQ Bot event"),
            }
        }
        Ok(())
    }

    async fn handle_event(&self, event: Event) {
        let result = match event {
            Event::MessageCreated { body } if !body.message.user.bot => {
                self.handle_created(&body.message).await
            }
            Event::DirectMessageCreated { body } if !body.message.user.bot => {
                self.dispatch_direct(&body.message).await;
                Ok(())
            }
            Event::MessageUpdated { body } if !body.message.user.bot => {
                self.handle_updated(&body.message).await
            }
            Event::MessageDeleted { body } => self.handle_deleted(body.message.id).await,
            Event::Joined => {
                tracing::debug!("Bot joined a channel");
                Ok(())
            }
            Event::Left => {
                tracing::debug!("Bot left a channel");
                Ok(())
            }
            Event::Ping => {
                tracing::trace!("Received a traQ Bot PING event");
                Ok(())
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            tracing::error!(%error, "Could not process a traQ Bot event");
        }
    }

    async fn handle_created(&self, message: &Message) -> Result<()> {
        if let Some(args) = commands::arguments(&self.mention, &message.plain_text) {
            if let Err(error) = self.dispatch(message, args).await {
                tracing::error!(%error, message_id = %message.id, "Bot command failed");
                self.client
                    .reply(message.channel_id, "コマンドの処理中にエラーが発生しました")
                    .await?;
            }
            return Ok(());
        }

        let verses = verse::detect(&message.text, &self.dictionary)
            .with_context(|| format!("Could not analyze message {}", message.id))?;
        if let Some(verse) = verses.mogamigawa {
            if self.store.is_mogamigawa_enabled(message.channel_id).await? {
                self.store
                    .record_mogamigawa(message.id, message.channel_id, &verse)
                    .await?;
                self.complete_mogamigawa(message).await?;
            }
            return Ok(());
        }
        if let Some(verse) = verses.senryu
            && self.store.is_enabled(message.channel_id).await?
            && self
                .store
                .record(message.id, message.channel_id, &verse)
                .await?
        {
            let link = self.client.message_link(message.id);
            self.client
                .reply(
                    message.channel_id,
                    &format!("川柳を検出しました！\n```\n{verse}\n```\n{link}"),
                )
                .await?;
        }
        Ok(())
    }

    async fn handle_updated(&self, message: &Message) -> Result<()> {
        let verses = match commands::arguments(&self.mention, &message.plain_text) {
            Some(_) => Default::default(),
            None => verse::detect(&message.text, &self.dictionary)
                .with_context(|| format!("Could not analyze edited message {}", message.id))?,
        };
        let allow_new =
            verses.senryu.is_some() && self.store.is_enabled(message.channel_id).await?;
        self.store
            .revise(
                message.id,
                message.channel_id,
                verses.senryu.as_deref(),
                allow_new,
            )
            .await?;
        self.store
            .revise_mogamigawa(message.id, verses.mogamigawa.as_deref())
            .await?;
        Ok(())
    }

    async fn handle_deleted(&self, message_id: uuid::Uuid) -> Result<()> {
        self.store.remove(message_id).await?;
        self.store.revise_mogamigawa(message_id, None).await?;
        Ok(())
    }
}
