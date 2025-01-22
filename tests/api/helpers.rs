use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use std::sync::LazyLock;
use std::future::IntoFuture;

use zero_to_prod::{configuration::get_configuration, get_subscriber, init_subscriber, startup};

static INIT_SUBSCRIBER: LazyLock<()> = LazyLock::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        init_subscriber(get_subscriber(
            "test".into(),
            "debug".into(),
            std::io::stdout,
        ));
    } else {
        init_subscriber(get_subscriber("test".into(), "debug".into(), std::io::sink));
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

impl TestApp {
    pub fn init_subscriber() {
        LazyLock::force(&INIT_SUBSCRIBER);
    }
    pub async fn spawn() -> TestApp {
        Self::init_subscriber();
        let mut configuration = get_configuration().expect("Failed to read configuration");
        let database = &mut configuration.database;
        database.database_name = Uuid::new_v4().to_string();
    
        let mut connection = PgConnection::connect_with(&database.without_db())
            .await
            .expect("Failed to connect to Postgres");
        connection
            .execute(format!(r#"CREATE DATABASE "{}";"#, database.database_name).as_str())
            .await
            .expect("Failed to create database.");

        let state = startup::build(configuration).unwrap();
        let db_pool = state.connection_pool.clone();
    
        sqlx::migrate!("./migrations")
            .run(&state.connection_pool)
            .await
            .expect("Failed to migrate the database");
    
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        // tokio::spawn(zero_to_prod::run(listener, db_pool.clone(), email_client));
        tokio::spawn(axum::serve(listener, startup::router(state)).into_future());

        TestApp {
            address: format!("127.0.0.1:{port}"),
            db_pool,
        }
    }
    
}

