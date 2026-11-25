use super::{Bot, markov};
use crate::{senryu::SenryuStore, text::Dictionary, traq::TraqClient};
use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};
use reqwest::Url;
use serde_json::{Value, json};
use sqlx::mysql::MySqlPoolOptions;
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::mpsc, time::timeout};
use uuid::Uuid;

const MENTION: &str = "@BOT_lachite";
const ALICE: Uuid = Uuid::from_u128(1);
const BOB: Uuid = Uuid::from_u128(2);
const CHANNEL: Uuid = Uuid::from_u128(3);
const DM: Uuid = Uuid::from_u128(4);
const CREATED_AT: &str = "2026-11-25T12:34:56Z";
const NO_NEW_TEXT: &str = "この階数では新しい文を作れませんでした。`n` を下げて試してください。";

struct Call {
    url: Url,
    body: Option<Value>,
}

impl Call {
    fn query(&self, name: &str) -> Option<String> {
        self.url
            .query_pairs()
            .find_map(|(key, value)| (key == name).then(|| value.into_owned()))
    }
}

async fn mock_traq(State(calls): State<mpsc::UnboundedSender<Call>>, request: Request) -> Response {
    let url = Url::parse(&format!("http://localhost{}", request.uri())).unwrap();
    if request.method() == Method::POST {
        let bytes = to_bytes(request.into_body(), 16384).await.unwrap();
        calls
            .send(Call {
                url,
                body: Some(serde_json::from_slice(&bytes).unwrap()),
            })
            .unwrap();
        return StatusCode::CREATED.into_response();
    }

    let call = Call { url, body: None };
    let reply = match call.url.path() {
        "/api/v3/users" => match call.query("name").as_deref() {
            Some("alice") => json!([{ "id": ALICE, "name": "alice" }]),
            Some("bob") => json!([{ "id": BOB, "name": "bob" }]),
            _ => json!([]),
        },
        "/api/v3/messages" => {
            let author: Uuid = call.query("from").unwrap().parse().unwrap();
            let limit = if call.query("in").is_some() { 100 } else { 1 };
            json!({ "hits": posts(&call, limit, author, CHANNEL) })
        }
        path if path == format!("/api/v3/users/{ALICE}/messages") => {
            let limit = call.query("limit").unwrap().parse().unwrap();
            let mut history = posts(&call, limit, ALICE, DM);
            history[0]["userId"] = json!(BOB);
            history[0]["channelId"] = json!(CHANNEL);
            json!(history)
        }
        path => panic!("unexpected route {path}"),
    };
    calls.send(call).unwrap();
    Json(reply).into_response()
}

fn posts(call: &Call, limit: usize, author: Uuid, channel: Uuid) -> Vec<Value> {
    let offset: u128 = call.query("offset").unwrap().parse().unwrap();
    (0..limit)
        .map(|index| {
            json!({
                "id": Uuid::from_u128(100 + offset + index as u128),
                "userId": author,
                "channelId": channel,
                "content": "**今日の予定** https://example.com\n\n> 引用文"
            })
        })
        .collect()
}

struct Fixture {
    bot: Bot,
    calls: mpsc::UnboundedReceiver<Call>,
}

impl Fixture {
    async fn new() -> Self {
        let (send_calls, calls) = mpsc::unbounded_channel();
        let api = Router::new().fallback(mock_traq).with_state(send_calls);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url: Url = format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        tokio::spawn(async { axum::serve(listener, api).await.unwrap() });

        let bot_id = Uuid::from_u128(5);
        let dictionary = Arc::new(Dictionary::sudachidict().unwrap());
        let worker_dictionary = Arc::clone(&dictionary);
        let worker_client = TraqClient::new(url.clone(), "test-token", bot_id).unwrap();
        let (markov_jobs, queue) = mpsc::channel(1);
        tokio::spawn(async move {
            markov::run(&worker_client, &worker_dictionary, MENTION, queue).await
        });

        let db = MySqlPoolOptions::new()
            .connect_lazy("mysql://rachite:unused@127.0.0.1/rachite_test")
            .unwrap();
        let bot = Bot {
            client: TraqClient::new(url, "test-token", bot_id).unwrap(),
            mention: MENTION.into(),
            store: SenryuStore::new(db),
            dictionary,
            stamp_jobs: mpsc::channel(1).0,
            markov_jobs,
        };
        Self { bot, calls }
    }

    async fn send(&self, direct: bool, command: &str) {
        let (kind, channel) = if direct {
            ("DIRECT_MESSAGE_CREATED", DM)
        } else {
            ("MESSAGE_CREATED", CHANNEL)
        };
        let text = format!("{MENTION} {command}");
        self.bot
            .handle_event(
                serde_json::from_value(json!({
                    "type": kind,
                    "body": { "message": {
                        "id": Uuid::now_v7(),
                        "channelId": channel,
                        "user": { "id": ALICE, "name": "alice", "bot": false },
                        "text": text,
                        "plainText": text,
                        "createdAt": CREATED_AT
                    } }
                }))
                .unwrap(),
            )
            .await;
    }

    async fn next_call(&mut self) -> Call {
        timeout(Duration::from_secs(5), self.calls.recv())
            .await
            .unwrap()
            .unwrap()
    }

    async fn expect_reply(&mut self, direct: bool, content: &str) {
        let reply = self.next_call().await;
        let destination = if direct {
            format!("users/{ALICE}")
        } else {
            format!("channels/{CHANNEL}")
        };
        assert_eq!(reply.url.path(), format!("/api/v3/{destination}/messages"));
        assert_eq!(
            reply.body,
            Some(json!({ "content": content, "embed": false }))
        );
    }
}

#[tokio::test]
async fn default_search_is_public_and_replies_to_the_execution_location() {
    let mut test = Fixture::new().await;
    for direct in [false, true] {
        test.send(direct, "markov @bob").await;
        assert_eq!(test.next_call().await.query("name").as_deref(), Some("bob"));
        let search = test.next_call().await;
        assert_eq!(search.url.path(), "/api/v3/messages");
        assert_eq!(search.query("from"), Some(BOB.to_string()));
        assert_eq!(search.query("in"), None);
        assert_eq!(search.query("sort").as_deref(), Some("createdAt"));
        test.expect_reply(direct, NO_NEW_TEXT).await;
    }
}

#[tokio::test]
async fn channel_here_searches_at_most_three_hundred_posts() {
    let mut test = Fixture::new().await;
    test.send(false, "markov alice 3 --here").await;
    assert_eq!(
        test.next_call().await.query("name").as_deref(),
        Some("alice")
    );
    for offset in ["0", "100", "200"] {
        let search = test.next_call().await;
        assert_eq!(search.query("in"), Some(CHANNEL.to_string()));
        assert_eq!(search.query("from"), Some(ALICE.to_string()));
        assert_eq!(search.query("limit").as_deref(), Some("100"));
        assert_eq!(search.query("offset").as_deref(), Some(offset));
    }
    test.expect_reply(false, NO_NEW_TEXT).await;
}

#[tokio::test]
async fn dm_here_only_uses_the_callers_conversation_and_filters_the_author() {
    let mut test = Fixture::new().await;
    test.send(true, "markov bob --here").await;
    assert_eq!(test.next_call().await.query("name").as_deref(), Some("bob"));
    for (offset, limit) in [("0", "200"), ("200", "100")] {
        let history = test.next_call().await;
        assert_eq!(
            history.url.path(),
            format!("/api/v3/users/{ALICE}/messages")
        );
        assert_eq!(history.query("limit").as_deref(), Some(limit));
        assert_eq!(history.query("offset").as_deref(), Some(offset));
        assert_eq!(history.query("order").as_deref(), Some("desc"));
    }
    test.expect_reply(true, "このDMに `bob` の生成に使える発言がありません。")
        .await;
}
