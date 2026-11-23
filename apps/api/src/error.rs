use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("一覧のページ指定が無効です")]
    InvalidCursor,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidCursor => (StatusCode::BAD_REQUEST, "invalid_cursor", self.to_string()),
            Self::Database(error) => {
                tracing::error!(%error, "Database operation failed");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "database_unavailable",
                    "データベースに接続できません".into(),
                )
            }
        };
        (status, Json(ErrorBody { code, message })).into_response()
    }
}
