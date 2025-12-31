use argon2::{
    Argon2, PasswordHasher,
    password_hash::{PasswordHash, PasswordVerifier, SaltString},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use password_hash::rand_core::OsRng;
use uuid::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use sea_orm::ActiveValue::NotSet;

use crate::{
    audit::log_audit,
    error::{AppError, AppResult},
    models::User,
    response::{ApiResponse, Meta},
    state::AppState,
    entity::users::{ActiveModel as UserActive, Column as UserCol, Entity as Users, Model as UserModel},
};
use crate::dto::auth::{Claims, LoginRequest, LoginResponse, RegisterRequest};

pub async fn register_user(
    state: &AppState,
    payload: RegisterRequest,
) -> AppResult<ApiResponse<User>> {
    let RegisterRequest { email, password } = payload;
    let exist = Users::find()
        .filter(UserCol::Email.eq(email.clone()))
        .one(&state.orm)
        .await?;

    if exist.is_some() {
        return Err(AppError::BadRequest("Email is already taken".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?
        .to_string();

    let active = UserActive {
        id: Set(Uuid::new_v4()),
        email: Set(email),
        password_hash: Set(password_hash),
        role: Set("user".into()),
        created_at: NotSet,
    };
    let user = active.insert(&state.orm).await?;

    if let Err(err) = log_audit(
        state,
        Some(user.id),
        "user_register",
        Some("users"),
        Some(serde_json::json!({ "user_id": user.id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }
    Ok(ApiResponse::success("User created", user_from_entity(user), None))
}

pub async fn login_user(
    state: &AppState,
    payload: LoginRequest,
) -> AppResult<ApiResponse<LoginResponse>> {
    let LoginRequest { email, password } = payload;
    let user = Users::find()
        .filter(UserCol::Email.eq(email.clone()))
        .one(&state.orm)
        .await?
        .map(user_from_entity);

    let user = match user {
        Some(u) => u,
        None => return Err(AppError::BadRequest("Invalid email or password".into())),
    };

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid password hash")))?;

    let argon2 = Argon2::default();
    if argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(AppError::BadRequest("Invalid email or password".into()));
    }

    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("JWT_SECRET is not set")))?;

    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to set expiration")))?;

    let claims = Claims {
        sub: user.id.to_string(),
        role: user.role.clone(),
        exp: expiration.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    let resp = LoginResponse {
        token: format!("Bearer {}", token),
    };

    if let Err(err) = log_audit(
        state,
        Some(user.id),
        "user_login",
        Some("users"),
        Some(serde_json::json!({ "user_id": user.id })),
    )
    .await
    {
        tracing::warn!(error = %err, "audit log failed");
    }

    Ok(ApiResponse::success(
        "Logged in",
        resp,
        Some(Meta::empty()),
    ))
}

fn user_from_entity(model: UserModel) -> User {
    User {
        id: model.id,
        email: model.email,
        password_hash: model.password_hash,
        created_at: model.created_at.with_timezone(&Utc),
        role: model.role,
    }
}
