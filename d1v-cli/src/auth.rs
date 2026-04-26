use std::io::{stdin, IsTerminal};

use secrecy::SecretString;
use serde::Serialize;
use tracing::debug;

use crate::error::Result;
use crate::output::format_duration;
use crate::text::{Field, Fields, Line, Render, RenderContext, Span, Text};
use crate::token::{TokenLoader, TokenStore};
use crate::ui::{Password, Select, SelectOption};
use crate::{i18n, prompt, symbols, t, theme, Context};

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

/// Presents an interactive menu to choose a login method, then executes it.
pub async fn login_interactive(ctx: &Context) -> Result<()> {
    enum Method {
        Code,
        Password,
        Token,
    }

    let method = Select::new(t!("auth-method-prompt"))
        .option(SelectOption::new(Method::Code, t!("auth-method-code")))
        .option(SelectOption::new(
            Method::Password,
            t!("auth-method-password"),
        ))
        .option(SelectOption::new(Method::Token, t!("auth-method-token")))
        .default_index(0)
        .prompt()?;

    match method {
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

struct AuthStatusView<'a> {
    status: &'a AuthStatus,
}

impl Render for AuthStatusView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        let auth_status = self.status;
        let (symbol, title, style) = if !auth_status.logged_in {
            (
                symbols::ERROR,
                t!("auth-status-not-logged-in"),
                theme::ansi::error(),
            )
        } else if auth_status.expired == Some(true) {
            (
                symbols::ERROR,
                t!("auth-status-expired"),
                theme::ansi::error(),
            )
        } else {
            (
                symbols::SUCCESS,
                t!("auth-status-logged-in"),
                theme::ansi::success(),
            )
        };

        let line = Line::styled(symbol, style)
            .push_plain(" ")
            .push_styled(title, style);

        let line = if let Some(source) = &auth_status.source {
            line.push_plain(" ")
                .push_styled(format!("({source})"), theme::ansi::dim())
        } else {
            line
        };

        let mut fields = Vec::new();
        if let Some(subject) = &auth_status.subject {
            fields.push(Field::new(
                Span::styled(t!("auth-status-label-user"), theme::ansi::label()),
                Span::styled(subject.clone(), theme::ansi::value()),
            ));
        }

        if let Some(secs) = auth_status.expires_in {
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
    let source = ctx.tokens.source().map(String::from);
    let logged_in = source.is_some();

    let (subject, expires_in, expired) = if logged_in && let Some(claims) = ctx.client.claims() {
        let expires_in = claims.expires_in().map(|d| d.as_secs());
        let expired = claims.is_expired();
        (claims.subject, expires_in, Some(expired))
    } else {
        (None, None, None)
    };

    let status = AuthStatus {
        logged_in,
        source,
        subject,
        expires_in,
        expired,
    };

    ctx.present(AuthStatusView { status: &status }, &status)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(status: &AuthStatus) -> String {
        let mut buf = Vec::new();
        let mut ctx = RenderContext::new(&mut buf, false);
        AuthStatusView { status }.render(&mut ctx).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn status_text_logged_in() {
        let status = AuthStatus {
            logged_in: true,
            source: Some("keyring".into()),
            subject: Some("user@example.com".into()),
            expires_in: Some(9000),
            expired: Some(false),
        };

        assert_eq!(
            render(&status),
            concat!(
                "✓ Logged in (keyring)\n",
                "  user        user@example.com\n",
                "  expires in  2h 30m\n",
            )
        );
    }

    #[test]
    fn status_text_expired() {
        let status = AuthStatus {
            logged_in: true,
            source: Some("config".into()),
            subject: Some("admin".into()),
            expires_in: None,
            expired: Some(true),
        };

        assert_eq!(
            render(&status),
            concat!("✗ Token expired (config)\n", "  user  admin\n",)
        );
    }

    #[test]
    fn status_text_not_logged_in() {
        let status = AuthStatus {
            logged_in: false,
            source: None,
            subject: None,
            expires_in: None,
            expired: Some(false),
        };

        assert_eq!(render(&status), "✗ Not logged in\n");
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
