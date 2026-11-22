pub mod auth;
pub mod error;
pub mod items;
use axum::{
    Json, Router,
    http::{HeaderValue, Method},
    routing::{get, post},
};
use serde::Serialize;
use sqlx::SqlitePool;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub jwt_secret: Arc<str>,
    pub http: reqwest::Client,
}
#[derive(Serialize, ToSchema)]
pub struct Health {
    status: String,
}
#[utoipa::path(get, path = "/api/health", responses((status = 200, body = Health)), tag = "system")]
async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<Json<Health>, error::ApiError> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok(Json(Health {
        status: "ok".into(),
    }))
}
#[derive(OpenApi)]
#[openapi(paths(health, auth::register, auth::login, auth::me, items::list, items::create, items::get_one, items::update, items::delete),
 components(schemas(Health, error::ErrorBody, auth::Credentials, auth::Session, auth::User, items::Item, items::ItemInput)), modifiers(&Security))]
pub struct ApiDoc;
struct Security;
impl utoipa::Modify for Security {
    fn modify(&self, doc: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = doc.components.as_mut() {
            components.add_security_scheme(
                "bearer",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            );
        }
    }
}
pub fn app(state: AppState, origin: &str) -> anyhow::Result<Router> {
    Ok(Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/me", get(auth::me))
        .route("/api/items", get(items::list).post(items::create))
        .route(
            "/api/items/{id}",
            get(items::get_one).put(items::update).delete(items::delete),
        )
        .merge(SwaggerUi::new("/swagger-ui").url("/api/openapi.json", ApiDoc::openapi()))
        .layer(
            CorsLayer::new()
                .allow_origin(origin.parse::<HeaderValue>()?)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ]),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}
