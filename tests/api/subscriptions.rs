use std::sync::Arc;

use reqwest::Url;
use zero_to_prod::email_client::EmailClient;

use crate::helpers::TestApp;

use wiremock::{
    matchers::{method, path},
    Mock, MockServer, Request, ResponseTemplate,
};

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = TestApp::spawn().await;
    let body = "name=Marcin&email=mail%40marszy.com";
    let client = reqwest::Client::new();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response_subscribe = app.post_subscriber(body).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);
    
    // Act
    let response_confimartion = client
        .get(confirmation_links.html)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "mail@marszy.com");
    assert_eq!(saved.name, "Marcin");
    assert_eq!(saved.status, "confirmed");
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_200_if_called() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = "name=Marcin&email=mail%40marszy.com";
    let client = reqwest::Client::new();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let response_subscribe = app.post_subscriber(body).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    // Assert

}

#[tokio::test]
async fn the_link_returned_by_subscribe_has_the_same_token_as_in_database() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = "name=Marcin&email=mail%40marszy.com";
    let client = reqwest::Client::new();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let response_subscribe = app.post_subscriber(body).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    let link_token = confirmation_links.html.path_segments().unwrap().last().unwrap();

    let saved = sqlx::query!("SELECT id FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    let db_token = sqlx::query!(
            r#"SELECT token FROM subscription_tokens WHERE subscriber_id = $1"#,
            saved.id,
        )
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to execute query");
        


    // Assert
    assert_eq!(link_token, db_token.token);
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = "name=marcin&email=mail%40marszy.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let response = app.post_subscriber(body).await;

    // Arrange
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "mail@marszy.com");
    assert_eq!(saved.name, "marcin");
    assert_eq!(saved.status, "pending".to_string());
}

#[tokio::test]
async fn subscribe_inserts_subscriber_into_db_for_valid_form_data() {
    // Arrange
    let app = TestApp::spawn().await;
    let body = "name=marcin&email=mail%40marszy.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let response = app.post_subscriber(body).await;

    // Assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "mail@marszy.com");
    assert_eq!(saved.name, "marcin");
    assert_eq!(saved.status, "pending".to_string());
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    // Arrange
    let app = TestApp::spawn().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=marcin", "missing mail"),
        ("email=mail%40marszy.com", "missing name"),
        ("", "missing both name and email"),
    ];

    // Act
    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriber(invalid_body).await;

        // Arrange
        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    // Arrange
    let app = TestApp::spawn().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriber(invalid_body).await;
        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let app = TestApp::spawn().await;
    let body = "name=Marcin&email=mail%40marszy.com";
    let client = reqwest::Client::new();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let response = app.post_subscriber(body).await;

    // Assert
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
    assert_eq!(200, response.status().as_u16());
}

#[ignore]
#[tokio::test]
async fn send_email_tests_postmark() {
    use secrecy::SecretString;
    use zero_to_prod::domain::SubscriberEmail;
    //         curl "https://api.postmarkapp.com/email" \
    //   -X POST \
    //   -H "Accept: application/json" \
    //   -H "Content-Type: application/json" \
    //   -H "X-Postmark-Server-Token: c78b0e40-cb3a-44e8-8501-e8a055ebb7d5" \
    //   -d '{
    //         "From": "mail@danirut.com",
    //         "To": "mail@danirut.com",
    //         "Subject": "Hello from Postmark",
    //         "HtmlBody": "<strong>Hello</strong> dear Postmark user.",
    //         "MessageStream": "newsletter_confirmations"
    //       }
    TestApp::init_subscriber();
    tracing::info!("Sending e-mail via postmark!");

    let email_client = EmailClient::new(
        "https://api.postmarkapp.com".into(),
        SubscriberEmail::parse("mail@danirut.com".into()).unwrap(),
        SecretString::new("c78b0e40-cb3a-44e8-8501-e8a055ebb7d5".into()),
        std::time::Duration::from_millis(1000),
    );
    let _ = email_client
        .send_email(
            SubscriberEmail::parse("mail@danirut.com".into()).unwrap(),
            "Hello from Postmark".into(),
            "<strong>Hello</strong> dear Postmark user.".into(),
            "Hello dear Postmark user.".into(),
        )
        .await
        .unwrap();
}
