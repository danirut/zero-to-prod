use secrecy::ExposeSecret;
use sqlx::PgPool;
use tokio::net::TcpListener;
use zero_to_prod::configuration::get_configuration;
use zero_to_prod::{get_subscriber, init_subscriber, run};

#[tokio::main]
async fn main() {
    init_subscriber(get_subscriber(
        "zero-to-prod".into(),
        "debug".into(),
        std::io::stdout,
    ));

    
    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool =
        PgPool::connect_lazy_with(configuration.database.with_db());
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).await.unwrap();
    run(listener, connection_pool).await;
}
