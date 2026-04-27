use std::io::{stdin, IsTerminal};

use anstyle::Style;
use secrecy::SecretString;
use serde::Serialize;
use tracing::debug;

use crate::error::Result;
use crate::output::format_duration;
use crate::text::{Field, Fields, Line, Render, RenderContext, Span, Text};
use crate::token::{TokenSource, TokenStore};
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

    fn render(status: &AuthStatus) -> String {
        AuthStatusView { status }.display().to_string()
    }

    #[test]
    fn status_text_logged_in() {
        let status = AuthStatus::LoggedIn {
            source: "keyring".into(),
            subject: Some("user@example.com".into()),
            expires_in: Some(9000),
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
            source: "keyring".into(),
            subject: Some("user@example.com".into()),
            expires_in: Some(9000),
        };

        let json: serde_json::Value = serde_json::to_value(&status).unwrap();
        assert_eq!(json["status"], "logged_in");
        assert_eq!(json["source"], "keyring");
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
