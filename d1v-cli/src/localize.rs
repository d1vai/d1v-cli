use d1v_api::{ApiCode, BadRequestKind, CodeError, EmailError, UrlError, ValidationError};

use crate::config::ConfigError;
use crate::error::{APIError, Error};
use crate::t;
use crate::token::TokenError;

/// Produces a localized, user-facing description of an error.
pub trait Localize {
    fn localize(&self) -> String;
}

impl Localize for Error {
    fn localize(&self) -> String {
        match self {
            Self::NotLoggedIn => t!("error-not-logged-in"),
            Self::TokenExpired => t!("error-token-expired"),
            Self::Canceled => t!("canceled"),
            Self::Interrupted => t!("interrupted"),
            Self::Config(err) => err.localize(),
            Self::Token(err) => err.localize(),
            Self::Api(err) => err.localize(),
            Self::InvalidBaseUrl { url, .. } => t!(
                "error-invalid-base-url",
                source = url.source().to_string(),
                value = url.as_str()
            ),
            Self::Io(_) | Self::Other(_) => format!("{self:#}"),
        }
    }
}

impl Localize for ApiCode {
    fn localize(&self) -> String {
        match self {
            ApiCode::BadRequest => t!("api-error-bad-request"),
            ApiCode::Unknown(code) => t!("api-error-unknown-code", code = code),
            _ => t!("api-error-unknown-code", code = self.raw()),
        }
    }
}

impl Localize for BadRequestKind {
    fn localize(&self) -> String {
        match self {
            BadRequestKind::PasswordNotSet => t!("api-error-password-not-set"),
            BadRequestKind::InvalidCredentials => t!("api-error-invalid-credentials"),
            BadRequestKind::EmailRequiredBeforePassword => {
                t!("api-error-email-required-before-password")
            }
            BadRequestKind::InvalidVerifyCode => t!("api-error-invalid-code"),
            BadRequestKind::VerifyCodeExpired => t!("api-error-code-expired"),
            BadRequestKind::InvalidOrExpiredCode => t!("api-error-code-invalid-or-expired"),
            BadRequestKind::UserNotFound => t!("api-error-user-not-found"),
            BadRequestKind::PasswordTooShort => t!("api-error-password-too-short"),
            BadRequestKind::EmailAlreadyInUse => t!("api-error-email-in-use"),
            BadRequestKind::EmailNotBound => t!("api-error-email-not-bound"),
            BadRequestKind::CannotAcceptOwnInviteCode => t!("api-error-invite-own-code"),
            BadRequestKind::InvalidInviteCode => t!("api-error-invite-invalid"),
            BadRequestKind::InviteCodeExpired => t!("api-error-invite-expired"),
            BadRequestKind::InviteCodeCapacityReached => t!("api-error-invite-capacity"),
            BadRequestKind::InviteLimitReached => t!("api-error-invite-limit"),
            BadRequestKind::InviteCodeNotBoundToInviter => t!("api-error-invite-not-bound"),
            BadRequestKind::InviterNotFound => t!("api-error-inviter-not-found"),
            _ => t!("api-error-bad-request"),
        }
    }
}

impl Localize for APIError {
    fn localize(&self) -> String {
        match self {
            Self::Http(err) if err.is_timeout() => t!("error-timeout"),
            Self::Http(err) if err.is_connect() => t!("error-connection-failed"),
            Self::Http(_) => t!("error-network"),
            Self::HttpStatus(_) => t!("error-http-status"),
            Self::Data(_) => t!("error-invalid-response"),
            Self::Url(_) => t!("error-invalid-url"),
            Self::ServerValidation(_) => t!("error-server-validation"),
            Self::Validation(err) => err.localize(),
            Self::Api {
                code: ApiCode::BadRequest,
                message,
            } => match BadRequestKind::from_message(message) {
                Some(kind) => kind.localize(),
                None if !message.is_empty() => t!("api-error-bad-request-message", message = message),
                None => t!("api-error-bad-request"),
            },
            Self::Api { code, message } if let ApiCode::Unknown(_) = code => {
                t!("api-error-unknown", code = code.raw(), message = message)
            }
            Self::Api { code, .. } => code.localize(),
            Self::TokenExpired => t!("error-token-expired"),
        }
    }
}

impl Localize for EmailError {
    fn localize(&self) -> String {
        match self {
            Self::Empty => t!("validation-email-required"),
            Self::Invalid => t!("validation-email-invalid"),
        }
    }
}

impl Localize for CodeError {
    fn localize(&self) -> String {
        match self {
            Self::Empty => t!("validation-code-required"),
            Self::InvalidLength => t!("validation-code-length"),
            Self::NonDigit => t!("validation-code-digit"),
        }
    }
}

impl Localize for UrlError {
    fn localize(&self) -> String {
        match self {
            Self::Invalid => t!("validation-url-invalid"),
        }
    }
}

impl Localize for ValidationError {
    fn localize(&self) -> String {
        match self {
            Self::Email(err) => err.localize(),
            Self::Code(err) => err.localize(),
            Self::Url(err) => err.localize(),
        }
    }
}

impl Localize for ConfigError {
    fn localize(&self) -> String {
        match self {
            Self::NoHomeDir => t!("error-no-home-dir"),
            Self::Read(_) => t!("error-read-config"),
            Self::Write(_) => t!("error-write-config"),
            Self::Parse(_) => t!("error-parse-config"),
            Self::Serialize(_) => t!("error-serialize-config"),
            Self::InvalidValue { key, value } => {
                t!("error-invalid-config-value", key = key, value = value)
            }
            Self::Open(_) => t!("config-edit-failed"),
        }
    }
}

impl Localize for TokenError {
    fn localize(&self) -> String {
        match self {
            Self::KeyringUnavailable => t!("error-keyring-unavailable"),
            Self::KeyringLoad(_) => t!("error-keyring-load"),
            Self::KeyringSave(_) => t!("error-keyring-save"),
            Self::KeyringDelete(_) => t!("error-keyring-delete"),
            Self::NoStore => t!("error-no-token-store"),
            Self::Config(err) => err.localize(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localize_cli_errors() {
        assert_eq!(Error::NotLoggedIn.localize(), "not logged in");
        assert_eq!(Error::TokenExpired.localize(), "token has expired");
        assert_eq!(Error::Canceled.localize(), "Cancelled.");
        assert_eq!(Error::Interrupted.localize(), "Interrupted.");
    }

    #[test]
    fn localize_api_code() {
        assert_eq!(ApiCode::BadRequest.localize(), "bad request");
        assert_eq!(ApiCode::Unknown(99999).localize(), "server error 99999");
    }

    #[test]
    fn localize_api_error() {
        let err = APIError::Api {
            code: ApiCode::BadRequest,
            message: "password not set".into(),
        };
        assert_eq!(err.localize(), "password not set");

        let err = APIError::Api {
            code: ApiCode::BadRequest,
            message: "invalid email or password".into(),
        };
        assert_eq!(err.localize(), "invalid email or password");

        let err = APIError::Api {
            code: ApiCode::BadRequest,
            message: "something changed".into(),
        };
        assert_eq!(err.localize(), "bad request (something changed)");

        assert_eq!(APIError::TokenExpired.localize(), "token has expired");
    }

    #[test]
    fn localize_validation_errors() {
        assert_eq!(EmailError::Empty.localize(), "email address is required");
        assert_eq!(EmailError::Invalid.localize(), "invalid email address");
        assert_eq!(CodeError::Empty.localize(), "verification code is required");
        assert_eq!(
            CodeError::InvalidLength.localize(),
            "verification code must be 6 digits"
        );
        assert_eq!(
            CodeError::NonDigit.localize(),
            "verification code must contain only digits"
        );
        assert_eq!(UrlError::Invalid.localize(), "invalid URL");
    }

    #[test]
    fn localize_validation_error_delegates() {
        let err = ValidationError::Email(EmailError::Empty);
        assert_eq!(err.localize(), EmailError::Empty.localize());

        let err = ValidationError::Code(CodeError::InvalidLength);
        assert_eq!(err.localize(), CodeError::InvalidLength.localize());

        let err = ValidationError::Url(UrlError::Invalid);
        assert_eq!(err.localize(), UrlError::Invalid.localize());
    }

    #[test]
    fn localize_config_error() {
        assert_eq!(
            ConfigError::NoHomeDir.localize(),
            "could not determine home directory"
        );
    }

    #[test]
    fn localize_token_error() {
        assert_eq!(
            TokenError::KeyringUnavailable.localize(),
            "keyring is not available"
        );
        assert_eq!(
            TokenError::KeyringLoad(keyring_core::Error::NoEntry).localize(),
            "failed to load from keyring"
        );
        assert_eq!(
            TokenError::KeyringSave(keyring_core::Error::NoEntry).localize(),
            "failed to save to keyring"
        );
        assert_eq!(
            TokenError::KeyringDelete(keyring_core::Error::NoEntry).localize(),
            "failed to delete from keyring"
        );
        assert_eq!(
            TokenError::NoStore.localize(),
            "no writable token store available"
        );
        assert_eq!(
            TokenError::Config(ConfigError::NoHomeDir).localize(),
            ConfigError::NoHomeDir.localize()
        );
    }
}
