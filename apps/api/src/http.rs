use crate::{
    bot::{BotStatus, Command, command_catalog},
    error::{ApiError, ErrorBody},
    senryu::{MogamigawaCursor, SenryuCursor, SenryuStore},
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
use utoipa_swagger_ui::{Config as SwaggerConfig, SwaggerUi};

const PAGE_SIZE: u32 = 20;

fn next_cursor<T>(entries: &mut Vec<T>, encode: impl Fn(&T) -> String) -> Option<String> {
    if entries.len() > PAGE_SIZE as usize {
        entries.pop();
        entries.last().map(encode)
    } else {
        None
    }
}

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
struct VerseQuery {
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

#[derive(Serialize, ToSchema)]
struct MogamigawaItem {
    message_id: String,
    channel_id: Option<String>,
    verse: Option<String>,
    detected_at: Option<DateTime<Utc>>,
    source_url: String,
}

#[derive(Serialize, ToSchema)]
struct MogamigawaPage {
    items: Vec<MogamigawaItem>,
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

#[utoipa::path(get, path = "/api/senryu", params(VerseQuery), responses((status = 200, body = SenryuPage), (status = 400, body = ErrorBody), (status = 503, body = ErrorBody)))]
async fn list_senryu(
    State(state): State<AppState>,
    Query(query): Query<VerseQuery>,
) -> Result<Json<SenryuPage>, ApiError> {
    let before = match query.before {
        Some(value) => Some(SenryuCursor::decode(&value).ok_or(ApiError::InvalidCursor)?),
        None => None,
    };
    let mut entries = state.store.list(before, PAGE_SIZE + 1).await?;
    let next_cursor = next_cursor(&mut entries, SenryuCursor::encode);
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

#[utoipa::path(get, path = "/api/mogamigawa", params(VerseQuery), responses((status = 200, body = MogamigawaPage), (status = 400, body = ErrorBody), (status = 503, body = ErrorBody)))]
async fn list_mogamigawa(
    State(state): State<AppState>,
    Query(query): Query<VerseQuery>,
) -> Result<Json<MogamigawaPage>, ApiError> {
    let before = match query.before {
        Some(value) => Some(MogamigawaCursor::decode(&value).ok_or(ApiError::InvalidCursor)?),
        None => None,
    };
    let mut entries = state.store.list_mogamigawa(before, PAGE_SIZE + 1).await?;
    let next_cursor = next_cursor(&mut entries, MogamigawaCursor::encode);
    let origin = state.traq_origin.as_str().trim_end_matches('/');
    let items = entries
        .into_iter()
        .map(|entry| MogamigawaItem {
            source_url: format!("{origin}/messages/{}", entry.message_id),
            message_id: entry.message_id,
            channel_id: entry.channel_id,
            verse: entry.verse,
            detected_at: entry.detected_at.map(|date| date.and_utc()),
        })
        .collect();
    Ok(Json(MogamigawaPage { items, next_cursor }))
}

#[derive(OpenApi)]
#[openapi(paths(health, bot_status, commands, list_senryu, list_mogamigawa))]
struct ApiDoc;

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/bot/status", get(bot_status))
        .route("/commands", get(commands))
        .route("/senryu", get(list_senryu))
        .route("/mogamigawa", get(list_mogamigawa))
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/openapi.json", ApiDoc::openapi())
                .config(SwaggerConfig::new(["/api/openapi.json"])),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
