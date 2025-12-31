use serde_json::Value;
use uuid::Uuid;
use sea_orm::{ActiveModelTrait, Set};
use sea_orm::ActiveValue::NotSet;

use crate::{error::AppResult, state::AppState, entity::audit_logs};

pub async fn log_audit(
    state: &AppState,
    user_id: Option<Uuid>,
    action: &str,
    resource: Option<&str>,
    metadata: Option<Value>,
) -> AppResult<()> {
    let active = audit_logs::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        action: Set(action.to_string()),
        resource: Set(resource.map(|r| r.to_string())),
        metadata: Set(metadata),
        created_at: NotSet,
    };

    active.insert(&state.orm).await?;

    Ok(())
}
