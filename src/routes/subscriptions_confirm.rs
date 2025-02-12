use axum::extract::{Path, State};
use hyper::StatusCode;
use uuid::Uuid;

use crate::app_state::{self, AppState};

pub async fn confirm_subscription(
    State(app_state): State<AppState>,
    Path(token): Path<String>,
) -> Result<(), StatusCode> {
    let id = match get_subscriber_id_from_token(&app_state, &token).await {
        Ok(id) => id,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    match id {
        // Non-existing token!
        None => return Err(StatusCode::UNAUTHORIZED),
        Some(subscriber_id) => {
            if confirm_subscriber(&app_state, subscriber_id).await.is_err() {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Ok(())
        }
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, app_state))]
pub async fn confirm_subscriber(app_state: &AppState, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(&app_state.connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(token, app_state))]
pub async fn get_subscriber_id_from_token(
    app_state: &AppState,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE token = $1"#,
        token,
    )
    .fetch_optional(&app_state.connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}
