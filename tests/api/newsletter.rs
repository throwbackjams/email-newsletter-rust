use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::Fake;
use std::time::Duration;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, MockBuilder, ResponseTemplate};

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(&serde_json::json!({
        "name": name,
        "email": email
    }))
    .unwrap();

    // Use mock guard and mount_as_scoped instead of mount to drop this server / not interfere with server in "main" test
    // Confirm that confirmation email was sent out to /email server after new individual sub&scribers
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;

    test_app
        .post_subscriptions(body.into())
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
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
    create_unconfirmed_subscriber(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0) // No request should be fired at Postmark
        .mount(&test_app.email_server)
        .await;

    test_app.login().await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response = test_app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = test_app.get_admin_newsletters_html().await;
    assert!(html_page.contains("The newsletter issue has been accepted"));

    test_app.dispatch_all_pending_emails().await;

    // Mock verifies on Drop that no newsletters were sent
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

    test_app.login().await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response = test_app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = test_app.get_admin_newsletters_html().await;
    assert!(html_page.contains("The newsletter issue has been accepted"));

    test_app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        (
            serde_json::json!({
                "html_content": "<p>Newsletter body as HTML</p>",
                "text_content": "Newsletter body as plain text",
                "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "text_content": "Newsletter body as plain text",
                "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            "missing html content",
        ),
    ];

    test_app.login().await;

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_newsletters(&invalid_body).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The newsletter API did not fail with 400 bad request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn newsletter_form_redirects_to_login_if_not_logged_in() {
    let test_app = spawn_app().await;

    let response = test_app.get_admin_newsletters().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn sending_newsletter_while_not_logged_in_redirects_to_login() {
    let test_app = spawn_app().await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response = test_app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;
    test_app.login().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });

    let response = test_app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = test_app.get_admin_newsletters_html().await;
    assert!(html_page.contains("The newsletter issue has been accepted"));

    // Submit newsletter form *again*
    let response = test_app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = test_app.get_admin_newsletters_html().await;
    assert!(html_page.contains("The newsletter issue has been accepted"));

    test_app.dispatch_all_pending_emails().await;

    // Mock verifies that the newsletter has been sent only once
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;
    test_app.login().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        // Set delay to give time for second request to arrive
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response1 = test_app.post_newsletters(&newsletter_request_body).await;
    let response2 = test_app.post_newsletters(&newsletter_request_body).await;

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );

    test_app.dispatch_all_pending_emails().await;

    // Mock verifies on Drop that only one newsetter was sent
}

fn when_sending_an_email() -> MockBuilder {
    Mock::given(path("/email")).and(method("POST"))
}
