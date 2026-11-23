use super::{Bot, arguments, command_catalog};
use crate::{
    bot::stampbomb::Job,
    senryu::{SenryuStore, tokenizer},
    traq::{Message, TraqClient},
};
use axum::{
    Json, Router,
    extract::{OriginalUri, State},
    http::StatusCode,
    routing::{get, post},
};
use serde_json::{Value, json};
use sqlx::mysql::MySqlPoolOptions;
use tokio::{net::TcpListener, sync::mpsc};
use uuid::Uuid;

const MENTION: &str = "@BOT_lachite";
const BOT_ID: Uuid = Uuid::from_u128(1);
const STAMP_ID: Uuid = Uuid::from_u128(2);

type Request = (String, Value);

struct TestBot {
    bot: Bot,
    channel: Uuid,
    requests: mpsc::UnboundedReceiver<Request>,
    jobs: mpsc::Receiver<Job>,
}

impl TestBot {
    async fn new() -> Self {
        let db = MySqlPoolOptions::new()
            .connect_lazy("mysql://rachite:unused@127.0.0.1/rachite_test")
            .unwrap();
        Self::with_store(SenryuStore::new(db)).await
    }

    async fn with_store(store: SenryuStore) -> Self {
        let (sender, requests) = mpsc::unbounded_channel();
        let api = Router::new()
            .route(
                "/api/v3/stamps",
                get(|| async {
                    Json(json!([
                        {"name": "Lachite_smile", "id": Uuid::from_u128(3)},
                        {"name": "old_lachite", "id": Uuid::from_u128(4)},
                        {"name": "N3_fox", "id": Uuid::from_u128(5)},
                        {"name": "old_n3", "id": Uuid::from_u128(6)},
                        {"name": "big_STAR", "id": Uuid::from_u128(7)},
                        {"name": "W", "id": STAMP_ID}
                    ]))
                }),
            )
            .route(
                "/api/v3/{*path}",
                post(
                    |State(sender): State<mpsc::UnboundedSender<Request>>,
                     OriginalUri(uri): OriginalUri,
                     Json(body): Json<Value>| async move {
                        sender.send((uri.path().into(), body)).unwrap();
                        StatusCode::NO_CONTENT
                    },
                ),
            )
            .with_state(sender);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async { axum::serve(listener, api).await.unwrap() });

        let (stamp_jobs, jobs) = mpsc::channel(1);
        Self {
            bot: Bot {
                client: TraqClient::new(origin.parse().unwrap(), "test-token", BOT_ID).unwrap(),
                mention: MENTION.into(),
                store,
                tokenizer: tokenizer().unwrap(),
                stamp_jobs,
            },
            channel: Uuid::now_v7(),
            requests,
            jobs,
        }
    }

    fn message(&self, command: &str) -> Message {
        let text = format!("{MENTION} {command}");
        serde_json::from_value(json!({
            "id": Uuid::now_v7(),
            "channelId": self.channel,
            "user": {"name": "alice", "bot": false},
            "text": text,
            "plainText": text,
            "createdAt": chrono::Utc::now()
        }))
        .unwrap()
    }

    async fn send(&self, command: &str) {
        self.bot
            .handle_created(&self.message(command))
            .await
            .unwrap();
    }

    fn reply(&mut self) -> String {
        let (path, body) = self.requests.try_recv().expect("expected a Bot reply");
        assert_eq!(path, format!("/api/v3/channels/{}/messages", self.channel));
        body["content"].as_str().unwrap().into()
    }

    async fn expect_reply(&mut self, command: &str, expected: &str) {
        self.send(command).await;
        assert_eq!(self.reply(), expected, "{command}");
    }
}

#[test]
fn recognizes_only_a_leading_bot_mention() {
    let words = arguments(MENTION, "  @bot_LACHITE   ping Hello ignored").unwrap();
    assert_eq!(words.collect::<Vec<_>>(), ["ping", "Hello", "ignored"]);
    assert_eq!(arguments(MENTION, MENTION).unwrap().next(), None);
    assert!(arguments(MENTION, "hello @BOT_lachite ping").is_none());
    assert!(arguments(MENTION, "@BOT_lachite_extra ping").is_none());
}

#[tokio::test]
async fn mention_ping_and_help_reply_in_the_command_channel() {
    let mut test = TestBot::new().await;
    test.expect_reply("", "@alice").await;
    for (command, prefix) in [
        ("ping", "@alice pong!  ("),
        ("PiNg hello", "@alice pong! hello ("),
    ] {
        test.send(command).await;
        let reply = test.reply();
        let elapsed = reply
            .strip_prefix(prefix)
            .unwrap()
            .strip_suffix(" ms)")
            .unwrap();
        assert!(elapsed.parse::<i64>().is_ok(), "{reply}");
    }

    test.send("help").await;
    let help = test.reply();
    let commands = command_catalog();
    assert_eq!(help.lines().count(), commands.len());
    for command in commands {
        let invocation = if command.usage.is_empty() {
            MENTION.into()
        } else {
            format!("{MENTION} {}", command.usage)
        };
        assert!(
            help.lines()
                .any(|line| line == format!("`{invocation}`  {}", command.description))
        );
    }

    test.send("unknown-command").await;
    assert!(test.requests.try_recv().is_err());
}

#[tokio::test]
async fn join_and_leave_use_the_configured_bot_and_command_channel() {
    let mut test = TestBot::new().await;
    for (command, reply) in [
        ("join", "このチャンネルに参加しました！"),
        ("leave", "このチャンネルから退出しました！"),
    ] {
        test.send(command).await;
        let (path, body) = test.requests.try_recv().unwrap();
        assert_eq!(path, format!("/api/v3/bots/{BOT_ID}/actions/{command}"));
        assert_eq!(body, json!({"channelId": test.channel}));
        assert_eq!(test.reply(), reply);
    }
}

#[tokio::test]
async fn gacha_commands_match_and_report_missing_stamps() {
    let mut test = TestBot::new().await;
    for (command, reply) in [
        ("lachitegacha", ":Lachite_smile.ex-large:"),
        ("n3gacha", ":N3_fox.ex-large:"),
        ("stampgacha sTaR", ":big_STAR.ex-large:"),
        (
            "stampgacha absent",
            "absent を含むスタンプが見つかりませんでした",
        ),
        ("stampgacha", "使い方: `@BOT_lachite stampgacha <search>`"),
    ] {
        test.expect_reply(command, reply).await;
    }
}

#[tokio::test]
async fn stampbomb_enqueues_the_quoted_message_and_reports_a_full_queue() {
    let mut test = TestBot::new().await;
    let target = Uuid::now_v7();
    let link = test.bot.client.message_link(target);
    let mut message = test.message("stampbomb :w: 1234 [引用メッセージ]");
    message.text = format!("{MENTION} stampbomb :w: 1234\n{link}");
    test.bot.handle_created(&message).await.unwrap();
    assert_eq!(
        test.reply(),
        format!("@alice の代わりに :W: を 1234 回連打します\n{link}")
    );

    test.expect_reply(
        &format!("stampbomb w 1 {link}"),
        "スタンプ送信が混雑中です。しばらくしてからお試しください。",
    )
    .await;
    let job = test.jobs.try_recv().unwrap();
    assert_eq!(job.message_id, target);
    assert_eq!(job.stamp_id, STAMP_ID);
    assert_eq!(job.requester_channel, test.channel);
    assert_eq!(job.count, 1234);
    assert!(test.jobs.try_recv().is_err());
}

#[tokio::test]
async fn stampbomb_rejects_invalid_arguments_without_enqueuing() {
    let mut test = TestBot::new().await;
    let link = test.bot.client.message_link(Uuid::now_v7());
    let usage = "使い方: `@BOT_lachite stampbomb <stamp> <number> <message-link>`";
    let range = "回数 <number> は1〜10000の間で指定してください";
    for (command, reply) in [
        ("stampbomb w 1".into(), usage),
        (format!("stampbomb w many {link}"), usage),
        (format!("stampbomb w 0 {link}"), range),
        (format!("stampbomb w 10001 {link}"), range),
        (
            "stampbomb w 1 invalid-link".into(),
            "無効なメッセージリンクです",
        ),
        (
            format!("stampbomb absent 1 {link}"),
            "スタンプ absent が見つかりませんでした",
        ),
    ] {
        test.expect_reply(&command, reply).await;
        assert!(test.jobs.try_recv().is_err(), "{command}");
    }
}

#[tokio::test]
async fn senryu_reports_usage_for_missing_or_invalid_actions() {
    let mut test = TestBot::new().await;
    let usage = "使い方: `@BOT_lachite senryu <status|on|off>`";
    test.expect_reply("senryu", usage).await;
    test.expect_reply("senryu invalid", usage).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL for a dedicated MariaDB database"]
async fn senryu_commands_persist_settings_per_channel() {
    let url = std::env::var("TEST_DATABASE_URL").expect("Set TEST_DATABASE_URL to a test database");
    let db = MySqlPoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    let mut test = TestBot::with_store(SenryuStore::new(db.clone())).await;
    let original_channel = test.channel;
    let on = "このチャンネルの川柳検出はオンです。";
    let off = "このチャンネルの川柳検出はオフです。";

    test.expect_reply("senryu status", on).await;
    test.expect_reply("senryu off", off).await;
    test.expect_reply("senryu status", off).await;
    test.channel = Uuid::now_v7();
    test.expect_reply("senryu status", on).await;
    test.channel = original_channel;
    test.expect_reply("senryu on", on).await;
    test.expect_reply("senryu status", on).await;
    sqlx::query("DELETE FROM channel_settings WHERE channel_id = ?")
        .bind(original_channel.to_string())
        .execute(&db)
        .await
        .unwrap();
}
