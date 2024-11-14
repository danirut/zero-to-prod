//! src/lib.rs
pub mod configuration;
pub mod routes;
pub mod startup;

use axum::{
    http::HeaderName,
    routing::{get, post},
    Router,
    extract::{Request, Path}
};
use tower::ServiceBuilder;
use tower_http::{
    ServiceBuilderExt,
    request_id::{
        MakeRequestUuid,
        SetRequestIdLayer,
        PropagateRequestIdLayer
    },
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};
use tokio::net::TcpListener;
use sqlx::PgPool;

pub fn get_subscriber<Sink>(name: String, env_filter: String, sink: Sink) -> impl Subscriber + Sync + Send
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static
{
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(env_filter));

    let formatting_layer = BunyanFormattingLayer::new(
        name,
        sink
    );

    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    subscriber
}

pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) {
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
}


pub async fn run(listener: TcpListener, connection_pool: PgPool) {
    // build our application with a single route

    let request_id = HeaderName::from_static("x-request-id");
    let app = Router::new()
        .route("/health_check", get(routes::health_check))
        .route("/subscriptions", post(routes::subscribe))
        .layer(
            ServiceBuilder::new()
                .layer(
                    SetRequestIdLayer::new(
                        request_id.clone(),
                        MakeRequestUuid))
                .layer(PropagateRequestIdLayer::new(request_id))
                .layer(TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().include_headers(true))
                    .on_response(DefaultOnResponse::new().include_headers(true))
                )
                
        )
        .with_state(connection_pool);

    tracing::info!("Starting zero-to-prod");
    // run our app with hyper, listening globally on port 3000
    axum::serve(listener, app).await.unwrap();
}