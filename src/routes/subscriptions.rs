use axum::{extract::State, Form};
use chrono::Utc;
use hyper::StatusCode;
use sqlx::PgPool;
use tracing::Instrument;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

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
    
    let new_subscriber = NewSubscriber {
        email: form.email,
        name: SubscriberName::parse(form.name).map_err(|e| StatusCode::BAD_REQUEST)?,
    };

    insert_subscriber(&connection_pool, &new_subscriber)
        .await
        .map_err(|e| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(connection_pool, new_subscriber)
)]
pub async fn insert_subscriber(
    connection_pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        new_subscriber.email,
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(connection_pool)
    .await
    .map(|_| {
        tracing::info!("New subscriber {} saved", new_subscriber.email);
    })
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })
}
