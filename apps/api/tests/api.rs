use axum_test::TestServer;
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use starter_api::{ApiDoc, AppState, app};
use utoipa::OpenApi;

async fn server() -> TestServer {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    let state = AppState {
        db,
        jwt_secret: "test-only-secret-with-at-least-32-bytes".into(),
        http: reqwest::Client::new(),
    };
    TestServer::new(app(state, "http://localhost:5173").unwrap())
}
async fn register(server: &TestServer, email: &str) -> String {
    let response = server
        .post("/api/auth/register")
        .json(&json!({"email":email,"password":"a-long-test-password"}))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);
    response.json::<Value>()["token"]
        .as_str()
        .unwrap()
        .to_owned()
}
#[tokio::test]
async fn health_and_openapi() {
    let server = server().await;
    server
        .get("/api/health")
        .await
        .assert_json(&json!({"status":"ok"}));
    let doc = ApiDoc::openapi();
    assert_eq!(doc.paths.paths.len(), 6);
    let response = server.get("/api/openapi.json").await;
    response.assert_status_ok();
    assert!(response.json::<Value>()["components"]["securitySchemes"]["bearer"].is_object());
}
#[tokio::test]
async fn authentication_validation_and_expired_token() {
    let server = server().await;
    server.get("/api/items").await.assert_status_unauthorized();
    server
        .post("/api/auth/register")
        .json(&json!({"email":"bad","password":"short"}))
        .await
        .assert_status_bad_request();
    let token = register(&server, "user@example.com").await;
    server
        .get("/api/auth/me")
        .add_header("authorization", format!("Bearer {token}"))
        .await
        .assert_status_ok();
    server
        .post("/api/auth/register")
        .json(&json!({"email":"USER@example.com","password":"a-long-test-password"}))
        .await
        .assert_status_conflict();
    server
        .post("/api/auth/login")
        .json(&json!({"email":"user@example.com","password":"wrong-password"}))
        .await
        .assert_status_unauthorized();
    server
        .post("/api/auth/login")
        .json(&json!({"email":"USER@example.com","password":"a-long-test-password"}))
        .await
        .assert_status_ok();
    let expired = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &json!({"sub":"x","exp":1,"iss":"starter-api"}),
        &jsonwebtoken::EncodingKey::from_secret(b"test-only-secret-with-at-least-32-bytes")
            .unwrap(),
    )
    .unwrap();
    server
        .get("/api/items")
        .add_header("authorization", format!("Bearer {expired}"))
        .await
        .assert_status_unauthorized();
}
#[tokio::test]
async fn crud_is_scoped_to_the_authenticated_owner() {
    let server = server().await;
    let alice = format!("Bearer {}", register(&server, "alice@example.com").await);
    let bob = format!("Bearer {}", register(&server, "bob@example.com").await);
    server
        .post("/api/items")
        .add_header("authorization", &alice)
        .json(&json!({"title":"   "}))
        .await
        .assert_status_bad_request();
    let response = server
        .post("/api/items")
        .add_header("authorization", &alice)
        .json(&json!({"title":" First item ","description":"Hello"}))
        .await;
    response.assert_status(axum::http::StatusCode::CREATED);
    let item = response.json::<Value>();
    assert_eq!(item["title"], "First item");
    let path = format!("/api/items/{}", item["id"].as_str().unwrap());
    server
        .get(&path)
        .add_header("authorization", &alice)
        .await
        .assert_status_ok();
    server
        .get(&path)
        .add_header("authorization", &bob)
        .await
        .assert_status_not_found();
    server
        .put(&path)
        .add_header("authorization", &bob)
        .json(&json!({"title":"stolen"}))
        .await
        .assert_status_not_found();
    server
        .delete(&path)
        .add_header("authorization", &bob)
        .await
        .assert_status_not_found();
    server
        .get("/api/items")
        .add_header("authorization", &bob)
        .await
        .assert_json(&json!([]));
    let response = server
        .put(&path)
        .add_header("authorization", &alice)
        .json(&json!({"title":"Updated","completed":true}))
        .await;
    response.assert_status_ok();
    assert_eq!(response.json::<Value>()["completed"], true);
    assert_eq!(
        server
            .get("/api/items")
            .add_header("authorization", &alice)
            .await
            .json::<Vec<Value>>()
            .len(),
        1
    );
    server
        .delete(&path)
        .add_header("authorization", &alice)
        .await
        .assert_status(axum::http::StatusCode::NO_CONTENT);
    server
        .get(&path)
        .add_header("authorization", &alice)
        .await
        .assert_status_not_found();
}
