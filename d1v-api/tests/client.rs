mod common;

use d1v_api::{Client, Error};
use httpmock::prelude::*;
use serde::{Deserialize, Serialize};

use crate::common::test_client;

#[tokio::test]
async fn test_get_ok() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/user/profile");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": {"name": "d1v"}}"#);
    });

    #[derive(Debug, Deserialize, PartialEq)]
    struct User {
        name: String,
    }

    let client = test_client(&server);
    let user: User = client.get("/api/user/profile").ok().await.unwrap();
    assert_eq!(user, User { name: "d1v".into() });

    mock.assert();
}

#[tokio::test]
async fn test_post_void() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/verify-code")
            .query_param("email", "test@example.com");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": null}"#);
    });

    let client = test_client(&server);
    client
        .post("/api/user/verify-code")
        .query(&[("email", "test@example.com")])
        .no_auth()
        .ok::<()>()
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn test_post_json_body() {
    let server = MockServer::start();

    #[derive(Debug, Serialize)]
    struct LoginRequest {
        email: String,
        code: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct LoginResponse {
        token: String,
    }

    let body = LoginRequest {
        email: "test@example.com".into(),
        code: "123456".into(),
    };

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/login")
            .header("content-type", "application/json")
            .json_body_obj(&body);
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": {"token": "abc123"}}"#);
    });

    let client = test_client(&server);
    let resp: LoginResponse = client
        .post("/api/user/login")
        .no_auth()
        .json(&body)
        .ok()
        .await
        .unwrap();
    assert_eq!(
        resp,
        LoginResponse {
            token: "abc123".into()
        }
    );

    mock.assert();
}

#[tokio::test]
async fn test_send_returns_response() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/api/items");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "ok", "data": [1, 2, 3], "total": 100}"#);
    });

    let client = test_client(&server);
    let resp = client.get("/api/items").send().await.unwrap();

    assert_eq!(resp.total, Some(100));
    let items: Vec<i32> = resp.ok().unwrap();
    assert_eq!(items, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_bearer_auth() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/protected")
            .header("authorization", "Bearer secret-token");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "ok", "data": null}"#);
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("secret-token")
        .build()
        .unwrap();
    client.get("/api/protected").ok::<()>().await.unwrap();

    mock.assert();
}

#[tokio::test]
async fn test_no_auth_skips_token() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/public")
            .header_missing("authorization");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "ok", "data": null}"#);
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("secret-token")
        .build()
        .unwrap();
    client
        .get("/api/public")
        .no_auth()
        .ok::<()>()
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn test_user_agent() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/test")
            .header("user-agent", "d1v-cli/0.1.0");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "ok", "data": null}"#);
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .user_agent("d1v-cli/0.1.0")
        .build()
        .unwrap();
    client.get("/api/test").ok::<()>().await.unwrap();

    mock.assert();
}

#[tokio::test]
async fn test_api_error() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/resource");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 401, "msg": "unauthorized", "data": null}"#);
    });

    let client = test_client(&server);
    let err = client.get("/api/resource").ok::<()>().await.unwrap_err();
    assert_eq!(err.to_string(), "api error 401: unauthorized");

    mock.assert();
}

#[tokio::test]
async fn test_validation_error() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/api/user/login");
        then.status(422)
            .header("content-type", "application/json")
            .body(
                r#"{"detail": [{"loc": ["body", "email"], "msg": "Field required", "type": "missing"}]}"#,
            );
    });

    let client = test_client(&server);
    let err = client.post("/api/user/login").ok::<()>().await.unwrap_err();
    assert!(matches!(err, Error::Validation(_)));

    mock.assert();
}

#[tokio::test]
async fn test_http_status_error() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/missing");
        then.status(404).body("not found");
    });

    let client = test_client(&server);
    let err = client.get("/api/missing").ok::<()>().await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "http status error (404 Not Found): not found"
    );

    mock.assert();
}
