use axum::http::StatusCode;
use axum_test::TestServer;
use rachite_api::{AppState, app, bot::BotStatus, verse::VerseStore};
use serde_json::{Value, json};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use tokio::sync::{OnceCell, watch};
use uuid::Uuid;

fn make_server(store: VerseStore) -> TestServer {
    let (_, bot_status) = watch::channel(BotStatus::Disabled);
    TestServer::new(app(AppState::new(
        store,
        "https://q.trap.jp".parse().unwrap(),
        "@BOT_lachite".into(),
        bot_status,
    )))
}

async fn test_database() -> MySqlPool {
    static MIGRATED: OnceCell<()> = OnceCell::const_new();
    let url = std::env::var("TEST_DATABASE_URL").expect("Set TEST_DATABASE_URL to a test database");
    let db = MySqlPoolOptions::new().connect(&url).await.unwrap();
    MIGRATED
        .get_or_init(|| async { sqlx::migrate!().run(&db).await.unwrap() })
        .await;
    db
}

#[tokio::test]
async fn command_catalog_bot_status_and_invalid_cursor() {
    let db = MySqlPoolOptions::new()
        .connect_lazy("mysql://rachite:unused@127.0.0.1/rachite_test")
        .unwrap();
    let server = make_server(VerseStore::new(db));

    let response = server.get("/commands").await;
    response.assert_status_ok();
    let catalog: Value = response.json();
    assert_eq!(catalog["mention"], "@BOT_lachite");
    for usage in [
        "stampbomb <stamp> <number> <message-link>",
        "markov <username> [n] [--here]",
    ] {
        assert!(
            catalog["commands"]
                .as_array()
                .unwrap()
                .iter()
                .any(|command| command["usage"] == usage)
        );
    }

    server
        .get("/bot/status")
        .await
        .assert_json(&json!({"status": "disabled"}));

    server
        .get("/senryu?before=invalid")
        .await
        .assert_status_bad_request();
    let invalid_cursor = server.get("/mogamigawa?before=legacy.invalid").await;
    invalid_cursor.assert_status_bad_request();
    invalid_cursor.assert_json(&json!({
        "code": "invalid_cursor",
        "message": "一覧のページ指定が無効です"
    }));

    let response = server.get("/openapi.json").await;
    response.assert_status_ok();
    let document: Value = response.json();
    assert!(document["paths"]["/api/senryu"]["get"].is_object());
    assert!(document["paths"]["/api/mogamigawa"]["get"].is_object());

    let response = server.get("/swagger-ui/swagger-initializer.js").await;
    response.assert_status_ok();
    assert!(response.text().contains("/api/openapi.json"));
}

#[tokio::test]
async fn database_backed_routes_report_service_unavailable() {
    let db = MySqlPoolOptions::new()
        .connect_lazy("mysql://rachite:unused@127.0.0.1/rachite_test")
        .unwrap();
    db.close().await;
    let server = make_server(VerseStore::new(db));

    for path in ["/health", "/senryu", "/mogamigawa"] {
        let response = server.get(path).await;
        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
        response.assert_json(&json!({
            "code": "database_unavailable",
            "message": "データベースに接続できません"
        }));
    }
    server.get("/commands").await.assert_status_ok();
    server.get("/bot/status").await.assert_status_ok();
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL for a dedicated MariaDB database"]
async fn senryu_lifecycle_and_pagination() {
    let db = test_database().await;
    let store = VerseStore::new(db.clone());
    let server = make_server(store.clone());
    server.get("/health").await.assert_status_ok();

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

    let first = server.get("/senryu").await.json::<Value>();
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
    for _ in 0..21 {
        let id = Uuid::now_v7();
        store.record(id, channel, "ページ分割").await.unwrap();
        newest.push(id);
    }
    sqlx::query(
        "UPDATE senryu
         SET detected_at = CASE message_id
             WHEN ? THEN '2026-11-23 01:02:03.123454'
             WHEN ? THEN '2026-11-23 01:02:03.123455'
             ELSE '2026-11-23 01:02:03.123456'
         END
         WHERE channel_id = ?",
    )
    .bind(original.to_string())
    .bind(discovered_in_edit.to_string())
    .bind(channel.to_string())
    .execute(&db)
    .await
    .unwrap();
    newest.sort_unstable_by(|a, b| b.cmp(a));

    let first = server.get("/senryu").await.json::<Value>();
    let first_ids: Vec<_> = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["message_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        first_ids,
        newest[..20].iter().map(Uuid::to_string).collect::<Vec<_>>()
    );
    let cursor = first["next_cursor"].as_str().unwrap();
    store.remove(newest[19]).await.unwrap();
    let second = server
        .get(&format!("/senryu?before={cursor}"))
        .await
        .json::<Value>();
    let second_ids: Vec<_> = second["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["message_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        second_ids,
        [newest[20], discovered_in_edit, original].map(|id| id.to_string())
    );
    assert_eq!(second["next_cursor"], Value::Null);

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

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL for a dedicated MariaDB database"]
async fn mogamigawa_paginates_dated_verses_before_legacy_links() {
    let db = test_database().await;
    let server = make_server(VerseStore::new(db.clone()));
    let mut dated_ids: Vec<_> = (0..20).map(|_| Uuid::now_v7()).collect();
    let mut legacy_ids: Vec<_> = (0..21).map(|_| Uuid::now_v7()).collect();
    dated_ids.sort_unstable_by(|a, b| b.cmp(a));
    legacy_ids.sort_unstable_by(|a, b| b.cmp(a));
    let channel = Uuid::now_v7();
    let verse = "古池や\n蛙飛びこむ\n最上川";
    for id in &dated_ids {
        sqlx::query(
            "INSERT INTO mogamigawa_responses (message_id, channel_id, verse, detected_at)
             VALUES (?, ?, ?, '2026-11-23 01:02:03')",
        )
        .bind(id.to_string())
        .bind(channel.to_string())
        .bind(verse)
        .execute(&db)
        .await
        .unwrap();
    }
    for id in &legacy_ids {
        sqlx::query("INSERT INTO mogamigawa_responses (message_id) VALUES (?)")
            .bind(id.to_string())
            .execute(&db)
            .await
            .unwrap();
    }

    let first = server.get("/mogamigawa").await.json::<Value>();
    let first_items = first["items"].as_array().unwrap();
    assert_eq!(first_items.len(), 20);
    assert_eq!(first_items[0]["message_id"], dated_ids[0].to_string());
    assert_eq!(first_items[19]["message_id"], dated_ids[19].to_string());
    assert_eq!(first_items[0]["verse"], verse);
    assert_eq!(first_items[0]["channel_id"], channel.to_string());
    assert_eq!(first_items[0]["detected_at"], "2026-11-23T01:02:03Z");

    let cursor = first["next_cursor"].as_str().unwrap();
    let second = server
        .get(&format!("/mogamigawa?before={cursor}"))
        .await
        .json::<Value>();
    let legacy_items = second["items"].as_array().unwrap();
    assert_eq!(legacy_items.len(), 20);
    assert_eq!(legacy_items[0]["message_id"], legacy_ids[0].to_string());
    assert_eq!(legacy_items[0]["verse"], Value::Null);
    assert_eq!(legacy_items[0]["detected_at"], Value::Null);
    assert_eq!(
        legacy_items[0]["source_url"],
        format!("https://q.trap.jp/messages/{}", legacy_ids[0])
    );

    let cursor = second["next_cursor"].as_str().unwrap();
    let third = server
        .get(&format!("/mogamigawa?before={cursor}"))
        .await
        .json::<Value>();
    assert_eq!(third["items"].as_array().unwrap().len(), 1);
    assert_eq!(third["items"][0]["message_id"], legacy_ids[20].to_string());
    assert_eq!(third["next_cursor"], Value::Null);

    for id in dated_ids.into_iter().chain(legacy_ids) {
        sqlx::query("DELETE FROM mogamigawa_responses WHERE message_id = ?")
            .bind(id.to_string())
            .execute(&db)
            .await
            .unwrap();
    }
}
