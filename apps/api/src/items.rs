use crate::{AppState, auth::AuthUser, error::ApiError};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ItemInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub completed: bool,
}
impl ItemInput {
    fn validate(&mut self) -> Result<(), ApiError> {
        self.title = self.title.trim().into();
        if self.title.is_empty() || self.title.chars().count() > 200 {
            return Err(ApiError::Validation(
                "タイトルは1〜200文字で入力してください".into(),
            ));
        }
        if self.description.chars().count() > 5000 {
            return Err(ApiError::Validation(
                "説明は5000文字以内で入力してください".into(),
            ));
        }
        Ok(())
    }
}
#[utoipa::path(get, path="/api/items", security(("bearer"=[])), responses((status=200, body=Vec<Item>), (status=401, body=crate::error::ErrorBody)), tag="items")]
pub async fn list(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
) -> Result<Json<Vec<Item>>, ApiError> {
    Ok(Json(sqlx::query_as("SELECT id,title,description,completed,created_at,updated_at FROM items WHERE owner_id=? ORDER BY created_at DESC").bind(owner).fetch_all(&state.db).await?))
}
#[utoipa::path(post, path="/api/items", request_body=ItemInput, security(("bearer"=[])), responses((status=201, body=Item), (status=400, body=crate::error::ErrorBody), (status=401, body=crate::error::ErrorBody)), tag="items")]
pub async fn create(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Json(mut input): Json<ItemInput>,
) -> Result<(StatusCode, Json<Item>), ApiError> {
    input.validate()?;
    let now = Utc::now().to_rfc3339();
    let item = Item {
        id: Uuid::now_v7().to_string(),
        title: input.title,
        description: input.description,
        completed: input.completed,
        created_at: now.clone(),
        updated_at: now,
    };
    sqlx::query("INSERT INTO items(id,owner_id,title,description,completed,created_at,updated_at) VALUES(?,?,?,?,?,?,?)").bind(&item.id).bind(owner).bind(&item.title).bind(&item.description).bind(item.completed).bind(&item.created_at).bind(&item.updated_at).execute(&state.db).await?;
    Ok((StatusCode::CREATED, Json(item)))
}
#[utoipa::path(get, path="/api/items/{id}", params(("id"=String, Path)), security(("bearer"=[])), responses((status=200, body=Item), (status=404, body=crate::error::ErrorBody)), tag="items")]
pub async fn get_one(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Item>, ApiError> {
    Ok(Json(sqlx::query_as("SELECT id,title,description,completed,created_at,updated_at FROM items WHERE id=? AND owner_id=?").bind(id).bind(owner).fetch_optional(&state.db).await?.ok_or(ApiError::NotFound)?))
}
#[utoipa::path(put, path="/api/items/{id}", params(("id"=String, Path)), request_body=ItemInput, security(("bearer"=[])), responses((status=200, body=Item), (status=400, body=crate::error::ErrorBody), (status=404, body=crate::error::ErrorBody)), tag="items")]
pub async fn update(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<String>,
    Json(mut input): Json<ItemInput>,
) -> Result<Json<Item>, ApiError> {
    input.validate()?;
    let item = sqlx::query_as("UPDATE items SET title=?,description=?,completed=?,updated_at=? WHERE id=? AND owner_id=? RETURNING id,title,description,completed,created_at,updated_at")
 .bind(input.title).bind(input.description).bind(input.completed).bind(Utc::now().to_rfc3339()).bind(id).bind(owner).fetch_optional(&state.db).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(item))
}
#[utoipa::path(delete, path="/api/items/{id}", params(("id"=String, Path)), security(("bearer"=[])), responses((status=204), (status=404, body=crate::error::ErrorBody)), tag="items")]
pub async fn delete(
    State(state): State<AppState>,
    AuthUser(owner): AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM items WHERE id=? AND owner_id=?")
        .bind(id)
        .bind(owner)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
