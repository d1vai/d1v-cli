mod common;

use crate::common::{authed_client, test_client};
use d1v_api::Client;
use httpmock::prelude::*;
use secrecy::{ExposeSecret, SecretString};
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
    client
        .user()
        .send_code("test@example.com", None)
        .await
        .unwrap();

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
        .login("test@example.com", "999999")
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
        .login_password("test@example.com", &SecretString::from("123456"))
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
        .login_password("test@example.com", &SecretString::from("wrong"))
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "api error 1: password is incorrect");

    mock.assert();
}

#[tokio::test]
async fn password_login() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/password/login")
            .header("content-type", "application/json")
            .json_body(json!({
                "email": "test@example.com",
                "password": "secret123"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success", "data": "token123"}"#);
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .build()
        .unwrap();
    let token = client
        .user()
        .password_login("test@example.com", &SecretString::from("secret123"))
        .await
        .unwrap();
    assert_eq!(token.expose_secret(), "token123");

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
    let user = client
        .user()
        .update_info()
        .industry("tech")
        .call()
        .await
        .unwrap();
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
    client
        .user()
        .set_password(&SecretString::from("password123"))
        .await
        .unwrap();

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
        .send_forgot_password_email("test@example.com", None)
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
        .reset_password(
            "test@example.com",
            "123456",
            &SecretString::from("password123"),
        )
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn send_bind_email_code() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/bind-email/send")
            .header("content-type", "application/json")
            .json_body(json!({ "email": "new@example.com" }))
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
    client
        .user()
        .send_bind_email_code("new@example.com", None)
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn confirm_bind_email() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/bind-email/confirm")
            .header("content-type", "application/json")
            .json_body(json!({
                "email": "new@example.com",
                "code": "123456",
            }))
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
    client
        .user()
        .confirm_bind_email("new@example.com", "123456")
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn send_change_email_code() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/email/change/send")
            .header("content-type", "application/json")
            .json_body(json!({ "new_email": "new@example.com" }))
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
    client
        .user()
        .send_change_email_code("new@example.com", None)
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn confirm_change_email() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/email/change/confirm")
            .header("content-type", "application/json")
            .json_body(json!({
                "new_email": "new@example.com",
                "code": "123456",
            }))
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
    client
        .user()
        .confirm_change_email("new@example.com", "123456")
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn accept_invitation() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/invitation/accept")
            .header("content-type", "application/json")
            .json_body(json!({ "invite_code": "ABC123" }))
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
    client.user().accept_invitation("ABC123").await.unwrap();

    mock.assert();
}

#[tokio::test]
async fn list_invitees() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/invitations")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": [user_json()] }));
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap();
    let invitees = client.user().list_invitees().await.unwrap();
    assert_eq!(invitees.len(), 1);
    assert_eq!(invitees[0].slug, "d1v");

    mock.assert();
}

#[tokio::test]
async fn set_onboarded() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/onboarded/set")
            .header("content-type", "application/json")
            .json_body(json!({ "value": true }))
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
    client.user().set_onboarded(true).await.unwrap();

    mock.assert();
}

fn activity_json() -> serde_json::Value {
    json!({
        "start_date": "2026-03-21",
        "end_date": "2026-03-23",
        "days": 3,
        "counts": [
            { "date": "2026-03-21", "count": 5 },
            { "date": "2026-03-22", "count": 3 },
            { "date": "2026-03-23", "count": 0 }
        ]
    })
}

#[tokio::test]
async fn prompt_daily_activity() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/activity/prompt-daily")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": activity_json() }));
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap();
    let activity = client.user().prompt_daily_activity(None).await.unwrap();
    assert_eq!(activity.days, 3);
    assert_eq!(activity.counts.len(), 3);

    mock.assert();
}

#[tokio::test]
async fn prompt_daily_activity_with_days() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/activity/prompt-daily")
            .query_param("days", "7")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": activity_json() }));
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap();
    client.user().prompt_daily_activity(Some(7)).await.unwrap();

    mock.assert();
}

#[tokio::test]
async fn prompt_daily_activity_by_slug() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/activity/prompt-daily/slug/d1v");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": activity_json() }));
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .build()
        .unwrap();
    let activity = client
        .user()
        .prompt_daily_activity_by_slug("d1v", None)
        .await
        .unwrap();
    assert_eq!(activity.start_date, "2026-03-21");

    mock.assert();
}

#[tokio::test]
async fn prompt_daily_activity_by_user() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/activity/prompt-daily/user/42")
            .query_param("days", "30")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": activity_json() }));
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap();
    let activity = client
        .user()
        .prompt_daily_activity_by_user(42, Some(30))
        .await
        .unwrap();
    assert_eq!(activity.counts[0].count, 5);

    mock.assert();
}

#[tokio::test]
async fn prompt_daily_activity_by_user_forbidden() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/activity/prompt-daily/user/42");
        then.status(403)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 403,
                "msg": "User does not have sufficient privileges.",
                "data": null,
            }));
    });

    let client = Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap();
    let err = client
        .user()
        .prompt_daily_activity_by_user(42, None)
        .await
        .unwrap_err();

    match err {
        d1v_api::Error::Api(api) => {
            assert_eq!(api.code.raw(), 403);
            assert_eq!(api.message, "User does not have sufficient privileges.");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }

    mock.assert();
}

#[tokio::test]
async fn send_delete_account_code() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/account/delete/send")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success"}"#);
    });

    authed_client(&server)
        .user()
        .send_delete_account_code(None)
        .await
        .unwrap();
    mock.assert();
}

#[tokio::test]
async fn delete_account() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/user/account")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({ "code": "123456" }));
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"code": 0, "msg": "success"}"#);
    });

    authed_client(&server)
        .user()
        .delete_account("123456")
        .await
        .unwrap();
    mock.assert();
}

#[tokio::test]
async fn list_api_keys() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/user/api-keys")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": [{
                    "id": 1,
                    "name": "default",
                    "key_prefix": "d1v_",
                    "created_at": "2026-05-01T00:00:00",
                    "last_used_at": null
                }]
            }));
    });

    let keys = authed_client(&server).user().api_keys().await.unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].name, "default");
    mock.assert();
}

#[tokio::test]
async fn create_api_key() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/user/api-keys")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({ "name": "my-key" }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "api_key": "d1v_xxx",
                    "item": {
                        "id": 2,
                        "name": "my-key",
                        "key_prefix": "d1v_",
                        "created_at": "2026-05-01T00:00:00",
                        "last_used_at": null
                    }
                }
            }));
    });

    let result = authed_client(&server)
        .user()
        .create_api_key("my-key", None)
        .await
        .unwrap();
    assert_eq!(result.item.name, "my-key");
    assert_eq!(result.api_key.expose_secret(), "d1v_xxx");
    mock.assert();
}

#[tokio::test]
async fn revoke_api_key() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/user/api-keys/1")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "id": 1,
                    "name": "default",
                    "key_prefix": "d1v_",
                    "created_at": "2026-05-01T00:00:00",
                    "last_used_at": null
                }
            }));
    });

    let key = authed_client(&server)
        .user()
        .revoke_api_key(1)
        .await
        .unwrap();
    assert_eq!(key.id, 1);
    mock.assert();
}
