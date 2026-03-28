use garde::Validate;
use serde::Serialize;

#[derive(Debug, Copy, Clone, Serialize, Validate)]
#[serde(transparent)]
#[garde(transparent)]
pub struct Email<'a>(#[garde(email)] pub &'a str);

#[derive(Debug, Copy, Clone, Serialize, Validate)]
#[serde(transparent)]
#[garde(transparent)]
pub struct Code<'a>(#[garde(pattern(r"^[0-9]{6}$"))] pub &'a str);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_valid() {
        assert!(Email("user@example.com").validate().is_ok());
        assert!(Email("a@b.c").validate().is_ok());
    }

    #[test]
    fn email_invalid() {
        assert!(Email("").validate().is_err());
        assert!(Email("not-an-email").validate().is_err());
        assert!(Email("@missing-local.com").validate().is_err());
    }

    #[test]
    fn code_valid() {
        assert!(Code("123456").validate().is_ok());
        assert!(Code("000000").validate().is_ok());
    }

    #[test]
    fn code_invalid() {
        assert!(Code("").validate().is_err());
        assert!(Code("12345").validate().is_err());
        assert!(Code("1234567").validate().is_err());
        assert!(Code("abcdef").validate().is_err());
        assert!(Code("12 456").validate().is_err());
    }
}
