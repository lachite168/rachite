use anyhow::Context;
use rachite_api::{
    AppState, app,
    bot::{self, BotStatus},
    config::Config,
    senryu::SenryuStore,
};
use sqlx::mysql::MySqlPoolOptions;
use tokio::{net::TcpListener, sync::watch};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::load()?;
    let db = MySqlPoolOptions::new()
        .max_connections(5)
        .connect_with(config.database)
        .await
        .context("Could not connect to MariaDB")?;
    sqlx::migrate!().run(&db).await?;

    let listener = TcpListener::bind((config.host.as_str(), config.port)).await?;
    let store = SenryuStore::new(db);
    let initial_status = if config.bot.is_some() {
        BotStatus::Disconnected
    } else {
        BotStatus::Disabled
    };
    let (bot_status, status) = watch::channel(initial_status);
    let state = AppState::new(
        store.clone(),
        config.traq_origin.clone(),
        config.command_mention.clone(),
        status,
    );
    let bot_task = config
        .bot
        .map(|bot_config| {
            bot::start(
                bot_config,
                config.traq_origin,
                config.command_mention,
                store,
                bot_status,
            )
        })
        .transpose()?;

    tracing::info!(address = %listener.local_addr()?, "API ready");
    let result = axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown())
        .await;

    if let Some(task) = bot_task {
        task.abort();
        let _ = task.await;
    }
    result?;
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
