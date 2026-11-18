//! The words NomNom shows outside its window: the tray menu and notifications. The window itself
//! is translated in the interface (src/lib/i18n). English is the key; an unknown text stays English.

use nomnom_core::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Pt,
    Es,
}

/// The language from the settings; "auto" follows the computer's language.
pub fn lang_of(settings: &Settings) -> Lang {
    match settings.general.language.as_str() {
        "pt" => Lang::Pt,
        "es" => Lang::Es,
        "en" => Lang::En,
        _ => system_lang(),
    }
}

fn from_tag(tag: &str) -> Option<Lang> {
    match tag.to_ascii_lowercase().split(['-', '_', '.']).next()? {
        "pt" => Some(Lang::Pt),
        "es" => Some(Lang::Es),
        "en" => Some(Lang::En),
        _ => None,
    }
}

#[cfg(windows)]
fn system_lang() -> Lang {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    // SAFETY: the buffer is 85 elements long, the size Windows documents for a locale name.
    let n = unsafe { GetUserDefaultLocaleName(&mut buf) };
    if n > 1 {
        let name = String::from_utf16_lossy(&buf[..(n as usize - 1)]);
        if let Some(l) = from_tag(&name) {
            return l;
        }
    }
    Lang::En
}

#[cfg(not(windows))]
fn system_lang() -> Lang {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(l) = std::env::var(var).ok().as_deref().and_then(from_tag) {
            return l;
        }
    }
    Lang::En
}

/// (English, Português, Español)
const TABLE: &[(&str, &str, &str)] = &[
    ("Open NomNom", "Abrir o NomNom", "Abrir NomNom"),
    (
        "Nothing downloading",
        "Nada sendo baixado",
        "Nada descargándose",
    ),
    (
        "Open Downloads folder",
        "Abrir a pasta Downloads",
        "Abrir la carpeta Descargas",
    ),
    ("Quit NomNom", "Sair do NomNom", "Salir de NomNom"),
    (
        "Download finished",
        "Download concluído",
        "Descarga terminada",
    ),
    (
        "Downloads finished",
        "Downloads concluídos",
        "Descargas terminadas",
    ),
    (
        "Couldn't download",
        "Não foi possível baixar",
        "No se pudo descargar",
    ),
    (
        "Downloads failed",
        "Os downloads falharam",
        "Las descargas fallaron",
    ),
    (
        "Files organized",
        "Arquivos organizados",
        "Archivos organizados",
    ),
    (
        "No supported link on the clipboard",
        "Nenhum link compatível na área de transferência",
        "Ningún enlace compatible en el portapapeles",
    ),
    (
        "File Tools",
        "Ferramentas de Arquivo",
        "Herramientas de Archivo",
    ),
];

/// Translates `text`, or returns it unchanged.
pub fn tr(lang: Lang, text: &str) -> String {
    for (en, pt, es) in TABLE {
        if *en == text {
            return match lang {
                Lang::En => en,
                Lang::Pt => pt,
                Lang::Es => es,
            }
            .to_string();
        }
    }
    text.to_string()
}

/// "N downloads are done." and similar sentences that carry numbers.
pub fn n_done(lang: Lang, n: usize) -> String {
    match lang {
        Lang::En => format!("{n} downloads are done."),
        Lang::Pt => format!("{n} downloads concluídos."),
        Lang::Es => format!("{n} descargas terminadas."),
    }
}

pub fn n_failed(lang: Lang, n: usize) -> String {
    match lang {
        Lang::En => format!("{n} downloads didn't finish."),
        Lang::Pt => format!("{n} downloads não terminaram."),
        Lang::Es => format!("{n} descargas no terminaron."),
    }
}

pub fn done_and_failed(lang: Lang, d: usize, f: usize) -> String {
    match lang {
        Lang::En => format!("{d} done, {f} failed."),
        Lang::Pt => format!("{d} concluídos, {f} com falha."),
        Lang::Es => format!("{d} terminadas, {f} con error."),
    }
}

pub fn active_count(lang: Lang, n: usize, percent: Option<u32>) -> String {
    let word = match lang {
        Lang::En => "active",
        Lang::Pt => {
            if n == 1 {
                "ativo"
            } else {
                "ativos"
            }
        }
        Lang::Es => {
            if n == 1 {
                "activa"
            } else {
                "activas"
            }
        }
    };
    match percent {
        Some(p) => format!("{n} {word} · {p}%"),
        None => format!("{n} {word}"),
    }
}

pub fn organized_toast(lang: Lang, moved: usize) -> String {
    match (lang, moved) {
        (Lang::En, 1) => "Moved 1 new download to your organized folders. You can undo it in NomNom.".into(),
        (Lang::En, n) => format!("Moved {n} new downloads to your organized folders. You can undo this in NomNom."),
        (Lang::Pt, 1) => "1 novo download foi movido para as suas pastas organizadas. Você pode desfazer no NomNom.".into(),
        (Lang::Pt, n) => format!("{n} novos downloads foram movidos para as suas pastas organizadas. Você pode desfazer no NomNom."),
        (Lang::Es, 1) => "Se movió 1 descarga nueva a tus carpetas organizadas. Puedes deshacerlo en NomNom.".into(),
        (Lang::Es, n) => format!("Se movieron {n} descargas nuevas a tus carpetas organizadas. Puedes deshacerlo en NomNom."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_tags_are_recognised() {
        assert_eq!(from_tag("pt-BR"), Some(Lang::Pt));
        assert_eq!(from_tag("es_MX.UTF-8"), Some(Lang::Es));
        assert_eq!(from_tag("en-US"), Some(Lang::En));
        assert_eq!(from_tag("fr-FR"), None);
        assert_eq!(from_tag(""), None);
    }

    #[test]
    fn known_text_is_translated_and_unknown_text_is_kept() {
        assert_eq!(tr(Lang::Pt, "Quit NomNom"), "Sair do NomNom");
        assert_eq!(tr(Lang::Es, "Open NomNom"), "Abrir NomNom");
        assert_eq!(tr(Lang::En, "Quit NomNom"), "Quit NomNom");
        assert_eq!(tr(Lang::Pt, "Something else"), "Something else");
    }

    #[test]
    fn sentences_with_numbers_follow_the_language() {
        assert_eq!(active_count(Lang::En, 2, Some(40)), "2 active · 40%");
        assert_eq!(active_count(Lang::Pt, 1, None), "1 ativo");
        assert_eq!(n_done(Lang::Es, 3), "3 descargas terminadas.");
    }
}
