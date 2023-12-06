use crate::helpers::{assert_is_redirected_to, spawn_app};


#[tokio::test]
async fn only_logged_in_access_dashboard() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard().await;

    assert_is_redirected_to(&response, "/login");
}
