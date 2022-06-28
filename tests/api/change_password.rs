use crate::helpers::{assert_is_redirect_to, spawn_app};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use uuid::Uuid;

#[tokio::test]
async fn you_must_be_logged_in_to_see_change_password_form() {
    let test_app = spawn_app().await;

    let response = test_app.get_change_password().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    let body = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": &new_password,
        "new_password_check": &new_password
    });

    let response = test_app.post_change_password(&body).await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let other_new_password = Uuid::new_v4().to_string();

    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password,
        }))
        .await;

    let body = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": &new_password,
        "new_password_check": &other_new_password
    });

    let response = test_app.post_change_password(&body).await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
        the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password
        }))
        .await;

    let body = serde_json::json!({
        "current_password": &wrong_password,
        "new_password": &new_password,
        "new_password_check": &new_password
    });

    let response = test_app.post_change_password(&body).await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[tokio::test]
async fn new_password_must_be_longer_than_12_characters() {
    let test_app = spawn_app().await;
    let too_short_password: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();

    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password
        }))
        .await;

    let body = serde_json::json!({
        "current_password": &test_app.test_user.password,
        "new_password": &too_short_password,
        "new_password_check": &too_short_password
    });

    let response = test_app.post_change_password(&body).await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>The new password must be longer than 12 characters and shorter than 128 characters.</i></p>"
    ));
}

#[tokio::test]
async fn new_password_must_be_shorter_than_256_characters() {
    let test_app = spawn_app().await;
    let too_long_password: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();

    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password
        }))
        .await;

    let body = serde_json::json!({
        "current_password": &test_app.test_user.password,
        "new_password": &too_long_password,
        "new_password_check": &too_long_password
    });

    let response = test_app.post_change_password(&body).await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>The new password must be longer than 12 characters and shorter than 128 characters.</i></p>"
    ));
}

#[tokio::test]
async fn changing_password_works() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    let login_body = serde_json::json!({
        "username": &test_app.test_user.username,
        "password": &test_app.test_user.password
    });

    let response = test_app.post_login(&login_body).await;
    // Successfully logged in
    assert_is_redirect_to(&response, "/admin/dashboard");

    let change_body = serde_json::json!({
        "current_password": &test_app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &new_password
    });

    let response = test_app.post_change_password(&change_body).await;
    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    // Successfully change password
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    let response = test_app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    let html_page = test_app.get_login_html().await;
    // Successfully logged out
    assert!(html_page.contains("<p><i>You have successfully logged out.</i></p>"));

    let login_body = serde_json::json!({
        "username": &test_app.test_user.username,
        "password": &new_password,
    });
    let response = test_app.post_login(&login_body).await;
    // Login with new password works
    assert_is_redirect_to(&response, "/admin/dashboard");
}
