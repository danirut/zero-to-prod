use axum::{extract::State, Form};
use chrono::Utc;
use hyper::StatusCode;
use uuid::Uuid;

use crate::{app_state::AppState, domain::{NewSubscriber, SubscriberEmail, SubscriberName}};
use eyre::Result;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error=eyre::Report;

    fn try_from(form: FormData) -> std::result::Result<Self, Self::Error> {
        Ok(NewSubscriber {
            email: SubscriberEmail::parse(form.email)?,
            name: SubscriberName::parse(form.name)?,
        })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(app_state, form),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    State(app_state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<(), StatusCode> {
    tracing::info!("Saving new subscriber details in the database");
    let new_subscriber: NewSubscriber = form.try_into().map_err(|_| StatusCode::BAD_REQUEST)?;
    
    insert_subscriber(&app_state, &new_subscriber)
        .await
        .map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(app_state, new_subscriber)
)]
pub async fn insert_subscriber(
    app_state: &AppState,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(&app_state.connection_pool)
    .await
    .map(|_| {
        tracing::info!("New subscriber {} saved", new_subscriber.email.as_ref());
    })
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })
}
