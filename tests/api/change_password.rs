use rand::distributions::{Alphanumeric, DistString};
use uuid::Uuid;

use crate::helpers::{assert_is_redirected_to, spawn_app};


#[tokio::test]
async fn only_logged_in_sees_change_password_form() {
    let app = spawn_app().await;

    let response = app.get_change_password().await;

    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn only_logged_in_changes_password() {
    let app = spawn_app().await;
    let new_pass = Uuid::new_v4().to_string();
    let payload = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": &new_pass,
        "new_password_check": &new_pass,
    });

    let response = app .post_change_password(&payload).await;

    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    let new_pass = Uuid::new_v4().to_string();
    let different_new_pass = Uuid::new_v4().to_string();
    let payload = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });

    app.post_login(&payload).await;

    let payload = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_pass,
        "new_password_check": &different_new_pass,
    });
    let response =app.post_change_password(&payload).await;

    assert_is_redirected_to(&response, "/admin/password");

    let html = app.get_change_password_html().await;

    assert!(html.contains("<p><i>Password fields must match.</i></p>"));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    let new_pass = Uuid::new_v4().to_string();
    let bad_pass = Uuid::new_v4().to_string();
    let payload = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });

    app.post_login(&payload).await;
    
    let payload = serde_json::json!({
        "current_password": &bad_pass,
        "new_password": &new_pass,
        "new_password_check": &new_pass,
    });
    let response = app
        .post_change_password(&payload)
        .await;

    assert_is_redirected_to(&response, "/admin/password");

    let html = app.get_change_password_html().await;

    assert!(html.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[tokio::test]
async fn new_password_must_be_correct_length() {
    let app = spawn_app().await;
    let new_pass = Alphanumeric.sample_string(&mut rand::thread_rng(), 12);
    let payload = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });

    app.post_login(&payload).await;
    
    let payload = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_pass,
        "new_password_check": &new_pass,
    });
    let response = app
        .post_change_password(&payload)
        .await;

    assert_is_redirected_to(&response, "/admin/password");

    let html = app.get_change_password_html().await;

    assert!(html.contains("<p><i>New password must be between 12 and 128 characters.</i></p>"));
    
    let new_pass = Alphanumeric.sample_string(&mut rand::thread_rng(), 128);
    let payload = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_pass,
        "new_password_check": &new_pass,
    });
    let response = app
        .post_change_password(&payload)
        .await;

    assert_is_redirected_to(&response, "/admin/password");

    let html = app.get_change_password_html().await;

    assert!(html.contains("<p><i>New password must be between 12 and 128 characters.</i></p>"));
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;

    let payload = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let response = app.post_login(&payload).await;

    assert_is_redirected_to(&response, "/admin/dashboard");
    
    let html = app.get_admin_dashboard_html().await;

    assert!(html.contains(&format!("Welcome {}", app.test_user.username)));

    let response = app.post_logout().await;
    
    assert_is_redirected_to(&response, "/login");

    let html = app.get_login_html().await;
    
    assert!(html.contains(r#"<p><i>You have successfully logged out.</i>"#));

    let response = app.get_admin_dashboard().await;

    assert_is_redirected_to(&response, "/login");
}

#[tokio::test]
async fn change_password_works() {
    let app = spawn_app().await;
    let new_pass = Uuid::new_v4().to_string();

    let payload = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let response = app.post_login(&payload).await;

    assert_is_redirected_to(&response, "/admin/dashboard");
    
    let payload = serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_pass,
        "new_password_check": &new_pass,
    });
    let response = app.post_change_password(&payload).await;
    
    assert_is_redirected_to(&response, "/admin/password");

    let html = app.get_change_password_html().await;

    assert!(html.contains("<p><i>Your password has been changed.</i></p>"));

    let response = app.post_logout().await;

    assert_is_redirected_to(&response, "/login");

    let html = app.get_login_html().await;

    assert!(html.contains("<p><i>You have successfully logged out.</i></p>"));
    
    let payload = serde_json::json!({
        "username": &app.test_user.username,
        "password": &new_pass,
    });
    let response = app.post_login(&payload).await;

    assert_is_redirected_to(&response, "/admin/dashboard");
}
