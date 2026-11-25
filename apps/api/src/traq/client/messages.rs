use super::TraqClient;
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{RequestBuilder, Url};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

const HISTORY_LIMIT: usize = 300;

#[derive(Clone, Copy)]
pub(crate) enum Destination {
    Channel(Uuid),
    Direct { user_id: Uuid, channel_id: Uuid },
}

impl Destination {
    pub fn channel_id(self) -> Uuid {
        match self {
            Self::Channel(id) | Self::Direct { channel_id: id, .. } => id,
        }
    }

    fn messages_path(self) -> String {
        match self {
            Self::Channel(id) => format!("channels/{id}/messages"),
            Self::Direct { user_id, .. } => format!("users/{user_id}/messages"),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryMessage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub content: String,
}

#[derive(Deserialize)]
struct SearchResults {
    hits: Vec<HistoryMessage>,
}

impl TraqClient {
    pub fn message_link(&self, message_id: Uuid) -> String {
        format!("{}messages/{message_id}", self.origin)
    }

    pub fn message_id(&self, input: &str) -> Option<Uuid> {
        let link = input
            .trim_start_matches('<')
            .trim_end_matches('>')
            .parse::<Url>()
            .ok()?;
        if link.origin() != self.origin.origin() {
            return None;
        }
        link.path().strip_prefix("/messages/")?.parse().ok()
    }

    pub async fn reply(&self, channel_id: Uuid, content: &str) -> Result<()> {
        self.post_message(Destination::Channel(channel_id), content, true)
            .await
    }

    pub async fn post_message(
        &self,
        destination: Destination,
        content: &str,
        embed: bool,
    ) -> Result<()> {
        self.http
            .post(self.api_url(&destination.messages_path()))
            .json(&json!({ "content": content, "embed": embed }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn search_history(
        &self,
        user_id: Uuid,
        channel_id: Option<Uuid>,
        before: DateTime<Utc>,
    ) -> Result<Vec<HistoryMessage>> {
        const PAGE_SIZE: usize = 100;
        let mut history = Vec::new();
        let mut url = self.api_url("messages");
        url.query_pairs_mut()
            .append_pair("from", &user_id.to_string())
            .append_pair("before", &before.to_rfc3339())
            .append_pair("sort", "createdAt");
        if let Some(channel_id) = channel_id {
            url.query_pairs_mut()
                .append_pair("in", &channel_id.to_string());
        }
        for offset in (0..HISTORY_LIMIT).step_by(PAGE_SIZE) {
            let limit = PAGE_SIZE.min(HISTORY_LIMIT - offset);
            let page: SearchResults = self
                .history_page(&url, limit, offset)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let complete = page.hits.len() < limit;
            history.extend(page.hits.into_iter().take(limit));
            if complete {
                break;
            }
        }
        Ok(history)
    }

    pub async fn direct_history(
        &self,
        user_id: Uuid,
        before: DateTime<Utc>,
    ) -> Result<Vec<HistoryMessage>> {
        const PAGE_SIZE: usize = 200;
        let mut history = Vec::new();
        let mut url = self.api_url(&format!("users/{user_id}/messages"));
        url.query_pairs_mut()
            .append_pair("until", &before.to_rfc3339())
            .append_pair("order", "desc");
        for offset in (0..HISTORY_LIMIT).step_by(PAGE_SIZE) {
            let limit = PAGE_SIZE.min(HISTORY_LIMIT - offset);
            let page: Vec<HistoryMessage> = self
                .history_page(&url, limit, offset)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let complete = page.len() < limit;
            history.extend(page.into_iter().take(limit));
            if complete {
                break;
            }
        }
        Ok(history)
    }

    fn history_page(&self, url: &Url, limit: usize, offset: usize) -> RequestBuilder {
        let mut url = url.clone();
        url.query_pairs_mut()
            .append_pair("limit", &limit.to_string())
            .append_pair("offset", &offset.to_string());
        self.http.get(url)
    }
}
