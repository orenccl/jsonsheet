use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    En,
    ZhHant,
}

impl Language {
    pub fn all() -> &'static [Self] {
        &[Self::En, Self::ZhHant]
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhHant => "zh-Hant",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::En),
            "zh-Hant" => Some(Self::ZhHant),
            _ => None,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::En => "language.option.en",
            Self::ZhHant => "language.option.zh_hant",
        }
    }
}

pub fn tr(language: Language, key: &'static str) -> &'static str {
    catalog(language)
        .get(key)
        .map(String::as_str)
        .or_else(|| catalog(Language::En).get(key).map(String::as_str))
        .unwrap_or(key)
}

fn catalog(language: Language) -> &'static BTreeMap<String, String> {
    match language {
        Language::En => EN_CATALOG.get_or_init(|| parse_catalog(Language::En)),
        Language::ZhHant => ZH_HANT_CATALOG.get_or_init(|| parse_catalog(Language::ZhHant)),
    }
}

fn parse_catalog(language: Language) -> BTreeMap<String, String> {
    let source = match language {
        Language::En => include_str!("../../assets/i18n/en.json"),
        Language::ZhHant => include_str!("../../assets/i18n/zh-Hant.json"),
    };

    serde_json::from_str(source).unwrap_or_else(|err| {
        panic!(
            "failed to parse i18n catalog for language '{}': {err}",
            language.code()
        )
    })
}

static EN_CATALOG: OnceLock<BTreeMap<String, String>> = OnceLock::new();
static ZH_HANT_CATALOG: OnceLock<BTreeMap<String, String>> = OnceLock::new();
