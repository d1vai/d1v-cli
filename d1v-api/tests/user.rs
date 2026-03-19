mod common;

use crate::common::test_client;
use httpmock::prelude::*;
use secrecy::ExposeSecret;
use serde_json::json;

#[tokio::test]
async fn send_verification_code() {
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
