mod messages;

use anyhow::Result;
use reqwest::{Client, Url, header};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::client::IntoClientRequest,
};
use uuid::Uuid;

pub(crate) use messages::Destination;

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

#[derive(Deserialize)]
struct Account {
    id: Uuid,
    name: String,
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
            .find(|account| account.name.eq_ignore_ascii_case(name))
            .map(|account| account.id))
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
