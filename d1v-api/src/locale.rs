use std::fmt;
use std::str::FromStr;

use serde::Serialize;
use strum::{AsRefStr, EnumCount, EnumIter, VariantArray};
use unic_langid::LanguageIdentifier;

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, AsRefStr, EnumCount, EnumIter, VariantArray,
)]
pub enum Locale {
    #[strum(serialize = "en")]
    #[serde(rename = "en")]
    English,

    #[strum(serialize = "zh-CN")]
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,

    #[strum(serialize = "zh-TW")]
    #[serde(rename = "zh-TW")]
    TraditionalChinese,

    #[strum(serialize = "es")]
    #[serde(rename = "es")]
    Spanish,

    #[strum(serialize = "fr")]
    #[serde(rename = "fr")]
    French,

    #[strum(serialize = "de")]
    #[serde(rename = "de")]
    German,

    #[strum(serialize = "pt-BR")]
    #[serde(rename = "pt-BR")]
    BrazilianPortuguese,

    #[strum(serialize = "pt-PT")]
    #[serde(rename = "pt-PT")]
    Portuguese,

    #[strum(serialize = "ja")]
    #[serde(rename = "ja")]
    Japanese,

    #[strum(serialize = "ko")]
    #[serde(rename = "ko")]
    Korean,

    #[strum(serialize = "ru")]
    #[serde(rename = "ru")]
    Russian,

    #[strum(serialize = "ar")]
    #[serde(rename = "ar")]
    Arabic,

    #[strum(serialize = "hi")]
    #[serde(rename = "hi")]
    Hindi,

    #[strum(serialize = "id")]
    #[serde(rename = "id")]
    Indonesian,

    #[strum(serialize = "th")]
    #[serde(rename = "th")]
    Thai,

    #[strum(serialize = "vi")]
    #[serde(rename = "vi")]
    Vietnamese,

    #[strum(serialize = "tr")]
    #[serde(rename = "tr")]
    Turkish,

    #[strum(serialize = "it")]
    #[serde(rename = "it")]
    Italian,

    #[strum(serialize = "nl")]
    #[serde(rename = "nl")]
    Dutch,

    #[strum(serialize = "pl")]
    #[serde(rename = "pl")]
    Polish,

    #[strum(serialize = "sv")]
    #[serde(rename = "sv")]
    Swedish,

    #[strum(serialize = "cs")]
    #[serde(rename = "cs")]
    Czech,

    #[strum(serialize = "he")]
    #[serde(rename = "he")]
    Hebrew,

    #[strum(serialize = "uk")]
    #[serde(rename = "uk")]
    Ukrainian,

    #[strum(serialize = "da")]
    #[serde(rename = "da")]
    Danish,

    #[strum(serialize = "nb")]
    #[serde(rename = "nb")]
    NorwegianBokmal,

    #[strum(serialize = "fi")]
    #[serde(rename = "fi")]
    Finnish,

    #[strum(serialize = "ro")]
    #[serde(rename = "ro")]
    Romanian,

    #[strum(serialize = "hu")]
    #[serde(rename = "hu")]
    Hungarian,

    #[strum(serialize = "el")]
    #[serde(rename = "el")]
    Greek,

    #[strum(serialize = "bg")]
    #[serde(rename = "bg")]
    Bulgarian,

    #[strum(serialize = "fa")]
    #[serde(rename = "fa")]
    Persian,

    #[strum(serialize = "bn")]
    #[serde(rename = "bn")]
    Bengali,

    #[strum(serialize = "ms")]
    #[serde(rename = "ms")]
    Malay,

    #[strum(serialize = "fil")]
    #[serde(rename = "fil")]
    Filipino,
}

impl Locale {
    /// Returns the native display name (e.g. "简体中文", "English").
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
            Self::TraditionalChinese => "繁體中文",
            Self::Spanish => "Español",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::BrazilianPortuguese => "Português (Brasil)",
            Self::Portuguese => "Português (Portugal)",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::Russian => "Русский",
            Self::Arabic => "العربية",
            Self::Hindi => "हिन्दी",
            Self::Indonesian => "Bahasa Indonesia",
            Self::Thai => "ไทย",
            Self::Vietnamese => "Tiếng Việt",
            Self::Turkish => "Türkçe",
            Self::Italian => "Italiano",
            Self::Dutch => "Nederlands",
            Self::Polish => "Polski",
            Self::Swedish => "Svenska",
            Self::Czech => "Čeština",
            Self::Hebrew => "עברית",
            Self::Ukrainian => "Українська",
            Self::Danish => "Dansk",
            Self::NorwegianBokmal => "Norsk (Bokmål)",
            Self::Finnish => "Suomi",
            Self::Romanian => "Română",
            Self::Hungarian => "Magyar",
            Self::Greek => "Ελληνικά",
            Self::Bulgarian => "Български",
            Self::Persian => "فارسی",
            Self::Bengali => "বাংলা",
            Self::Malay => "Bahasa Melayu",
            Self::Filipino => "Filipino",
        }
    }

    /// Returns the parsed [`LanguageIdentifier`] for this locale.
    fn lang_id(&self) -> LanguageIdentifier {
        self.as_ref().parse().unwrap()
    }

    /// Resolves a BCP-47 locale tag to the closest server-supported locale.
    ///
    /// Parses `tag` as a [`LanguageIdentifier`] and matches against the
    /// supported variants: exact match first, then Chinese script mapping
    /// (`zh-Hans` → [`SimplifiedChinese`](Self::SimplifiedChinese)),
    /// and finally language-only fallback (`en-US` → [`English`](Self::English)).
    ///
    /// Returns `None` if `tag` is not a valid BCP-47 tag or no match is found.
    pub fn resolve(tag: impl AsRef<str>) -> Option<Self> {
        let id: LanguageIdentifier = tag.as_ref().parse().ok()?;
        Self::resolve_id(&id)
    }

    pub fn resolve_id(id: &LanguageIdentifier) -> Option<Self> {
        if let Some(locale) = Self::VARIANTS
            .iter()
            .find(|&locale| locale.lang_id() == *id)
        {
            return Some(*locale);
        }

        if id.language.as_str() == "zh" {
            if let Some(script) = id.script {
                return if script.as_str() == "Hant" {
                    Some(Locale::TraditionalChinese)
                } else {
                    Some(Locale::SimplifiedChinese)
                };
            }

            return if let Some(region) = id.region
                && matches!(region.as_str(), "TW" | "HK" | "MO")
            {
                Some(Self::TraditionalChinese)
            } else {
                Some(Self::SimplifiedChinese)
            };
        }

        Self::VARIANTS
            .iter()
            .find(|&locale| locale.lang_id().language == id.language)
            .copied()
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

/// An error returned when parsing a [`Locale`] from a string.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unsupported locale: {0}")]
pub struct ParseLocaleError(String);

impl FromStr for Locale {
    type Err = ParseLocaleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::resolve(s).ok_or_else(|| ParseLocaleError(s.to_string()))
    }
}

/// Converts a value into an optional [`Locale`] for API parameters.
pub trait IntoLocale {
    fn into_locale(self) -> Option<Locale>;
}

impl IntoLocale for Locale {
    fn into_locale(self) -> Option<Locale> {
        Some(self)
    }
}

impl IntoLocale for Option<Locale> {
    fn into_locale(self) -> Option<Locale> {
        self
    }
}

impl IntoLocale for &str {
    fn into_locale(self) -> Option<Locale> {
        Locale::resolve(self)
    }
}

impl IntoLocale for String {
    fn into_locale(self) -> Option<Locale> {
        Locale::resolve(&self)
    }
}

impl IntoLocale for &LanguageIdentifier {
    fn into_locale(self) -> Option<Locale> {
        Locale::resolve_id(self)
    }
}

impl IntoLocale for LanguageIdentifier {
    fn into_locale(self) -> Option<Locale> {
        Locale::resolve_id(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use strum::IntoEnumIterator;

    #[test]
    fn variant_count() {
        assert_eq!(Locale::COUNT, 35);
        assert_eq!(Locale::iter().count(), 35);
    }

    #[test]
    fn parse() {
        assert_eq!("en".parse::<Locale>().unwrap(), Locale::English);
        assert_eq!(
            "zh-CN".parse::<Locale>().unwrap(),
            Locale::SimplifiedChinese
        );

        assert_eq!(
            "zh-Hans".parse::<Locale>().unwrap(),
            Locale::SimplifiedChinese
        );
        assert_eq!("en-US".parse::<Locale>().unwrap(), Locale::English);

        assert!("unknown".parse::<Locale>().is_err());
    }

    #[test]
    fn resolve() {
        assert_eq!(Locale::resolve("zh-Hans"), Some(Locale::SimplifiedChinese));
        assert_eq!(Locale::resolve("zh-Hant"), Some(Locale::TraditionalChinese));
        assert_eq!(
            Locale::resolve("zh-Hant-TW"),
            Some(Locale::TraditionalChinese)
        );
        assert_eq!(
            Locale::resolve("zh-Hans-CN"),
            Some(Locale::SimplifiedChinese)
        );

        assert_eq!(Locale::resolve("zh-TW"), Some(Locale::TraditionalChinese));
        assert_eq!(Locale::resolve("zh-HK"), Some(Locale::TraditionalChinese));
        assert_eq!(Locale::resolve("zh-MO"), Some(Locale::TraditionalChinese));

        assert_eq!(Locale::resolve("en-US"), Some(Locale::English));
        assert_eq!(Locale::resolve("en-GB"), Some(Locale::English));
        assert_eq!(Locale::resolve("fr-FR"), Some(Locale::French));

        assert_eq!(Locale::resolve("unknown"), None);
    }

    #[test]
    fn display_and_serialize() {
        assert_eq!(Locale::SimplifiedChinese.to_string(), "zh-CN");
        assert_eq!(Locale::English.to_string(), "en");

        assert_eq!(Locale::English.display_name(), "English");
        assert_eq!(Locale::SimplifiedChinese.display_name(), "简体中文");

        assert_eq!(
            serde_json::to_string(&Locale::SimplifiedChinese).unwrap(),
            r#""zh-CN""#
        );
        assert_eq!(serde_json::to_string(&Locale::English).unwrap(), r#""en""#);
    }

    #[test]
    fn into_locale() {
        assert_eq!(Locale::English.into_locale(), Some(Locale::English));
        assert_eq!(Some(Locale::French).into_locale(), Some(Locale::French));
        assert_eq!(None::<Locale>.into_locale(), None);
        assert_eq!("zh-Hans".into_locale(), Some(Locale::SimplifiedChinese));
        assert_eq!("unknown".into_locale(), None);

        let id: LanguageIdentifier = "en-US".parse().unwrap();
        assert_eq!(id.into_locale(), Some(Locale::English));
    }
}
