mod common;

use crate::common::test_client;
use d1v_api::{Client, UpdateUser};
use httpmock::prelude::*;
use secrecy::ExposeSecret;
use serde_json::json;

#[tokio::test]
async fn send_code() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/verify-code")
            .query_param("email", "test@example.com")
            .header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": null}"#);
    });

    let client = test_client(&server);
    client.user().send_code("test@example.com").await.unwrap();

    mock.assert();
}

#[tokio::test]
async fn check_code() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/verify-code/check")
            .header("content-type", "application/json")
            .json_body(json!({
                "email": "test@example.com",
                "code": "123456",
                "purpose": "login",
            }))
            .header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": null}"#);
    });

    let client = test_client(&server);
    client
        .user()
        .check_code("test@example.com", "123456", "login")
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn login() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/login")
            .header("content-type", "application/json")
            .json_body(json!({
                "email": "test@example.com",
                "verify_code": "123456",
            }))
            .header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": "abc123"}"#);
    });

    let client = test_client(&server);
    let token = client
        .user()
        .login("test@example.com", "123456")
        .await
        .unwrap();
    assert_eq!(token.expose_secret(), "abc123");

    mock.assert();
}

#[tokio::test]
async fn login_wrong_code() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/api/user/login");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 1, "msg": "verification code is incorrect", "data": null}"#);
    });

    let client = test_client(&server);
    let err = client
        .user()
        .login("test@example.com", "wrong")
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "api error 1: verification code is incorrect"
    );

    mock.assert();
}

#[tokio::test]
async fn login_password() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/login/password")
            .header("content-type", "application/json")
            .json_body(json!({
                "email": "test@example.com",
                "password": "123456",
            }))
            .header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": "abc123"}"#);
    });

    let client = test_client(&server);
    let token = client
        .user()
        .login_password("test@example.com", "123456")
        .await
        .unwrap();
    assert_eq!(token.expose_secret(), "abc123");

    mock.assert();
}

#[tokio::test]
async fn login_password_wrong() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/api/user/login/password");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 1, "msg": "password is incorrect", "data": null}"#);
    });

    let client = test_client(&server);
    let err = client
        .user()
        .login_password("test@example.com", "wrong")
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "api error 1: password is incorrect");

    mock.assert();
}

fn user_json() -> serde_json::Value {
    json!({
        "id": 1,
        "slug": "d1v",
        "is_agent": false,
        "picture": "https://d1v.ai/avatar.png",
        "is_onboarded": true,
        "email": "test@example.com",
    })
}

#[tokio::test]
async fn info() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/info")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": user_json() }));
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap();
    let user = client.user().info().await.unwrap();
    assert_eq!(user.id, 1);
    assert_eq!(user.slug, "d1v");
    assert_eq!(user.email.as_deref(), Some("test@example.com"));

    mock.assert();
}

#[tokio::test]
async fn update_info() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/api/user/info")
            .header("content-type", "application/json")
            .json_body(json!({ "industry": "tech" }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": user_json() }));
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap();
    let update = UpdateUser {
        industry: Some("tech".into()),
        ..Default::default()
    };
    let user = client.user().update_info(&update).await.unwrap();
    assert_eq!(user.id, 1);

    mock.assert();
}

#[tokio::test]
async fn public_user() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/public/42")
            .header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": user_json() }));
    });

    let client = test_client(&server);
    let user = client.user().public_user(42).await.unwrap();
    assert_eq!(user.id, 1);

    mock.assert();
}

#[tokio::test]
async fn public_user_by_slug() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/public/slug/d1v")
            .header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": user_json() }));
    });

    let client = test_client(&server);
    let user = client.user().public_user_by_slug("d1v").await.unwrap();
    assert_eq!(user.slug, "d1v");

    mock.assert();
}

#[tokio::test]
async fn all_users() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/user/all");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": [user_json()] }));
    });

    let client = test_client(&server);
    let users = client.user().all_users().await.unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].id, 1);

    mock.assert();
}

#[tokio::test]
async fn set_password() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/password/set")
            .header("content-type", "application/json")
            .json_body(json!({ "password": "password123" }))
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": null}"#);
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap();
    client.user().set_password("password123").await.unwrap();

    mock.assert();
}

#[tokio::test]
async fn send_forgot_password_email() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/password/forgot/send")
            .header("content-type", "application/json")
            .json_body(json!({ "email": "test@example.com" }))
            .header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": null}"#);
    });

    let client = test_client(&server);
    client
        .user()
        .send_forgot_password_email("test@example.com")
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn reset_password() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/password/reset")
            .header("content-type", "application/json")
            .json_body(json!({
                "email": "test@example.com",
                "code": "123456",
                "new_password": "password123",
            }))
            .header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": null}"#);
    });

    let client = test_client(&server);
    client
        .user()
        .reset_password("test@example.com", "123456", "password123")
        .await
        .unwrap();

    mock.assert();
}
