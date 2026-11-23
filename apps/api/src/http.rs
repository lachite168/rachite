use crate::{
    bot::{BotStatus, Command, command_catalog},
    error::{ApiError, ErrorBody},
    senryu::{SenryuCursor, SenryuStore},
};
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tower_http::trace::TraceLayer;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

const PAGE_SIZE: u32 = 20;

#[derive(Clone)]
pub struct AppState {
    store: SenryuStore,
    traq_origin: Url,
    mention: String,
    bot_status: watch::Receiver<BotStatus>,
}

impl AppState {
    pub fn new(
        store: SenryuStore,
        traq_origin: Url,
        mention: String,
        bot_status: watch::Receiver<BotStatus>,
    ) -> Self {
        Self {
            store,
            traq_origin,
            mention,
            bot_status,
        }
    }
}

#[derive(Serialize, ToSchema)]
struct Health {
    status: &'static str,
}

#[derive(Serialize, ToSchema)]
struct BotHealth {
    status: BotStatus,
}

#[derive(Serialize, ToSchema)]
struct CommandCatalog {
    mention: String,
    commands: &'static [Command],
}

#[derive(Deserialize, IntoParams)]
struct SenryuQuery {
    /// Cursor returned by the previous page.
    before: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct SenryuItem {
    message_id: String,
    channel_id: String,
    verse: String,
    detected_at: DateTime<Utc>,
    source_url: String,
}

#[derive(Serialize, ToSchema)]
struct SenryuPage {
    items: Vec<SenryuItem>,
    next_cursor: Option<String>,
}

#[utoipa::path(get, path = "/api/health", responses((status = 200, body = Health), (status = 503, body = ErrorBody)))]
async fn health(State(state): State<AppState>) -> Result<Json<Health>, ApiError> {
    state.store.check_connection().await?;
    Ok(Json(Health { status: "ok" }))
}

#[utoipa::path(get, path = "/api/bot/status", responses((status = 200, body = BotHealth)))]
async fn bot_status(State(state): State<AppState>) -> Json<BotHealth> {
    Json(BotHealth {
        status: *state.bot_status.borrow(),
    })
}

#[utoipa::path(get, path = "/api/commands", responses((status = 200, body = CommandCatalog)))]
async fn commands(State(state): State<AppState>) -> Json<CommandCatalog> {
    Json(CommandCatalog {
        mention: state.mention,
        commands: command_catalog(),
    })
}

#[utoipa::path(get, path = "/api/senryu", params(SenryuQuery), responses((status = 200, body = SenryuPage), (status = 400, body = ErrorBody), (status = 503, body = ErrorBody)))]
async fn list_senryu(
    State(state): State<AppState>,
    Query(query): Query<SenryuQuery>,
) -> Result<Json<SenryuPage>, ApiError> {
    let before = match query.before {
        Some(value) => Some(SenryuCursor::decode(&value).ok_or(ApiError::InvalidCursor)?),
        None => None,
    };
    let mut entries = state.store.list(before, PAGE_SIZE + 1).await?;
    let next_cursor = if entries.len() > PAGE_SIZE as usize {
        entries.pop();
        entries.last().map(SenryuCursor::encode)
    } else {
        None
    };
    let origin = state.traq_origin.as_str().trim_end_matches('/');
    let items = entries
        .into_iter()
        .map(|entry| SenryuItem {
            source_url: format!("{origin}/messages/{}", entry.message_id),
            message_id: entry.message_id,
            channel_id: entry.channel_id,
            verse: entry.verse,
            detected_at: entry.detected_at.and_utc(),
        })
        .collect();
    Ok(Json(SenryuPage { items, next_cursor }))
}

#[derive(OpenApi)]
#[openapi(paths(health, bot_status, commands, list_senryu))]
struct ApiDoc;

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/bot/status", get(bot_status))
        .route("/api/commands", get(commands))
        .route("/api/senryu", get(list_senryu))
        .merge(SwaggerUi::new("/swagger-ui").url("/api/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
