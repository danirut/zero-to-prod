use tokio::net::TcpListener;
use zero_to_prod::configuration::get_configuration;
use zero_to_prod::{startup::{router, build}, get_subscriber, init_subscriber};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    init_subscriber(get_subscriber(
        "zero-to-prod".into(),
        "debug".into(),
        std::io::stdout,
    ));

    let configuration = get_configuration().expect("Failed to read configuration.");
    let listener = TcpListener::bind(configuration.application.address()).await.unwrap();
    let app = router(build(configuration)?);
    tracing::info!("Starting zero-to-prod");
    // run our app with hyper, listening globally on port 3000
    Ok(axum::serve(listener, app).await?)
}
