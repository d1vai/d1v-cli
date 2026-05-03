use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{Loader, static_loader};
use parking_lot::RwLock;
use tracing::debug;
use unic_langid::{LanguageIdentifier, langid};

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

static LOCALE: RwLock<LanguageIdentifier> = RwLock::new(langid!("en"));

pub fn locale() -> LanguageIdentifier {
    LOCALE.read().clone()
}

pub fn init(locales: impl IntoIterator<Item = impl AsRef<str>>) {
    let locale = LocaleResolver::new().resolve(locales);
    debug!(%locale, "i18n initialized");
    *LOCALE.write() = locale;
}

struct LocaleResolver {
    available: Vec<LanguageIdentifier>,
}

impl LocaleResolver {
    fn new() -> Self {
        Self {
            available: LOCALES.locales().cloned().collect(),
        }
    }

    fn resolve(&self, locales: impl IntoIterator<Item = impl AsRef<str>>) -> LanguageIdentifier {
        locales
            .into_iter()
            .filter_map(|s| s.as_ref().parse::<LanguageIdentifier>().ok())
            .find_map(|id| self.find_best(&id))
            .unwrap_or_else(|| langid!("en"))
    }

    fn find_best(&self, requested: &LanguageIdentifier) -> Option<LanguageIdentifier> {
        self.available
            .iter()
            .find(|&id| id == requested)
            .or_else(|| {
                self.available
                    .iter()
                    .find(|id| id.language == requested.language)
            })
            .cloned()
    }
}

fn fluent_args<'a>(
    args: &'a [(&'static str, String)],
) -> Option<HashMap<Cow<'static, str>, FluentValue<'a>>> {
    (!args.is_empty()).then(|| {
        args.iter()
            .map(|(k, v)| (Cow::Borrowed(*k), FluentValue::from(v.as_str())))
            .collect()
    })
}

pub fn lookup(id: &str, args: &[(&'static str, String)]) -> String {
    LOCALES
        .try_lookup_complete(&locale(), id, fluent_args(args).as_ref())
        .unwrap_or_else(|| id.to_string())
}

#[macro_export]
macro_rules! t {
    ($id:expr) => {
        $crate::i18n::lookup($id, &[])
    };
    ($id:expr, $($key:ident = $val:expr),+ $(,)?) => {{
        $crate::i18n::lookup($id, &[
            $( (stringify!($key), $val.to_string()), )+
        ])
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup_lang(lang: &LanguageIdentifier, id: &str, args: &[(&'static str, String)]) -> String {
        LOCALES
            .try_lookup_complete(lang, id, fluent_args(args).as_ref())
            .unwrap_or_else(|| id.to_string())
    }

    #[test]
    fn en() {
        assert_eq!(
            lookup_lang(&langid!("en"), "auth-login-success", &[]),
            "Login successful!"
        );
    }

    #[test]
    fn zh() {
        assert_eq!(
            lookup_lang(&langid!("zh-Hans"), "auth-login-success", &[]),
            "登录成功！"
        );
    }

    #[test]
    fn en_args() {
        assert_eq!(
            lookup_lang(
                &langid!("en"),
                "auth-code-sent",
                &[("email", "test@example.com".into())]
            ),
            "Verification code sent to test@example.com"
        );
    }

    #[test]
    fn zh_args() {
        assert_eq!(
            lookup_lang(
                &langid!("zh-Hans"),
                "auth-code-sent",
                &[("email", "test@example.com".into())]
            ),
            "验证码已发送至 test@example.com"
        );
    }

    #[test]
    fn fallback() {
        assert_eq!(
            lookup_lang(&langid!("fr"), "auth-login-success", &[]),
            "Login successful!"
        );
    }

    #[test]
    fn missing_key() {
        assert_eq!(
            lookup_lang(&langid!("en"), "nonexistent-key", &[]),
            "nonexistent-key"
        );
    }

    #[test]
    fn resolve_default() {
        let resolver = LocaleResolver::new();
        assert_eq!(resolver.resolve(std::iter::empty::<&str>()), langid!("en"));
    }

    #[test]
    fn resolve_override() {
        let resolver = LocaleResolver::new();
        assert_eq!(resolver.resolve(["zh-Hans"]).language.as_str(), "zh");
    }

    #[test]
    fn resolve_priority() {
        let resolver = LocaleResolver::new();
        assert_eq!(resolver.resolve(["en", "zh-Hans"]).language.as_str(), "en");
    }
}
