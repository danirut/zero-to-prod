use axum::{
    http::HeaderName,
    routing::{get, post},
    Router,
};
use eyre::Result;
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{
        MakeRequestUuid,
        SetRequestIdLayer,
        PropagateRequestIdLayer
    },
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use crate::{app_state::AppState, configuration::Settings, email_client::EmailClient, routes};


pub fn build(configuration: Settings) -> Result<AppState> {
    let timeout = configuration.email_client.timeout();
    let connection_pool = PgPool::connect_lazy_with(configuration.database.with_db());
    let email_client = EmailClient::new(
        configuration.email_client.base_url.clone(), 
        configuration.email_client.sender()?,
        configuration.email_client.authorization_token,
        timeout,
    );

    // run(listener, connection_pool, email_client)
    Ok(AppState {
        connection_pool,
        email_client,
    })
}

pub fn router(state: AppState) -> Router {
    let request_id = HeaderName::from_static("x-request-id");
    
    Router::new()
    .route("/health_check", get(routes::health_check))
    .route("/subscriptions", post(routes::subscribe))
    .layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(request_id.clone(), MakeRequestUuid))
            .layer(PropagateRequestIdLayer::new(request_id))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().include_headers(true))
                    .on_response(DefaultOnResponse::new().include_headers(true)),
            ),
    )
    .with_state(state)
}