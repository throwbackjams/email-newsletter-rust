use crate::helpers::{spawn_app, TestApp, ConfirmationLinks};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
    create_unconfirmed_subscriber(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0) // No request should be fired at Postmark
        .mount(&test_app.email_server)
        .await;
    
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = test_app.post_newsletters(newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 200);

}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;
    
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = test_app.post_newsletters(newsletter_request_body).await;
    
    assert_eq!(response.status().as_u16(), 200);

}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Use mock guard and mount_as_scoped instead of mount to drop this server / not interfere with server in "main" test
    // Confirm that confirmation email was sent out to /email server after new individual subscribers
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;
    
    test_app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &test_app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    
    test_app.get_confirmation_links(email_request)
    
}

async fn create_confirmed_subscriber(test_app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(test_app).await;

    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        (
        serde_json::json!({
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
            }
        }),
        "missing title",
        ),
        (
        serde_json::json!({"title": "Newsletter!"}),
        "missing content",
        ),
        ];


    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_newsletters(invalid_body).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The newsletter API did not fail with 400 bad request when the payload was {}.",
            error_message);
    }
}
