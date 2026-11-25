use super::{Bot, arguments, command_catalog};
use crate::{
    bot::stampbomb::Job,
    text::Dictionary,
    traq::{Message, TraqClient},
    verse::VerseStore,
};
use axum::{
    Json, Router,
    extract::{OriginalUri, State},
    http::StatusCode,
    routing::{get, post},
};
use serde_json::{Value, json};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use std::{sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    sync::{OnceCell, mpsc},
};
use uuid::Uuid;

const MENTION: &str = "@BOT_lachite";
const BOT_ID: Uuid = Uuid::from_u128(1);
const STAMP_ID: Uuid = Uuid::from_u128(2);
const MOGAMIGAWA_STAMPS: [Uuid; 3] = [Uuid::from_u128(8), Uuid::from_u128(9), Uuid::from_u128(10)];

type Request = (String, Value);

async fn test_database() -> MySqlPool {
    static MIGRATED: OnceCell<()> = OnceCell::const_new();
    let url = std::env::var("TEST_DATABASE_URL").expect("Set TEST_DATABASE_URL to a test database");
    let db = MySqlPoolOptions::new().connect(&url).await.unwrap();
    MIGRATED
        .get_or_init(|| async { sqlx::migrate!().run(&db).await.unwrap() })
        .await;
    db
}

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
        Self::with_store(VerseStore::new(db)).await
    }

    async fn with_store(store: VerseStore) -> Self {
        Self::with_failing_stamp(store, None).await
    }

    async fn with_failing_stamp(store: VerseStore, failing_stamp: Option<Uuid>) -> Self {
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
                        {"name": "W", "id": STAMP_ID},
                        {"name": "foo/bar", "id": Uuid::from_u128(11)},
                        {"name": "prefix/foo", "id": Uuid::from_u128(12)},
                        {"name": format!("{}b", "a".repeat(28)), "id": Uuid::from_u128(13)},
                        {"name": "mogamigawa03_gawa", "id": MOGAMIGAWA_STAMPS[2]},
                        {"name": "mogamigawa01_mo", "id": MOGAMIGAWA_STAMPS[0]},
                        {"name": "mogamigawa02_gami", "id": MOGAMIGAWA_STAMPS[1]}
                    ]))
                }),
            )
            .route(
                "/api/v3/{*path}",
                post(
                    move |State(sender): State<mpsc::UnboundedSender<Request>>,
                          OriginalUri(uri): OriginalUri,
                          Json(body): Json<Value>| async move {
                        sender.send((uri.path().into(), body)).unwrap();
                        if failing_stamp
                            .is_some_and(|id| uri.path().ends_with(&format!("/stamps/{id}")))
                        {
                            StatusCode::SERVICE_UNAVAILABLE
                        } else {
                            StatusCode::NO_CONTENT
                        }
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
                dictionary: Arc::new(Dictionary::sudachidict().unwrap()),
                stamp_jobs,
                markov_jobs: mpsc::channel(1).0,
            },
            channel: Uuid::now_v7(),
            requests,
            jobs,
        }
    }

    fn message(&self, command: &str) -> Message {
        let text = format!("{MENTION} {command}");
        self.channel_message(&text, &text)
    }

    fn channel_message(&self, text: &str, plain_text: &str) -> Message {
        serde_json::from_value(json!({
            "id": Uuid::now_v7(),
            "channelId": self.channel,
            "user": {"id": Uuid::from_u128(12), "name": "alice", "bot": false},
            "text": text,
            "plainText": plain_text,
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

    fn expect_mogamigawa_reply_and_stamps(&mut self, message: &Message, stamps: &[Uuid]) {
        assert_eq!(
            self.reply(),
            format!("最上川\n{}", self.bot.client.message_link(message.id))
        );
        for stamp in stamps {
            let (path, body) = self.requests.try_recv().expect("expected a stamp request");
            assert_eq!(
                path,
                format!("/api/v3/messages/{}/stamps/{stamp}", message.id)
            );
            assert_eq!(body, json!({"count": 1}));
        }
        assert!(self.requests.try_recv().is_err());
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
        ("stampgacha sTaR ignored", ":big_STAR.ex-large:"),
        (
            "stampgacha ^big_",
            "^big_ を含むスタンプが見つかりませんでした",
        ),
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
async fn stampgacha_supports_ecmascript_regexp_literals() {
    let mut test = TestBot::new().await;
    for (command, reply) in [
        ("stampgacha /^BIG_[a-z]+$/ memo", ":big_STAR.ex-large:"),
        ("stampgacha /^n3_fox$/i", ":N3_fox.ex-large:"),
        ("stampgacha /^n3_fox$/u", ":N3_fox.ex-large:"),
        ("stampgacha /^n3_[a-z]+(?=x$)x$/v", ":N3_fox.ex-large:"),
        (r"stampgacha /^n3(\x5f)fox$/", ":N3_fox.ex-large:"),
        (r"stampgacha /^foo\/bar$/", ":foo/bar.ex-large:"),
        ("stampgacha /^foo[/]bar$/", ":foo/bar.ex-large:"),
        ("stampgacha /foo", ":prefix/foo.ex-large:"),
        ("stampgacha /foo bar/", ":prefix/foo.ex-large:"),
        ("stampgacha /(?-i:^big_STAR$)/", ":big_STAR.ex-large:"),
        (
            "stampgacha /(?-i:^BIG_)/",
            "正規表現に一致するスタンプが見つかりませんでした",
        ),
        (
            "stampgacha /^absent$/",
            "正規表現に一致するスタンプが見つかりませんでした",
        ),
    ] {
        test.expect_reply(command, reply).await;
    }
    for search in ["//", "/[/", "/foo/bar", "/foo/ii", "/foo/uv"] {
        test.expect_reply(
            &format!("stampgacha {search}"),
            "無効な正規表現です。パターンとフラグを確認してください",
        )
        .await;
    }
}

#[tokio::test]
async fn stampgacha_reports_regexp_timeout_and_handles_the_next_command() {
    let mut test = TestBot::new().await;
    tokio::time::timeout(
        Duration::from_secs(5),
        test.expect_reply(
            "stampgacha /^(a+)+$/",
            "正規表現の照合が時間切れになりました",
        ),
    )
    .await
    .expect("QuickJS should interrupt matching");
    test.expect_reply("stampgacha /^n3_fox$/", ":N3_fox.ex-large:")
        .await;
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
async fn detection_commands_report_usage_for_missing_or_invalid_actions() {
    let mut test = TestBot::new().await;
    let usage = "使い方: `@BOT_lachite senryu <status|on|off>`";
    test.expect_reply("senryu", usage).await;
    test.expect_reply("senryu invalid", usage).await;
    let usage = "使い方: `@BOT_lachite mogamigawa <status|on|off>`";
    test.expect_reply("mogamigawa", usage).await;
    test.expect_reply("mogamigawa invalid", usage).await;
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL for a dedicated MariaDB database"]
async fn senryu_commands_persist_settings_per_channel() {
    let db = test_database().await;
    let mut test = TestBot::with_store(VerseStore::new(db.clone())).await;
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

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL for a dedicated MariaDB database"]
async fn senryu_events_notify_once_and_keep_published_entries_in_sync() {
    let db = test_database().await;
    let mut test = TestBot::with_store(VerseStore::new(db.clone())).await;
    let channel = test.channel;
    let message_id = Uuid::now_v7();
    let original =
        "1. **古池や**\n2. [蛙飛びこむ](https://example.com)\n\n[添付ファイル]\n\n---\n\n> 水の音";
    let edited = "柿くへば [添付ファイル] **鐘が鳴るなり** 法隆寺";
    let attachment = r#"!{"raw":"","type":"file","id":"ee764d5f-71d9-4a40-bc7b-547d8d097c91"}"#;
    let message_event = |kind: &str, is_bot: bool, text: &str| {
        serde_json::from_value(json!({
            "type": kind,
            "body": {"message": {
                "id": message_id,
                "channelId": channel,
                "user": {"id": Uuid::from_u128(12), "name": "alice", "bot": is_bot},
                "text": text.replace("[添付ファイル]", attachment),
                "plainText": text,
                "createdAt": "2026-11-23T01:02:03Z"
            }}
        }))
        .unwrap()
    };
    let stored_verse = || {
        sqlx::query_scalar::<_, String>("SELECT verse FROM senryu WHERE message_id = ?")
            .bind(message_id.to_string())
            .fetch_optional(&db)
    };

    test.bot
        .handle_event(message_event("MESSAGE_CREATED", true, original))
        .await;
    assert!(test.requests.try_recv().is_err());
    assert_eq!(stored_verse().await.unwrap(), None);

    test.bot
        .handle_event(message_event("MESSAGE_CREATED", false, original))
        .await;
    let link = test.bot.client.message_link(message_id);
    assert_eq!(
        test.reply(),
        format!("川柳を検出しました！\n```\n古池や\n蛙飛びこむ\n水の音\n```\n{link}")
    );
    test.bot
        .handle_event(message_event("MESSAGE_CREATED", false, original))
        .await;
    assert!(test.requests.try_recv().is_err());

    test.expect_reply("senryu off", "このチャンネルの川柳検出はオフです。")
        .await;
    test.bot
        .handle_event(message_event("MESSAGE_UPDATED", false, edited))
        .await;
    assert_eq!(
        stored_verse().await.unwrap().as_deref(),
        Some("柿くへば\n鐘が鳴るなり\n法隆寺")
    );
    assert!(test.requests.try_recv().is_err());

    test.bot
        .handle_event(
            serde_json::from_value(json!({
                "type": "MESSAGE_DELETED",
                "body": {"message": {"id": message_id}}
            }))
            .unwrap(),
        )
        .await;
    assert_eq!(stored_verse().await.unwrap(), None);
    assert!(test.requests.try_recv().is_err());
    sqlx::query("DELETE FROM channel_settings WHERE channel_id = ?")
        .bind(channel.to_string())
        .execute(&db)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL for a dedicated MariaDB database"]
async fn mogamigawa_responds_once_and_uses_its_own_channel_setting() {
    let db = test_database().await;
    let mut test = TestBot::with_store(VerseStore::new(db.clone())).await;
    let on = "このチャンネルの最上川はオンです。";
    let off = "このチャンネルの最上川はオフです。";
    test.expect_reply("mogamigawa status", on).await;

    let verse = "コーヒーは 朝に飲みます";
    let literal_citation = format!("{verse} [引用メッセージ]");
    let literal = test.channel_message(&literal_citation, &literal_citation);
    test.bot.handle_created(&literal).await.unwrap();
    assert!(test.requests.try_recv().is_err());

    let link = test.bot.client.message_link(Uuid::now_v7());
    let message = test.channel_message(&format!("{verse} {link}"), &literal_citation);
    test.bot.handle_created(&message).await.unwrap();
    test.expect_mogamigawa_reply_and_stamps(&message, &MOGAMIGAWA_STAMPS);
    let entries = test.bot.store.list_mogamigawa(None, 100).await.unwrap();
    let recorded = entries
        .iter()
        .find(|entry| entry.message_id == message.id.to_string())
        .unwrap();
    assert_eq!(
        recorded.verse.as_deref(),
        Some("コーヒーは\n朝に飲みます\n最上川")
    );
    test.bot.handle_created(&message).await.unwrap();
    assert!(test.requests.try_recv().is_err());

    test.expect_reply("senryu off", "このチャンネルの川柳検出はオフです。")
        .await;
    let senryu_disabled = test.channel_message(verse, verse);
    test.bot.handle_created(&senryu_disabled).await.unwrap();
    test.expect_mogamigawa_reply_and_stamps(&senryu_disabled, &MOGAMIGAWA_STAMPS);

    test.expect_reply("mogamigawa off", off).await;
    test.expect_reply("mogamigawa status", off).await;
    let channel = test.channel;
    test.channel = Uuid::now_v7();
    test.expect_reply("mogamigawa status", on).await;
    test.channel = channel;
    test.expect_reply("senryu status", "このチャンネルの川柳検出はオフです。")
        .await;
    test.expect_reply("senryu on", "このチャンネルの川柳検出はオンです。")
        .await;
    let disabled = test.channel_message(verse, verse);
    test.bot.handle_created(&disabled).await.unwrap();
    assert!(test.requests.try_recv().is_err());

    test.expect_reply("mogamigawa on", on).await;
    test.expect_reply("mogamigawa status", on).await;
    test.bot.handle_updated(&disabled).await.unwrap();
    assert!(test.requests.try_recv().is_err());
    let reenabled = test.channel_message(verse, verse);
    test.bot.handle_created(&reenabled).await.unwrap();
    test.expect_mogamigawa_reply_and_stamps(&reenabled, &MOGAMIGAWA_STAMPS);

    let edited_verse = "古池や 蛙飛びこむ";
    let mut edited = test.channel_message(edited_verse, edited_verse);
    edited.id = message.id;
    test.bot.handle_updated(&edited).await.unwrap();
    let entries = test.bot.store.list_mogamigawa(None, 100).await.unwrap();
    let updated = entries
        .iter()
        .find(|entry| entry.message_id == message.id.to_string())
        .unwrap();
    assert_eq!(updated.verse.as_deref(), Some("古池や\n蛙飛びこむ\n最上川"));
    assert_eq!(updated.detected_at, recorded.detected_at);

    edited.text = "古池や zzzzz 蛙飛びこむ".into();
    edited.plain_text.clone_from(&edited.text);
    test.bot.handle_updated(&edited).await.unwrap();
    test.bot.handle_deleted(reenabled.id).await.unwrap();
    test.bot.handle_created(&reenabled).await.unwrap();
    let entries = test.bot.store.list_mogamigawa(None, 100).await.unwrap();
    for hidden in [message.id, disabled.id, reenabled.id] {
        assert!(
            !entries
                .iter()
                .any(|entry| entry.message_id == hidden.to_string())
        );
    }
    assert!(test.requests.try_recv().is_err());

    for message in [message, senryu_disabled, reenabled] {
        sqlx::query("DELETE FROM mogamigawa_responses WHERE message_id = ?")
            .bind(message.id.to_string())
            .execute(&db)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM channel_settings WHERE channel_id = ?")
        .bind(test.channel.to_string())
        .execute(&db)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL for a dedicated MariaDB database"]
async fn mogamigawa_stops_at_a_failed_stamp_and_does_not_retry() {
    let db = test_database().await;
    let mut test =
        TestBot::with_failing_stamp(VerseStore::new(db.clone()), Some(MOGAMIGAWA_STAMPS[1])).await;
    let verse = "古池や 蛙飛びこむ";
    let message = test.channel_message(verse, verse);

    let error = test.bot.handle_created(&message).await.unwrap_err();
    assert!(error.to_string().contains("mogamigawa02_gami"));
    test.expect_mogamigawa_reply_and_stamps(&message, &MOGAMIGAWA_STAMPS[..2]);
    let entries = test.bot.store.list_mogamigawa(None, 100).await.unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.message_id == message.id.to_string())
    );
    test.bot.handle_created(&message).await.unwrap();
    assert!(test.requests.try_recv().is_err());

    sqlx::query("DELETE FROM mogamigawa_responses WHERE message_id = ?")
        .bind(message.id.to_string())
        .execute(&db)
        .await
        .unwrap();
}
