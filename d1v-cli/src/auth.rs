use std::fmt;
use std::fmt::{Display, Formatter};
use std::io::{stdin, IsTerminal};

use owo_colors::{OwoColorize, Stream};
use secrecy::SecretString;
use serde::Serialize;
use tracing::debug;

use crate::error::{Error, Result};
use crate::localize::Localize;
use crate::output::{format_duration, pad_label};
use crate::token::{TokenLoader, TokenStore};
use crate::ui::{Confirm, Password};
use crate::{i18n, prompt, symbols, t, Context};

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
    Ok(pending
        .spin_ok(ctx.client.user().login(&email, &code))
        .await?)
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

pub async fn logout(ctx: &Context) -> Result<()> {
    if ctx.tokens.load()?.is_none() {
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
    ctx.output.error(&Error::TokenExpired.localize());

    let confirmed = Confirm::new(t!("auth-relogin-prompt"))
        .default(true)
        .prompt()?;

    if !confirmed {
        return Ok(false);
    }

    login(ctx, false).await?;
    Ok(true)
}

#[derive(Debug, Serialize)]
struct AuthStatus {
    logged_in: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expired: Option<bool>,
}

impl Display for AuthStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if !self.logged_in {
            return write!(
                f,
                "{} {}",
                symbols::ERROR.if_supports_color(Stream::Stdout, |s| s.bright_red()),
                t!("auth-status-not-logged-in")
                    .if_supports_color(Stream::Stdout, |s| s.bright_red()),
            );
        }

        if self.expired == Some(true) {
            write!(
                f,
                "{} {}",
                symbols::ERROR.if_supports_color(Stream::Stdout, |s| s.bright_red()),
                t!("auth-status-expired").if_supports_color(Stream::Stdout, |s| s.bright_red()),
            )?;
        } else {
            write!(
                f,
                "{} {}",
                symbols::SUCCESS.if_supports_color(Stream::Stdout, |s| s.green()),
                t!("auth-status-logged-in").if_supports_color(Stream::Stdout, |s| s.green()),
            )?;
        }

        if let Some(source) = &self.source {
            let s = format!("({source})");
            write!(
                f,
                " {}",
                s.if_supports_color(Stream::Stdout, |s| s.dimmed())
            )?;
        }

        if let Some(subject) = &self.subject {
            write!(
                f,
                "\n  {}{}",
                pad_label(t!("auth-status-label-user"), 12),
                subject.if_supports_color(Stream::Stdout, |s| s.cyan()),
            )?;
        }

        if let Some(secs) = self.expires_in {
            write!(
                f,
                "\n  {}{}",
                pad_label(t!("auth-status-label-expires"), 12),
                format_duration(secs).if_supports_color(Stream::Stdout, |s| s.cyan()),
            )?;
        }

        Ok(())
    }
}

pub fn status(ctx: &Context) -> Result<()> {
    let source = ctx.tokens.source().map(String::from);
    let logged_in = source.is_some();

    let (subject, expires_in, expired) = if logged_in && let Some(claims) = ctx.client.claims() {
        let expires_in = claims.expires_in().map(|d| d.as_secs());
        let expired = claims.is_expired();
        (claims.subject, expires_in, Some(expired))
    } else {
        (None, None, None)
    };

    ctx.print(&AuthStatus {
        logged_in,
        source,
        subject,
        expires_in,
        expired,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_display_logged_in() {
        let status = AuthStatus {
            logged_in: true,
            source: Some("keyring".into()),
            subject: Some("user@example.com".into()),
            expires_in: Some(9000),
            expired: Some(false),
        };

        assert_eq!(
            status.to_string(),
            concat!(
                "✓ Logged in (keyring)\n",
                "  user:       user@example.com\n",
                "  expires in: 2h 30m",
            )
        );
    }

    #[test]
    fn status_display_expired() {
        let status = AuthStatus {
            logged_in: true,
            source: Some("config".into()),
            subject: Some("admin".into()),
            expires_in: None,
            expired: Some(true),
        };

        assert_eq!(
            status.to_string(),
            concat!("✗ Token expired (config)\n", "  user:       admin",)
        );
    }

    #[test]
    fn status_display_not_logged_in() {
        let status = AuthStatus {
            logged_in: false,
            source: None,
            subject: None,
            expires_in: None,
            expired: Some(false),
        };

        assert_eq!(status.to_string(), "✗ Not logged in");
    }

    #[test]
    fn status_json_logged_in() {
        let status = AuthStatus {
            logged_in: true,
            source: Some("keyring".into()),
            subject: Some("user@example.com".into()),
            expires_in: Some(9000),
            expired: Some(false),
        };

        let json: serde_json::Value = serde_json::to_value(&status).unwrap();
        assert_eq!(json["logged_in"], true);
        assert_eq!(json["source"], "keyring");
        assert_eq!(json["subject"], "user@example.com");
        assert_eq!(json["expires_in"], 9000);
        assert_eq!(json["expired"], false);
    }

    #[test]
    fn status_json_not_logged_in() {
        let status = AuthStatus {
            logged_in: false,
            source: None,
            subject: None,
            expires_in: None,
            expired: None,
        };

        let json: serde_json::Value = serde_json::to_value(&status).unwrap();
        assert_eq!(json["logged_in"], false);
        assert!(json.get("source").is_none());
        assert!(json.get("subject").is_none());
        assert!(json.get("expires_in").is_none());
        assert!(json.get("expired").is_none());
    }
}
