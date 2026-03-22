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

    fn resolve(&self, sources: impl IntoIterator<Item = impl AsRef<str>>) -> LanguageIdentifier {
        sources
            .into_iter()
            .filter_map(|s| s.as_ref().parse::<LanguageIdentifier>().ok())
            .find_map(|id| self.find_best(&id))
            .unwrap_or_else(|| langid!("en"))
    }

    fn find_best(&self, requested: impl AsRef<LanguageIdentifier>) -> Option<LanguageIdentifier> {
        let requested = requested.as_ref();

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
