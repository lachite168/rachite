use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{Client, RequestBuilder, Url, header};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::client::IntoClientRequest,
};
use uuid::Uuid;

const HISTORY_LIMIT: usize = 300;

pub(crate) struct TraqClient {
    http: Client,
    origin: Url,
    auth: header::HeaderValue,
    bot_id: Uuid,
}

#[derive(Deserialize)]
pub(crate) struct Stamp {
    pub id: Uuid,
    pub name: String,
}

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
struct Account {
    id: Uuid,
    name: String,
}

#[derive(Deserialize)]
struct SearchResults {
    hits: Vec<HistoryMessage>,
}

impl TraqClient {
    pub fn new(origin: Url, token: &str, bot_id: Uuid) -> Result<Self> {
        let mut auth = header::HeaderValue::from_str(&format!("Bearer {token}"))?;
        auth.set_sensitive(true);
        let mut headers = header::HeaderMap::new();
        headers.insert(header::AUTHORIZATION, auth.clone());

        let http = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()?;
        Ok(Self {
            http,
            origin,
            auth,
            bot_id,
        })
    }

    pub async fn connect_events(&self) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let mut url = self.api_url("bots/ws");
        let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
        url.set_scheme(scheme)
            .expect("HTTP and WebSocket URLs support the same authority");
        let mut request = url.as_str().into_client_request()?;
        request
            .headers_mut()
            .insert(header::AUTHORIZATION, self.auth.clone());
        let (stream, _) =
            tokio::time::timeout(Duration::from_secs(10), connect_async(request)).await??;
        Ok(stream)
    }

    fn api_url(&self, path: &str) -> Url {
        let mut url = self.origin.clone();
        url.set_path(&format!("/api/v3/{path}"));
        url
    }

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

    pub async fn user_id(&self, name: &str) -> Result<Option<Uuid>> {
        let mut url = self.api_url("users");
        url.query_pairs_mut().append_pair("name", name);
        let accounts: Vec<Account> = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(accounts
            .into_iter()
            .find(|account| account.name == name)
            .map(|account| account.id))
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

    pub async fn set_channel_membership(&self, channel_id: Uuid, join: bool) -> Result<()> {
        let action = if join { "join" } else { "leave" };
        self.http
            .post(self.api_url(&format!("bots/{}/actions/{action}", self.bot_id)))
            .json(&json!({ "channelId": channel_id }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn stamps(&self) -> Result<Vec<Stamp>> {
        Ok(self
            .http
            .get(self.api_url("stamps"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn add_stamp(&self, message_id: Uuid, stamp_id: Uuid, count: u32) -> Result<()> {
        let response = self
            .http
            .post(self.api_url(&format!("messages/{message_id}/stamps/{stamp_id}")))
            .json(&json!({ "count": count }))
            .send()
            .await?
            .error_for_status()?;
        anyhow::ensure!(
            response.status().is_success(),
            "Unexpected stamp response: {}",
            response.status()
        );
        Ok(())
    }
}
