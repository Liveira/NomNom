//! Translating text with DeepL: which server to ask, what to send, how to read the answer, and what
//! to tell the person when it goes wrong. Nothing here touches the network; the app does that.

use serde::{Deserialize, Serialize};

/// The most characters sent at once from a selection.
pub const MAX_CHARS: usize = 5000;

/// DeepL's free keys end in ":fx" and use their own server.
pub fn endpoint(key: &str) -> &'static str {
    if key.trim().ends_with(":fx") {
        "https://api-free.deepl.com"
    } else {
        "https://api.deepl.com"
    }
}

/// A language NomNom can translate into.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
}

pub const LANGUAGES: &[Language] = &[
    Language {
        code: "EN-US",
        name: "English (US)",
    },
    Language {
        code: "EN-GB",
        name: "English (UK)",
    },
    Language {
        code: "PT-BR",
        name: "Português (Brasil)",
    },
    Language {
        code: "PT-PT",
        name: "Português (Portugal)",
    },
    Language {
        code: "ES",
        name: "Español",
    },
    Language {
        code: "FR",
        name: "Français",
    },
    Language {
        code: "DE",
        name: "Deutsch",
    },
    Language {
        code: "IT",
        name: "Italiano",
    },
    Language {
        code: "NL",
        name: "Nederlands",
    },
    Language {
        code: "PL",
        name: "Polski",
    },
    Language {
        code: "RU",
        name: "Русский",
    },
    Language {
        code: "TR",
        name: "Türkçe",
    },
    Language {
        code: "UK",
        name: "Українська",
    },
    Language {
        code: "JA",
        name: "日本語",
    },
    Language {
        code: "KO",
        name: "한국어",
    },
    Language {
        code: "ZH-HANS",
        name: "中文 (简体)",
    },
    Language {
        code: "ZH-HANT",
        name: "中文 (繁體)",
    },
    Language {
        code: "ID",
        name: "Bahasa Indonesia",
    },
    Language {
        code: "SV",
        name: "Svenska",
    },
    Language {
        code: "CS",
        name: "Čeština",
    },
];

/// The language to translate into. `setting` is what the person chose in Settings; "auto" follows
/// the language NomNom itself is shown in (`ui` is "en", "pt" or "es").
pub fn target_for(setting: &str, ui: &str) -> &'static str {
    let wanted = setting.trim().to_ascii_uppercase();
    if let Some(l) = LANGUAGES.iter().find(|l| l.code == wanted) {
        return l.code;
    }
    match ui {
        "pt" => "PT-BR",
        "es" => "ES",
        _ => "EN-US",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Translation {
    pub text: String,
    /// The language DeepL recognised, such as "PT".
    pub detected: String,
}

/// The JSON to send for one text.
pub fn request_body(text: &str, target: &str) -> serde_json::Value {
    serde_json::json!({ "text": [text], "target_lang": target })
}

/// The translation inside DeepL's answer.
pub fn parse_response(json: &str) -> Result<Translation, String> {
    let v: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| "DeepL sent an answer NomNom couldn't read.".to_string())?;
    let first = v
        .get("translations")
        .and_then(|t| t.get(0))
        .ok_or_else(|| "DeepL sent no translation.".to_string())?;
    let text = first
        .get("text")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "DeepL sent no translation.".to_string())?;
    Ok(Translation {
        text: text.to_string(),
        detected: first
            .get("detected_source_language")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// What to tell the person for an HTTP status DeepL answered with.
pub fn explain_status(status: u16) -> String {
    match status {
        401 | 403 => "DeepL didn't accept the key. Check it in the Translator page.",
        456 => "This month's DeepL characters are used up.",
        429 | 529 => "DeepL is busy right now. Try again in a moment.",
        413 => "That text is too long for DeepL.",
        400 => "DeepL couldn't use that text or language.",
        500..=599 => "DeepL had a problem. Try again in a moment.",
        _ => "DeepL answered with an error.",
    }
    .to_string()
}

/// The text of a usage answer: "123 of 500000 characters".
pub fn parse_usage(json: &str) -> Option<(u64, u64)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    Some((
        v.get("character_count")?.as_u64()?,
        v.get("character_limit")?.as_u64()?,
    ))
}

/// Does the text look like it is already in `target` (by DeepL's own recognition)? "PT" is "PT-BR".
pub fn already_in(detected: &str, target: &str) -> bool {
    let d = detected.to_ascii_uppercase();
    !d.is_empty()
        && target.to_ascii_uppercase().split('-').next() == Some(d.split('-').next().unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_and_paid_keys_use_their_own_servers() {
        assert_eq!(endpoint("abc:fx"), "https://api-free.deepl.com");
        assert_eq!(endpoint(" abc:fx "), "https://api-free.deepl.com");
        assert_eq!(endpoint("abc"), "https://api.deepl.com");
    }

    #[test]
    fn the_language_follows_the_choice_or_the_app() {
        assert_eq!(target_for("PT-BR", "en"), "PT-BR");
        assert_eq!(target_for("ja", "en"), "JA");
        assert_eq!(target_for("auto", "pt"), "PT-BR");
        assert_eq!(target_for("auto", "es"), "ES");
        assert_eq!(target_for("auto", "en"), "EN-US");
        assert_eq!(target_for("klingon", "es"), "ES");
        assert_eq!(target_for("", "pt"), "PT-BR");
    }

    #[test]
    fn every_language_code_is_listed_once() {
        let mut codes: Vec<_> = LANGUAGES.iter().map(|l| l.code).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), LANGUAGES.len());
    }

    #[test]
    fn the_request_and_the_answer_are_the_shape_deepl_uses() {
        let body = request_body("Olá", "EN-US");
        assert_eq!(body["text"][0], "Olá");
        assert_eq!(body["target_lang"], "EN-US");
        let t = parse_response(
            r#"{"translations":[{"detected_source_language":"PT","text":"Hello"}]}"#,
        )
        .unwrap();
        assert_eq!(
            t,
            Translation {
                text: "Hello".into(),
                detected: "PT".into()
            }
        );
        assert!(parse_response("{}").is_err());
        assert!(parse_response("not json").is_err());
        assert!(parse_response(r#"{"translations":[]}"#).is_err());
    }

    #[test]
    fn errors_are_told_in_plain_words() {
        assert!(explain_status(403).contains("key"));
        assert!(explain_status(456).contains("used up"));
        assert!(explain_status(429).contains("busy"));
        assert!(explain_status(503).contains("problem"));
    }

    #[test]
    fn usage_is_read_and_a_matching_language_is_noticed() {
        assert_eq!(
            parse_usage(r#"{"character_count":120,"character_limit":500000}"#),
            Some((120, 500000))
        );
        assert_eq!(parse_usage("{}"), None);
        assert!(already_in("PT", "PT-BR"));
        assert!(already_in("EN", "EN-US"));
        assert!(!already_in("EN", "PT-BR"));
        assert!(!already_in("", "PT-BR"));
    }
}
