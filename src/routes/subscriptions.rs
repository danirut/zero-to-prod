use axum::{extract::State, Form};
use chrono::Utc;
use hyper::StatusCode;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(connection_pool, form),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    State(connection_pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> Result<(), StatusCode> {
    tracing::info!("Saving new subscriber details in the database");
    insert_subscriber(&connection_pool, &form)
        .await
        .map_err(|e| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(connection_pool, form)
)]
pub async fn insert_subscriber(
    connection_pool: &PgPool,
    form: &FormData,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(connection_pool)
    .await
    .map(|_| {
        tracing::info!("New subscriber {} saved", form.email);
    })
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })
}
