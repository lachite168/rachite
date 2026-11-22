use anyhow::Context;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use starter_api::{AppState, app};
use std::{str::FromStr, time::Duration};
use tracing_subscriber::EnvFilter;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let secret = std::env::var("JWT_SECRET").context("JWT_SECRET is required; see .env.example")?;
    anyhow::ensure!(
        secret.len() >= 32 && !secret.starts_with("replace-with-"),
        "Set a random JWT_SECRET with at least 32 characters"
    );
    let options = SqliteConnectOptions::from_str(
        &std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://app.db".into()),
    )?
    .create_if_missing(true)
    .foreign_keys(true)
    .journal_mode(SqliteJournalMode::Wal)
    .busy_timeout(Duration::from_secs(5));
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::migrate!().run(&db).await?;
    let origin = std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".into());
    let state = AppState {
        db,
        jwt_secret: secret.into(),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?,
    };
    let address = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into()),
        std::env::var("PORT").unwrap_or_else(|_| "3000".into())
    );
    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!(%address, "API ready");
    axum::serve(listener, app(state, &origin)?)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}
async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}
