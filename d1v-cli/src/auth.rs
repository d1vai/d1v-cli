use std::fs::{self, OpenOptions};
use std::io::{IsTerminal, Write, stdin};
use std::path::PathBuf;
use std::time::Duration;

use anstyle::Style;
use base64::Engine;
use secrecy::SecretString;
use serde::Serialize;
use tracing::debug;

use crate::error::{Error, Result};
use crate::output::format_duration;
use crate::text::{Field, Fields, Line, Render, RenderContext, Span, Text};
use crate::token::{TokenSource, TokenStore};
use crate::ui::{Password, Select, SelectOption};
use crate::{Context, i18n, prompt, symbols, t, theme};

pub async fn login(ctx: &Context, password: bool) -> Result<()> {
    let token = if password {
        let email = prompt::email()?;
        authenticate_password(ctx, &email).await?
    } else {
        authenticate_code(ctx).await?
    };

    ctx.tokens.save(&token)?;
    ctx.client.token(token);
    debug!("login successful");
    ctx.success(t!("auth-login-success"));

    Ok(())
}

/// Uses a one-time browser approval session to obtain a durable, revocable API key.
pub async fn login_with_browser(ctx: &Context) -> Result<()> {
    let device_id = load_or_create_device_id()?;
    let session = ctx
        .client
        .user()
        .create_cli_login_session(&device_id, "This device")
        .await?;

    // The browser URL contains a one-time nonce but never the key or poll secret.
    if open::that(&session.browser_url).is_err() {
        ctx.message(format!(
            "Open this URL in your browser:\n{}",
            session.browser_url
        ));
    } else {
        ctx.info("Your browser has opened. Complete login there, then return here.");
    }

    for _ in 0..300 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let status = ctx
            .client
            .user()
            .cli_login_status(&session.session_id, &session.poll_secret)
            .await?;
        match status.status.as_str() {
            "pending" => continue,
            "approved" => {
                let key = ctx
                    .client
                    .user()
                    .consume_cli_login_session(&session.session_id, &session.poll_secret)
                    .await?
                    .api_key;
                ctx.tokens.save(&key)?;
                ctx.client.token(key);
                ctx.success(t!("auth-login-success"));
                return Ok(());
            }
            "expired" => return Err(anyhow::anyhow!("browser login timed out").into()),
            "consumed" => {
                return Err(anyhow::anyhow!("browser login session was already consumed").into());
            }
            _ => return Err(anyhow::anyhow!("invalid browser login status").into()),
        }
    }
    Err(anyhow::anyhow!("browser login timed out").into())
}

/// Validates the configured credential against the API and repairs it through
/// the browser device flow when it is missing or rejected.
pub async fn ensure_authenticated(ctx: &Context) -> Result<()> {
    let has_token = ctx.tokens.lookup()?.is_some();
    let locally_expired = ctx.client.is_token_expired();

    if has_token && !locally_expired {
        match ctx.client.user().info().await {
            Ok(_) => return Ok(()),
            Err(error) if is_auth_failure(&error) => {}
            Err(error) => return Err(error.into()),
        }
    } else if has_token && locally_expired {
        if !stdin().is_terminal() {
            return Err(Error::TokenExpired);
        }
        ctx.info("Stored credential is expired; opening browser login.");
    }

    if !stdin().is_terminal() {
        return if has_token {
            Err(Error::Api(d1v_api::Error::Api(d1v_api::ApiError::new(
                d1v_api::ApiCode::Unauthorized,
                "stored credential is invalid or expired",
            ))))
        } else {
            Err(Error::NotLoggedIn)
        };
    }

    ctx.info("Credential rejected; opening browser login.");
    login_with_browser(ctx).await?;
    ctx.client
        .user()
        .info()
        .await
        .map(|_| ())
        .map_err(Into::into)
}

fn is_auth_failure(error: &d1v_api::Error) -> bool {
    matches!(
        error,
        d1v_api::Error::Api(api)
            if matches!(api.code, d1v_api::ApiCode::Unauthorized | d1v_api::ApiCode::Forbidden)
                || api.message.to_ascii_lowercase().contains("invalid api key")
    ) || matches!(
        error,
        d1v_api::Error::HttpStatus(status)
            if status.status == reqwest::StatusCode::UNAUTHORIZED
                || status.status == reqwest::StatusCode::FORBIDDEN
    )
}

fn device_id_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("home directory is unavailable"))?;
    Ok(home.join(".d1v").join("device-id"))
}

fn load_or_create_device_id() -> Result<String> {
    let path = device_id_path()?;
    load_or_create_device_id_at(&path)
}

fn load_or_create_device_id_at(path: &std::path::Path) -> Result<String> {
    if let Ok(value) = fs::read_to_string(&path) {
        let value = value.trim();
        if value.len() >= 16 && value.len() <= 128 {
            return Ok(value.to_owned());
        }
    }
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|err| anyhow::anyhow!("failed to generate device ID: {err}"))?;
    let value = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let parent = path.parent().expect("device id has a parent");
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".device-id.{}.tmp", std::process::id()));
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(value.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    fs::rename(&temp, &path)?;
    Ok(value)
}

/// Presents an interactive menu to choose a login method, then executes it.
pub async fn login_interactive(ctx: &Context) -> Result<()> {
    enum Method {
        Browser,
        ApiKey,
        Code,
        Password,
        Token,
    }

    let method = Select::new(t!("auth-method-prompt"))
        .option(SelectOption::new(
            Method::Browser,
            t!("auth-method-browser"),
        ))
        .option(SelectOption::new(Method::ApiKey, t!("auth-method-api-key")))
        .option(SelectOption::new(Method::Code, t!("auth-method-code")))
        .option(SelectOption::new(
            Method::Password,
            t!("auth-method-password"),
        ))
        .option(SelectOption::new(Method::Token, t!("auth-method-token")))
        .default_index(0)
        .prompt()?;

    match method {
        Method::Browser => login_with_browser(ctx).await,
        Method::ApiKey => login_with_api_key(ctx),
        Method::Code => login(ctx, false).await,
        Method::Password => login(ctx, true).await,
        Method::Token => login_with_token(ctx),
    }
}

pub fn login_with_token(ctx: &Context) -> Result<()> {
    let token = if stdin().is_terminal() {
        Password::new(t!("auth-token-prompt"))
            .with_validator(|s| {
                if s.is_empty() {
                    Err(t!("auth-token-empty"))
                } else {
                    Ok(())
                }
            })
            .prompt()?
    } else {
        let mut buf = String::new();
        stdin().read_line(&mut buf)?;
        SecretString::from(buf.trim())
    };

    ctx.tokens.save(&token)?;
    ctx.client.token(token);
    debug!("login with token successful");
    ctx.success(t!("auth-login-success"));

    Ok(())
}

pub fn login_with_api_key(ctx: &Context) -> Result<()> {
    let api_key = if stdin().is_terminal() {
        Password::new(t!("auth-api-key-prompt"))
            .with_validator(|s| {
                if s.is_empty() {
                    Err(t!("auth-api-key-empty"))
                } else {
                    Ok(())
                }
            })
            .prompt()?
    } else {
        let mut buf = String::new();
        stdin().read_line(&mut buf)?;
        SecretString::from(buf.trim())
    };

    ctx.tokens.save(&api_key)?;
    ctx.client.token(api_key);
    debug!("login with API key successful");
    ctx.success(t!("auth-login-success"));

    Ok(())
}

async fn authenticate_code(ctx: &Context) -> Result<SecretString> {
    let pending = prompt::email_pending()?;
    let email = pending.value().to_string();

    debug!("sending verification code");
    pending
        .spin_ok(ctx.client.user().send_code(&email, i18n::locale()))
        .await?;
    ctx.info(t!("auth-code-sent", email = &email));

    let pending = prompt::code_pending()?;
    let code = pending.value().to_string();

    debug!("logging in with verification code");
    pending
        .spin_ok(ctx.client.user().login(&email, &code))
        .await
}

async fn authenticate_password(ctx: &Context, email: &str) -> Result<SecretString> {
    let password = Password::new(t!("auth-password-prompt"))
        .with_validator(|s| {
            if s.is_empty() {
                Err(t!("password-empty"))
            } else {
                Ok(())
            }
        })
        .prompt()?;

    debug!("logging in with password");
    Ok(ctx.client.user().login_password(email, &password).await?)
}

pub fn logout(ctx: &Context) -> Result<()> {
    if ctx.tokens.lookup()?.is_none() {
        ctx.message(t!("auth-not-logged-in"));
        return Ok(());
    }

    ctx.tokens.delete()?;
    debug!("logged out");
    ctx.success(t!("auth-logout-success"));

    Ok(())
}

/// Prompts the user to re-authenticate when the token has expired.
pub async fn prompt_relogin(ctx: &Context) -> Result<bool> {
    enum Choice {
        Yes,
        No,
    }

    let choice = Select::new(t!("auth-relogin-prompt"))
        .option(SelectOption::new(Choice::Yes, t!("auth-relogin-yes")))
        .option(SelectOption::new(Choice::No, t!("auth-relogin-no")))
        .default_index(0)
        .prompt()?;

    match choice {
        Choice::Yes => {
            login(ctx, false).await?;
            Ok(true)
        }
        Choice::No => Ok(false),
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum AuthStatus {
    NotLoggedIn,
    LoggedIn {
        source: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subject: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_in: Option<i64>,
    },
    Expired {
        source: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subject: Option<String>,
    },
}

struct AuthStatusView<'a> {
    status: &'a AuthStatus,
}

impl AuthStatusView<'_> {
    fn symbol(&self) -> &'static str {
        match self.status {
            AuthStatus::LoggedIn { .. } => symbols::SUCCESS,
            AuthStatus::NotLoggedIn | AuthStatus::Expired { .. } => symbols::ERROR,
        }
    }

    fn title(&self) -> String {
        match self.status {
            AuthStatus::NotLoggedIn => t!("auth-status-not-logged-in"),
            AuthStatus::Expired { .. } => t!("auth-status-expired"),
            AuthStatus::LoggedIn { .. } => t!("auth-status-logged-in"),
        }
    }

    fn style(&self) -> Style {
        match self.status {
            AuthStatus::LoggedIn { .. } => theme::ansi::success(),
            AuthStatus::NotLoggedIn | AuthStatus::Expired { .. } => theme::ansi::error(),
        }
    }

    fn source(&self) -> Option<&str> {
        match self.status {
            AuthStatus::NotLoggedIn => None,
            AuthStatus::LoggedIn { source, .. } | AuthStatus::Expired { source, .. } => {
                Some(source)
            }
        }
    }

    fn subject(&self) -> Option<&str> {
        match self.status {
            AuthStatus::NotLoggedIn => None,
            AuthStatus::LoggedIn { subject, .. } | AuthStatus::Expired { subject, .. } => {
                subject.as_deref()
            }
        }
    }

    fn expires_in(&self) -> Option<i64> {
        match self.status {
            AuthStatus::LoggedIn { expires_in, .. } => *expires_in,
            AuthStatus::NotLoggedIn | AuthStatus::Expired { .. } => None,
        }
    }
}

impl Render for AuthStatusView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        let style = self.style();

        let line = Line::styled(self.symbol(), style)
            .push_plain(" ")
            .push_styled(self.title(), style);

        let line = if let Some(source) = self.source() {
            line.push_plain(" ")
                .push_styled(format!("({source})"), theme::ansi::dim())
        } else {
            line
        };

        let mut fields = Vec::new();
        if let Some(subject) = self.subject() {
            fields.push(Field::new(
                Span::styled(t!("auth-status-label-user"), theme::ansi::label()),
                Span::styled(subject.to_owned(), theme::ansi::value()),
            ));
        }

        if let Some(secs) = self.expires_in() {
            fields.push(Field::new(
                Span::styled(t!("auth-status-label-expires"), theme::ansi::label()),
                Span::styled(format_duration(secs), theme::ansi::value()),
            ));
        }

        Text::new().line(line).render(ctx)?;
        Fields::new(fields).indent(2).render(ctx)
    }
}

pub fn status(ctx: &Context) -> Result<()> {
    let status = match ctx.tokens.source().map(String::from) {
        None => AuthStatus::NotLoggedIn,
        Some(source) if let Some(claims) = ctx.client.claims() => {
            let expired = claims.is_expired();
            let expires_in = claims.expires_in().map(|d| d.as_secs());
            let subject = claims.subject;

            if expired {
                AuthStatus::Expired { source, subject }
            } else {
                AuthStatus::LoggedIn {
                    source,
                    subject,
                    expires_in,
                }
            }
        }
        Some(source) => AuthStatus::LoggedIn {
            source,
            subject: None,
            expires_in: None,
        },
    };

    ctx.present(AuthStatusView { status: &status }, &status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::RenderExt;

    #[test]
    fn device_id_is_private_and_stable() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state").join("device-id");
        let first = load_or_create_device_id_at(&path).unwrap();
        assert_eq!(first, load_or_create_device_id_at(&path).unwrap());
        assert!(first.len() >= 16);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    fn render(status: &AuthStatus) -> String {
        AuthStatusView { status }.display().to_string()
    }

    #[test]
    fn status_text_logged_in() {
        let status = AuthStatus::LoggedIn {
            source: "config".into(),
            subject: Some("user@example.com".into()),
            expires_in: Some(9000),
        };

        assert_eq!(
            render(&status),
            concat!(
                "✓ Logged in (config)\n",
                "  user        user@example.com\n",
                "  expires in  2h 30m\n",
            )
        );
    }

    #[test]
    fn status_text_expired() {
        let status = AuthStatus::Expired {
            source: "config".into(),
            subject: Some("admin".into()),
        };

        assert_eq!(
            render(&status),
            concat!("✗ Token expired (config)\n", "  user  admin\n",)
        );
    }

    #[test]
    fn status_text_not_logged_in() {
        let status = AuthStatus::NotLoggedIn;

        assert_eq!(render(&status), "✗ Not logged in\n");
    }

    #[test]
    fn status_json_logged_in() {
        let status = AuthStatus::LoggedIn {
            source: "config".into(),
            subject: Some("user@example.com".into()),
            expires_in: Some(9000),
        };

        let json: serde_json::Value = serde_json::to_value(&status).unwrap();
        assert_eq!(json["status"], "logged_in");
        assert_eq!(json["source"], "config");
        assert_eq!(json["subject"], "user@example.com");
        assert_eq!(json["expires_in"], 9000);
    }

    #[test]
    fn status_json_expired() {
        let status = AuthStatus::Expired {
            source: "config".into(),
            subject: Some("admin".into()),
        };

        let json: serde_json::Value = serde_json::to_value(&status).unwrap();
        assert_eq!(json["status"], "expired");
        assert_eq!(json["source"], "config");
        assert_eq!(json["subject"], "admin");
        assert!(json.get("expires_in").is_none());
    }

    #[test]
    fn status_json_not_logged_in() {
        let status = AuthStatus::NotLoggedIn;

        let json: serde_json::Value = serde_json::to_value(&status).unwrap();
        assert_eq!(json["status"], "not_logged_in");
        assert!(json.get("source").is_none());
        assert!(json.get("subject").is_none());
        assert!(json.get("expires_in").is_none());
    }
}
