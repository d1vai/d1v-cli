use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{static_loader, Loader};
use parking_lot::RwLock;
use tracing::debug;
use unic_langid::{langid, LanguageIdentifier};

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

pub fn lookup(id: &str, args: &[(&'static str, String)]) -> String {
    let map = (!args.is_empty()).then(|| {
        args.iter()
            .map(|(k, v)| (Cow::Borrowed(*k), FluentValue::from(v.as_str())))
            .collect()
    });

    LOCALES
        .try_lookup_complete(&locale(), id, map.as_ref())
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
