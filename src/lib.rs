//! src/lib.rs
pub mod app_state;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod routes;
pub mod startup;

use tracing::{Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

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