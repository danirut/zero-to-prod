use crate::helpers::TestApp;

#[tokio::test]
async fn test_health_check() {
    // Arrange
    let TestApp { base_url, port, .. } = TestApp::spawn().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("http://{base_url}:{port}/health_check"))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}