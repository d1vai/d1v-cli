use garde::rules::email;
use serde::Serialize;

pub trait Validate {
    type Error: std::error::Error;

    fn validate(&self) -> Result<(), Self::Error>;
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(transparent)]
pub struct Email<'a>(pub &'a str);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmailError {
    #[error("email is empty")]
    Empty,
    #[error("invalid email address")]
    Invalid,
}

impl Validate for Email<'_> {
    type Error = EmailError;

    fn validate(&self) -> Result<(), Self::Error> {
        email::parse_email(self.0).map_err(|e| match e {
            email::InvalidEmail::Empty => EmailError::Empty,
            _ => EmailError::Invalid,
        })
    }
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(transparent)]
pub struct Code<'a>(pub &'a str);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodeError {
    #[error("code is empty")]
    Empty,
    #[error("code must be 6 digits")]
    InvalidLength,
    #[error("code must contain only digits")]
    NonDigit,
}

impl Validate for Code<'_> {
    type Error = CodeError;

    fn validate(&self) -> Result<(), Self::Error> {
        let s = self.0;
        if s.is_empty() {
            return Err(CodeError::Empty);
        }
        if s.len() != 6 {
            return Err(CodeError::InvalidLength);
        }
        if !s.chars().all(|c| c.is_ascii_digit()) {
            return Err(CodeError::NonDigit);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlError {
    #[error("invalid URL")]
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error(transparent)]
    Email(#[from] EmailError),
    #[error(transparent)]
    Code(#[from] CodeError),
    #[error(transparent)]
    Url(#[from] UrlError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_email() {
        assert!(Email("user@example.com").validate().is_ok());
        assert!(Email("a@b.c").validate().is_ok());
        assert_eq!(Email("").validate().unwrap_err(), EmailError::Empty);
        assert_eq!(
            Email("not-an-email").validate().unwrap_err(),
            EmailError::Invalid
        );
        assert_eq!(
            Email("@missing-local.com").validate().unwrap_err(),
            EmailError::Invalid
        );
    }

    #[test]
    fn validate_code() {
        assert!(Code("123456").validate().is_ok());
        assert!(Code("000000").validate().is_ok());
        assert_eq!(Code("").validate().unwrap_err(), CodeError::Empty);
        assert_eq!(
            Code("12345").validate().unwrap_err(),
            CodeError::InvalidLength
        );
        assert_eq!(Code("abcdef").validate().unwrap_err(), CodeError::NonDigit);
        assert!(Code("12 456").validate().is_err());
    }
}
