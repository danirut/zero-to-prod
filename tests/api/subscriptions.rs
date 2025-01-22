use zero_to_prod::email_client::EmailClient;

use crate::helpers::TestApp;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let TestApp {
        address: host,
        db_pool,
    } = TestApp::spawn().await;
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
    let TestApp { address: host, .. } = TestApp::spawn().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=marcin", "missing mail"),
        ("email=mail%40marszy.com", "missing name"),
        ("", "missing both name and email"),
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
    let TestApp { address: host, .. } = TestApp::spawn().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("http://{host}/subscriptions"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");
        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[ignore]
    #[tokio::test]
    async fn send_email_tests_postmark() {
        use zero_to_prod::domain::SubscriberEmail;
        use secrecy::SecretString;
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
        let _ = email_client.send_email(
            SubscriberEmail::parse("mail@danirut.com".into()).unwrap(),
            "Hello from Postmark".into(),
            "<strong>Hello</strong> dear Postmark user.".into(),
            "Hello dear Postmark user.".into(),
        ).await
        .unwrap();


    }