use serde_json::Value;
use uuid::Uuid;

use crate::{db::DbPool, error::AppResult};

pub async fn log_audit(
    pool: &DbPool,
    user_id: Option<Uuid>,
    action: &str,
    resource: Option<&str>,
    metadata: Option<Value>,
) -> AppResult<()> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO audit_logs (id, user_id, action, resource, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(action)
    .bind(resource)
    .bind(metadata)
    .execute(pool)
    .await?;

    Ok(())
}
