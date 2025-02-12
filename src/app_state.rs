use crate::email_client;

use sqlx::PgPool;
use email_client::EmailClient;

#[derive(Clone)]
pub struct AppState {
    pub connection_pool: PgPool,
    pub email_client: EmailClient,
    pub host: String,
}