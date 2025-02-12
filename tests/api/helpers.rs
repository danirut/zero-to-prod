use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::future::IntoFuture;
use std::{net::ToSocketAddrs, sync::LazyLock};
use uuid::Uuid;
use wiremock::MockServer;

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

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestApp {
    pub base_url: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
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

        let email_server = MockServer::start().await;
        configuration.email_client.base_url = email_server.uri();

        let state = startup::build(configuration).unwrap();
        let db_pool = state.connection_pool.clone();

        sqlx::migrate!("./migrations")
            .run(&state.connection_pool)
            .await
            .expect("Failed to migrate the database");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        // tokio::spawn(zero_to_prod::run(listener, db_pool.clone(), email_client));
        tokio::spawn(axum::serve(listener, startup::router(state)).into_future());

        TestApp {
            base_url: "127.0.0.1".to_owned(),
            port,
            db_pool,
            email_server,
        }
    }

    pub async fn post_subscriber(&self, body: &str) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("http://{}:{}/subscriptions", self.base_url, self.port))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_string())
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|link| *link.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            
            assert_eq!(confirmation_link.host_str().unwrap(), self.base_url);
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };
        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }
}
