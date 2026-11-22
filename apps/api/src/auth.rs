use crate::{AppState, error::ApiError};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}
#[derive(Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct User {
    pub id: String,
    pub email: String,
    pub created_at: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct Session {
    pub token: String,
    pub user: User,
}
#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iss: String,
}
pub struct AuthUser(pub String);
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(ApiError::Unauthorized)?;
        let mut validation = Validation::default();
        validation.set_issuer(&["starter-api"]);
        let key =
            DecodingKey::from_secret(state.jwt_secret.as_bytes()).map_err(anyhow::Error::from)?;
        let claims = decode::<Claims>(token, &key, &validation)
            .map_err(|_| ApiError::Unauthorized)?
            .claims;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)")
            .bind(&claims.sub)
            .fetch_one(&state.db)
            .await?;
        if !exists {
            return Err(ApiError::Unauthorized);
        }
        Ok(Self(claims.sub))
    }
}
fn session(user: User, secret: &str) -> Result<Session, ApiError> {
    let claims = Claims {
        sub: user.id.clone(),
        exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iss: "starter-api".into(),
    };
    let key = EncodingKey::from_secret(secret.as_bytes()).map_err(anyhow::Error::from)?;
    let token = encode(&Header::default(), &claims, &key).map_err(anyhow::Error::from)?;
    Ok(Session { token, user })
}
fn normalize(input: &Credentials) -> Result<String, ApiError> {
    let email = input.email.trim().to_lowercase();
    let valid_email = email.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
    });
    if email.len() > 254 || email.chars().any(char::is_whitespace) || !valid_email {
        return Err(ApiError::Validation(
            "有効なメールアドレスを入力してください".into(),
        ));
    }
    if input.password.len() < 12 || input.password.len() > 128 {
        return Err(ApiError::Validation(
            "パスワードは12〜128バイトで入力してください".into(),
        ));
    }
    Ok(email)
}
#[utoipa::path(post, path="/api/auth/register", request_body=Credentials, responses((status=201, body=Session), (status=400, body=crate::error::ErrorBody), (status=409, body=crate::error::ErrorBody)), tag="auth")]
pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<Credentials>,
) -> Result<(StatusCode, Json<Session>), ApiError> {
    let email = normalize(&input)?;
    let password = input.password;
    let hash = tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map(|h| h.to_string())
            .map_err(|e| anyhow::anyhow!("{e}"))
    })
    .await
    .map_err(anyhow::Error::from)??;
    let user = User {
        id: Uuid::now_v7().to_string(),
        email,
        created_at: Utc::now().to_rfc3339(),
    };
    let result =
        sqlx::query("INSERT INTO users(id,email,password_hash,created_at) VALUES(?,?,?,?)")
            .bind(&user.id)
            .bind(&user.email)
            .bind(hash)
            .bind(&user.created_at)
            .execute(&state.db)
            .await;
    match result {
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => return Err(ApiError::Conflict),
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }
    Ok((StatusCode::CREATED, Json(session(user, &state.jwt_secret)?)))
}
#[utoipa::path(post, path="/api/auth/login", request_body=Credentials, responses((status=200, body=Session), (status=401, body=crate::error::ErrorBody)), tag="auth")]
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<Credentials>,
) -> Result<Json<Session>, ApiError> {
    let email = normalize(&input)?;
    let record: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT id,email,created_at,password_hash FROM users WHERE email=?")
            .bind(email)
            .fetch_optional(&state.db)
            .await?;
    // Run Argon2 even for unknown accounts to avoid a cheap account-existence timing signal.
    let password = input.password;
    let hash = record.as_ref().map(|r| r.3.clone());
    let valid = tokio::task::spawn_blocking(move || {
        if let Some(hash) = hash {
            PasswordHash::new(&hash).is_ok_and(|h| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &h)
                    .is_ok()
            })
        } else {
            let _ = Argon2::default()
                .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng));
            false
        }
    })
    .await
    .map_err(anyhow::Error::from)?;
    if !valid {
        return Err(ApiError::Unauthorized);
    }
    let (id, email, created_at, _) = record.ok_or(ApiError::Unauthorized)?;
    Ok(Json(session(
        User {
            id,
            email,
            created_at,
        },
        &state.jwt_secret,
    )?))
}
#[utoipa::path(get, path="/api/auth/me", security(("bearer"=[])), responses((status=200, body=User), (status=401, body=crate::error::ErrorBody)), tag="auth")]
pub async fn me(
    State(state): State<AppState>,
    AuthUser(id): AuthUser,
) -> Result<Json<User>, ApiError> {
    Ok(Json(
        sqlx::query_as("SELECT id,email,created_at FROM users WHERE id=?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(ApiError::Unauthorized)?,
    ))
}
