use std::fmt;
use std::fmt::{Display, Formatter};

/// A `User-Agent` header value.
///
/// # Format
///
/// ```text
/// {product}/{version} ({os_type} {os_version}; {arch}) {lang}/{lang_version}
/// ```
///
/// # Examples
///
/// ```
/// use d1v_api::UserAgent;
///
/// // Rust (default)
/// let ua = UserAgent::new("d1v-cli", "0.1.0");
/// assert!(ua.to_string().starts_with("d1v-cli/0.1.0 ("));
///
/// // Python binding
/// let ua = UserAgent::new("d1v-api", "0.1.0").lang("python", "3.14.3");
/// assert!(ua.to_string().ends_with("python/3.14.3"));
/// ```
#[derive(Debug, Clone)]
pub struct UserAgent {
    product: String,
    version: String,
    os_type: String,
    os_version: String,
    arch: &'static str,
    lang: String,
    lang_version: String,
}

impl UserAgent {
    /// Creates a [`UserAgent`] for the given product, defaulting to Rust.
    pub fn new(product: impl Into<String>, version: impl Into<String>) -> Self {
        let info = os_info::get();

        Self {
            product: product.into(),
            version: version.into(),
            os_type: info.os_type().to_string(),
            os_version: info.version().to_string(),
            arch: std::env::consts::ARCH,
            lang: "rust".into(),
            lang_version: env!("D1V_RUSTC_VERSION").into(),
        }
    }

    /// Overrides the language and its version.
    #[must_use]
    pub fn lang(mut self, lang: impl Into<String>, version: impl Into<String>) -> Self {
        self.lang = lang.into();
        self.lang_version = version.into();

        self
    }
}

impl Display for UserAgent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self {
            product,
            version,
            os_type,
            os_version,
            arch,
            lang,
            lang_version,
        } = self;

        write!(
            f,
            "{product}/{version} ({os_type} {os_version}; {arch}) {lang}/{lang_version}"
        )
    }
}

impl From<UserAgent> for String {
    fn from(ua: UserAgent) -> Self {
        ua.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn print() {
        println!("{}", UserAgent::new("d1v-api", "0.1.0"));
        println!("{}", UserAgent::new("d1v-cli", "0.1.0"));
    }

    #[test]
    fn default_format() {
        let ua = UserAgent::new("d1v-cli", "0.1.0").to_string();

        assert!(ua.starts_with("d1v-cli/0.1.0 ("), "{ua}");
        assert!(ua.contains(std::env::consts::ARCH), "{ua}");
        assert!(
            ua.ends_with(&format!("rust/{}", env!("D1V_RUSTC_VERSION"))),
            "{ua}"
        );
    }

    #[test]
    fn override_lang() {
        let ua = UserAgent::new("d1v-api", "0.1.0")
            .lang("python", "3.14.3")
            .to_string();

        assert!(ua.starts_with("d1v-api/0.1.0 ("), "{ua}");
        assert!(ua.ends_with("python/3.14.3"), "{ua}");
    }
}
