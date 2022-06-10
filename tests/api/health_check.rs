use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;
    let endpoint = format!("{}/health_check", test_app.address);

    let client = reqwest::Client::new();

    let response = client
        .get(endpoint)
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
