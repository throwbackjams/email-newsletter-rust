use zero2prod::startup::run;
use std::net::TcpListener;
use sqlx::{PgConnection, Connection};
use zero2prod::configruation::get_configuration;

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();
    let endpoint = format!("{}/health_check", address);

    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let address = spawn_app();
    let endpoint = format!("{}/subscribe", &address);
    let configuration = get_configuration().expect("Failed to get configuration");

    let connection_string = configuration.database.connection_string();

    let connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to Postgres");

    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute subscribe reqeust");
    
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_a_400_when_request_is_missing() {
    let address = spawn_app();
    let endpoint = format!("{}/subscribe", &address);

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email")
        ];

    let client = reqwest::Client::new();

    for (invalid_response, error_message) in test_cases {

        let response = client
            .post(&format!("{}/subscribe", &address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_response)
            .send()
            .await
            .expect("Failed to execute subscribe request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 bad request when the payload was {}",
            error_message
        );

    }


}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind address");
    let port = listener.local_addr().unwrap().port();
    let server = run(listener).expect("Failed to run server");

    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)

}