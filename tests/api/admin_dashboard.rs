use crate::helpers::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    let test_app = spawn_app().await;

    let response = test_app.get_admin_dashboard.html().await;

    assert_is_redirect_to(&response, "/login");

}