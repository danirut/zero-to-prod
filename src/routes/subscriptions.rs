use std::ops::DerefMut;

use axum::{extract::State, Form};
use chrono::Utc;
use hyper::StatusCode;
use rand::{distr::Alphanumeric, rng, Rng};
use sqlx::{Acquire, Database, Executor, Transaction};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
};
use eyre::Result;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = eyre::Report;

    fn try_from(form: FormData) -> std::result::Result<Self, Self::Error> {
        Ok(NewSubscriber {
            email: SubscriberEmail::parse(form.email)?,
            name: SubscriberName::parse(form.name)?,
        })
    }
}

fn generate_subscription_token() -> String {
    let mut rng = rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
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

    let mut transaction:Transaction<'_, _> =
        match app_state.connection_pool.begin().await {
            Ok(transaction) => transaction,
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        };

    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .map_err(|e| {
            tracing::error!("Faield to insert subscription token {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if transaction.commit().await.is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    send_confirmation_email(
        &app_state.host,
        &app_state.email_client,
        &new_subscriber.email,
        &subscription_token,
    )
    .await
    .map_err(|e| {
        tracing::error!("Faield to send confirmation email {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(())
}

pub async fn store_token(
    transaction: &mut Transaction<'_, sqlx::Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (token, subscriber_id)
    VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, sqlx::Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending')
    "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(&mut **transaction)
    .await
    .map(|_| {
        tracing::info!("New subscriber {} saved", new_subscriber.email.as_ref());
        subscriber_id
    })
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })
}
#[tracing::instrument(
    name = "Sending confirmation email",
    skip(host, email_client, email, subscription_token)
)]
pub async fn send_confirmation_email(
    host: &str,
    email_client: &EmailClient,
    email: &SubscriberEmail,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "http://{}/subscriptions/confirm/{}",
        host, subscription_token
    );

    email_client
        .send_email(
            email.clone(),
            "Welcome!",
            &format!(
                "Welcome to my newsletter!<br />Click <a href={}>here</a> to confirm subscription.",
                confirmation_link
            ),
            &format!(
                "Welcome to my newsletter. Click {} to confirm subscrition.",
                confirmation_link
            ),
        )
        .await
}
