use axum_test::TestServer;
use rachite_api::{AppState, app, bot::BotStatus, senryu::SenryuStore};
use serde_json::{Value, json};
use sqlx::mysql::MySqlPoolOptions;
use tokio::sync::watch;
use uuid::Uuid;

fn make_server(store: SenryuStore) -> TestServer {
    let (_, bot_status) = watch::channel(BotStatus::Disabled);
    TestServer::new(app(AppState::new(
        store,
        "https://q.trap.jp".parse().unwrap(),
        "@BOT_lachite".into(),
        bot_status,
    )))
}

#[tokio::test]
async fn command_catalog_bot_status_and_invalid_cursor() {
    let db = MySqlPoolOptions::new()
        .connect_lazy("mysql://rachite:unused@127.0.0.1/rachite_test")
        .unwrap();
    let server = make_server(SenryuStore::new(db));

    let response = server.get("/api/commands").await;
    response.assert_status_ok();
    let catalog: Value = response.json();
    assert_eq!(catalog["mention"], "@BOT_lachite");
    assert!(
        catalog["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|command| command["usage"] == "stampbomb <stamp> <number> <message-link>")
    );

    server
        .get("/api/bot/status")
        .await
        .assert_json(&json!({"status": "disabled"}));

    server
        .get("/api/senryu?before=invalid")
        .await
        .assert_status_bad_request();

    let response = server.get("/api/openapi.json").await;
    response.assert_status_ok();
    let document: Value = response.json();
    assert!(document["paths"]["/api/senryu"]["get"].is_object());
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL for a dedicated MariaDB database"]
async fn senryu_lifecycle_and_pagination() {
    let url = std::env::var("TEST_DATABASE_URL").expect("Set TEST_DATABASE_URL to a test database");
    let db = MySqlPoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    let store = SenryuStore::new(db);
    let server = make_server(store.clone());
    server.get("/api/health").await.assert_status_ok();

    let channel = Uuid::now_v7();
    let original = Uuid::now_v7();
    let discovered_in_edit = Uuid::now_v7();
    let disabled_new = Uuid::now_v7();

    assert!(store.is_enabled(channel).await.unwrap());
    assert!(store.record(original, channel, "初稿").await.unwrap());
    assert!(!store.record(original, channel, "重複").await.unwrap());
    store
        .revise(discovered_in_edit, channel, Some("編集で検出"), true)
        .await
        .unwrap();
    store.set_enabled(channel, false).await.unwrap();
    assert!(!store.is_enabled(channel).await.unwrap());
    store
        .revise(disabled_new, channel, Some("登録しない"), false)
        .await
        .unwrap();
    store
        .revise(original, channel, Some("改稿"), false)
        .await
        .unwrap();

    let first = server.get("/api/senryu").await.json::<Value>();
    let entries = first["items"].as_array().unwrap();
    let original_entry = entries
        .iter()
        .find(|row| row["message_id"] == original.to_string())
        .unwrap();
    assert_eq!(original_entry["verse"], "改稿");
    assert_eq!(
        original_entry["source_url"],
        format!("https://q.trap.jp/messages/{original}")
    );
    assert!(
        !entries
            .iter()
            .any(|row| row["message_id"] == disabled_new.to_string())
    );
    assert!(
        entries
            .iter()
            .any(|row| row["message_id"] == discovered_in_edit.to_string())
    );

    let mut newest = Vec::new();
    for _ in 0..20 {
        let id = Uuid::now_v7();
        store.record(id, channel, "ページ分割").await.unwrap();
        newest.push(id);
    }
    let first = server.get("/api/senryu").await.json::<Value>();
    assert_eq!(first["items"].as_array().unwrap().len(), 20);
    let cursor = first["next_cursor"].as_str().unwrap();
    let boundary = first["items"][19]["message_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    store.remove(boundary).await.unwrap();
    let second = server
        .get(&format!("/api/senryu?before={cursor}"))
        .await
        .json::<Value>();
    assert_eq!(
        second["items"][0]["message_id"],
        discovered_in_edit.to_string()
    );

    store.revise(original, channel, None, false).await.unwrap();
    store.remove(discovered_in_edit).await.unwrap();
    let list = store.list(None, 100).await.unwrap();
    assert!(
        !list.iter().any(|row| row.message_id == original.to_string()
            || row.message_id == discovered_in_edit.to_string())
    );

    for id in newest {
        store.remove(id).await.unwrap();
    }
}
