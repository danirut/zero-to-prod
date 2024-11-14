use sqlx::{PgConnection, Connection, PgPool, Executor};
use zero_to_prod::{
    configuration::get_configuration,
    get_subscriber,
    init_subscriber};
use std::sync::LazyLock;
use uuid::Uuid;
use secrecy::ExposeSecret;

static INIT_SUBSCRIBER: LazyLock<()> = LazyLock::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        init_subscriber(get_subscriber("test".into(), "debug".into(), std::io::stdout));
    } else {
        init_subscriber(get_subscriber("test".into(), "debug".into(), std::io::sink));
    }

});

#[tokio::test]
async fn test_health_check() {
    // Arrange
    let TestApp { address: host, .. } = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("http://{host}/health_check"))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let TestApp { address: host, db_pool } = spawn_app().await;
    let client = reqwest::Client::new();
    let body = "name=marcin&email=mail%40marszy.com";


    // Act
    let response = client
        .post(&format!("http://{host}/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");
    
    // Arrange
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    
    assert_eq!(saved.email, "mail@marszy.com");
    assert_eq!(saved.name, "marcin");
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    // Arrange
    let TestApp { address: host, .. } = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=marcin", "missing mail"),
        ("email=mail%40marszy.com", "missing name"),
        ("", "missing both name and email")
    ];

    // Act
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("http://{host}/subscriptions"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");
        
        // Arrange
        assert_eq!(422, response.status().as_u16(), 
            "The API did not fail with 400 Bad Request when the payload was {}",
        error_message);
    }
}

struct TestApp {
    address: String,
    db_pool: PgPool
}

async fn spawn_app() -> TestApp {
    LazyLock::force(&INIT_SUBSCRIBER); 
    
    let mut configuration = get_configuration().expect("Failed to read configuration");
    let database = &mut configuration.database;
    database.database_name = Uuid::new_v4().to_string();

    let mut connection = PgConnection::connect_with(&database.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection.execute(format!(r#"CREATE DATABASE "{}";"#, database.database_name).as_str())
        .await
        .expect("Failed to create database.");
        
    let db_pool = PgPool::connect_with(database.with_db())
        .await
        .expect("Failed to connect to Postgres");
    
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(zero_to_prod::run(listener, db_pool.clone()));

    TestApp {
        address: format!("127.0.0.1:{port}"),
        db_pool
    }
}