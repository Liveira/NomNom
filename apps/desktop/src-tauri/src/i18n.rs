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
    (
        "Scheduled download couldn't start",
        "O download agendado não pôde começar",
        "La descarga programada no pudo empezar",
    ),
    (
        "This is how notifications look",
        "Assim ficam as notificações",
        "Así se ven las notificaciones",
    ),
    (
        "They go away by themselves. Click one to open its page.",
        "Elas somem sozinhas. Clique em uma para abrir a página dela.",
        "Se van solas. Haz clic en una para abrir su página.",
    ),
    (
        "Couldn't translate",
        "Não foi possível traduzir",
        "No se pudo traducir",
    ),
    (
        "Add your DeepL key in the Translator page first.",
        "Adicione sua chave do DeepL na página Tradutor primeiro.",
        "Añade tu clave de DeepL en la página Traductor primero.",
    ),
    (
        "Select some text first.",
        "Selecione algum texto primeiro.",
        "Selecciona algún texto primero.",
    ),
    (
        "The selection isn't text.",
        "A seleção não é texto.",
        "La selección no es texto.",
    ),
    (
        "Already in that language",
        "Já está nesse idioma",
        "Ya está en ese idioma",
    ),
    (
        "Nothing to change.",
        "Nada para mudar.",
        "Nada que cambiar.",
    ),
    ("Translated", "Traduzido", "Traducido"),
    (
        "It is on the clipboard: press Ctrl+V to paste it.",
        "Está na área de transferência: pressione Ctrl+V para colar.",
        "Está en el portapapeles: pulsa Ctrl+V para pegarlo.",
    ),
    (
        "Couldn't use the clipboard.",
        "Não foi possível usar a área de transferência.",
        "No se pudo usar el portapapeles.",
    ),
    (
        "Couldn't open the translation window.",
        "Não foi possível abrir a janela de tradução.",
        "No se pudo abrir la ventana de traducción.",
    ),
    (
        "To translate selected text, install wl-clipboard (wl-paste).",
        "Para traduzir texto selecionado, instale o wl-clipboard (wl-paste).",
        "Para traducir texto seleccionado, instala wl-clipboard (wl-paste).",
    ),
    (
        "To translate selected text, install xclip or xsel.",
        "Para traduzir texto selecionado, instale o xclip ou o xsel.",
        "Para traducir texto seleccionado, instala xclip o xsel.",
    ),
    (
        "DeepL didn't accept the key. Check it in the Translator page.",
        "O DeepL não aceitou a chave. Confira na página Tradutor.",
        "DeepL no aceptó la clave. Revísala en la página Traductor.",
    ),
    (
        "This month's DeepL characters are used up.",
        "Os caracteres do DeepL deste mês acabaram.",
        "Se acabaron los caracteres de DeepL de este mes.",
    ),
    (
        "DeepL is busy right now. Try again in a moment.",
        "O DeepL está ocupado agora. Tente de novo em instantes.",
        "DeepL está ocupado ahora. Inténtalo de nuevo en un momento.",
    ),
    (
        "That text is too long for DeepL.",
        "Esse texto é longo demais para o DeepL.",
        "Ese texto es demasiado largo para DeepL.",
    ),
    (
        "DeepL couldn't use that text or language.",
        "O DeepL não conseguiu usar esse texto ou idioma.",
        "DeepL no pudo usar ese texto o idioma.",
    ),
    (
        "DeepL had a problem. Try again in a moment.",
        "O DeepL teve um problema. Tente de novo em instantes.",
        "DeepL tuvo un problema. Inténtalo de nuevo en un momento.",
    ),
    (
        "DeepL answered with an error.",
        "O DeepL respondeu com um erro.",
        "DeepL respondió con un error.",
    ),
    (
        "Couldn't reach DeepL. Check your internet connection.",
        "Não foi possível falar com o DeepL. Confira sua conexão com a internet.",
        "No se pudo contactar con DeepL. Revisa tu conexión a internet.",
    ),
    (
        "Sharing with your phone stopped",
        "O compartilhamento com o celular parou",
        "El uso compartido con el móvil se detuvo",
    ),
    (
        "Nobody used it for a while, so NomNom turned it off.",
        "Ninguém usou por um tempo, então o NomNom desligou.",
        "Nadie lo usó durante un rato, así que NomNom lo apagó.",
    ),
    (
        "Couldn't read the screen",
        "Não foi possível ler a tela",
        "No se pudo leer la pantalla",
    ),
    (
        "Windows didn't let NomNom take a picture of the screen.",
        "O Windows não deixou o NomNom tirar uma foto da tela.",
        "Windows no dejó que NomNom tomara una imagen de la pantalla.",
    ),
    (
        "No text found",
        "Nenhum texto encontrado",
        "No se encontró texto",
    ),
    (
        "NomNom didn't find any text in that part of the screen.",
        "O NomNom não achou texto nessa parte da tela.",
        "NomNom no encontró texto en esa parte de la pantalla.",
    ),
    (
        "To read text from the screen, install tesseract.",
        "Para ler texto da tela, instale o tesseract.",
        "Para leer texto de la pantalla, instala tesseract.",
    ),
    (
        "To read text from the screen on Wayland, install grim and slurp.",
        "Para ler texto da tela no Wayland, instale grim e slurp.",
        "Para leer texto de la pantalla en Wayland, instala grim y slurp.",
    ),
    (
        "To read text from the screen on X11, install maim.",
        "Para ler texto da tela no X11, instale o maim.",
        "Para leer texto de la pantalla en X11, instala maim.",
    ),
    ("Reminder", "Lembrete", "Recordatorio"),
    ("Pinned on top", "Fixada no topo", "Fijada encima"),
    ("Not on top any more", "Não está mais no topo", "Ya no está encima"),
    ("There is no window to pin.", "Não há janela para fixar.", "No hay ventana que fijar."),
    (
        "Pinning windows works on Windows only.",
        "Fixar janelas funciona só no Windows.",
        "Fijar ventanas funciona solo en Windows.",
    ),
    ("Link cleaned", "Link limpo", "Enlace limpio"),
    (
        "There was no tracking in the link on the clipboard.",
        "Não havia rastreio no link da área de transferência.",
        "No había rastreo en el enlace del portapapeles.",
    ),
    (
        "The clipboard has no text.",
        "A área de transferência não tem texto.",
        "El portapapeles no tiene texto.",
    ),
    ("Colour copied", "Cor copiada", "Color copiado"),
    (
        "Picking a colour works on Windows only.",
        "Escolher uma cor funciona só no Windows.",
        "Elegir un color funciona solo en Windows.",
    ),
    (
        "Nothing was selected.",
        "Nada foi selecionado.",
        "No se seleccionó nada.",
    ),
    (
        "Windows has no text recognition for your language yet. Add one in Settings, Time & language, Language & region.",
        "O Windows ainda não tem reconhecimento de texto para o seu idioma. Adicione um em Configurações, Hora e idioma, Idioma e região.",
        "Windows aún no tiene reconocimiento de texto para tu idioma. Añade uno en Configuración, Hora e idioma, Idioma y región.",
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

/// "1 h 20 min" for a number of minutes.
pub fn hm(minutes: u64) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

/// The text of a Playtime goal alert.
pub fn goal_text(
    lang: Lang,
    alert: nomnom_activity::goals::Alert,
    name: &str,
    used_mins: u32,
    goal_mins: u32,
    period: nomnom_activity::goals::Period,
) -> (String, String) {
    use nomnom_activity::goals::{Alert, Period};
    let when = match (lang, period) {
        (Lang::En, Period::Day) => "today",
        (Lang::En, Period::Week) => "this week",
        (Lang::Pt, Period::Day) => "hoje",
        (Lang::Pt, Period::Week) => "nesta semana",
        (Lang::Es, Period::Day) => "hoy",
        (Lang::Es, Period::Week) => "esta semana",
    };
    let (u, g) = (hm(u64::from(used_mins)), hm(u64::from(goal_mins)));
    let missing = hm(u64::from(goal_mins.saturating_sub(used_mins)));
    match (lang, alert) {
        (Lang::En, Alert::LimitNear) => (
            "Almost at your limit".into(),
            format!("{name}: {u} of {g} {when}."),
        ),
        (Lang::En, Alert::LimitOver) => (
            "Limit reached".into(),
            format!("{name}: {u} {when}, over your {g} limit."),
        ),
        (Lang::En, Alert::TargetMet) => (
            "Goal reached".into(),
            format!("{name}: {u} of {g} {when}. Well done!"),
        ),
        (Lang::En, Alert::TargetBehind) => (
            "A little short of your goal".into(),
            format!("{name}: {u} of {g} {when}. {missing} to go."),
        ),
        (Lang::Pt, Alert::LimitNear) => (
            "Quase no seu limite".into(),
            format!("{name}: {u} de {g} {when}."),
        ),
        (Lang::Pt, Alert::LimitOver) => (
            "Limite atingido".into(),
            format!("{name}: {u} {when}, acima do limite de {g}."),
        ),
        (Lang::Pt, Alert::TargetMet) => (
            "Meta batida".into(),
            format!("{name}: {u} de {g} {when}. Muito bem!"),
        ),
        (Lang::Pt, Alert::TargetBehind) => (
            "Falta pouco para a meta".into(),
            format!("{name}: {u} de {g} {when}. Faltam {missing}."),
        ),
        (Lang::Es, Alert::LimitNear) => (
            "Casi en tu límite".into(),
            format!("{name}: {u} de {g} {when}."),
        ),
        (Lang::Es, Alert::LimitOver) => (
            "Límite alcanzado".into(),
            format!("{name}: {u} {when}, por encima del límite de {g}."),
        ),
        (Lang::Es, Alert::TargetMet) => (
            "Meta cumplida".into(),
            format!("{name}: {u} de {g} {when}. ¡Bien hecho!"),
        ),
        (Lang::Es, Alert::TargetBehind) => (
            "Te falta poco para la meta".into(),
            format!("{name}: {u} de {g} {when}. Faltan {missing}."),
        ),
    }
}

/// The notice that new videos of a followed channel were started.
pub fn new_videos_text(lang: Lang, name: &str, n: usize) -> (String, String) {
    match lang {
        Lang::En => (
            format!("New from {name}"),
            if n == 1 {
                "1 new video is downloading.".into()
            } else {
                format!("{n} new videos are downloading.")
            },
        ),
        Lang::Pt => (
            format!("Novidade de {name}"),
            if n == 1 {
                "1 vídeo novo está sendo baixado.".into()
            } else {
                format!("{n} vídeos novos estão sendo baixados.")
            },
        ),
        Lang::Es => (
            format!("Novedad de {name}"),
            if n == 1 {
                "1 vídeo nuevo se está descargando.".into()
            } else {
                format!("{n} vídeos nuevos se están descargando.")
            },
        ),
    }
}

/// The notice that text was read from the screen.
pub fn ocr_text(lang: Lang, text: &str, translated: bool) -> (String, String) {
    let one_line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let preview: String = if one_line.chars().count() > 90 {
        format!("{}…", one_line.chars().take(90).collect::<String>())
    } else {
        one_line
    };
    let title = match (lang, translated) {
        (Lang::En, false) => "Text copied",
        (Lang::En, true) => "Translated text copied",
        (Lang::Pt, false) => "Texto copiado",
        (Lang::Pt, true) => "Texto traduzido copiado",
        (Lang::Es, false) => "Texto copiado",
        (Lang::Es, true) => "Texto traducido copiado",
    };
    (title.to_string(), preview)
}

/// The notice that files arrived from the phone.
pub fn phone_text(lang: Lang, n: usize, folder: &str) -> (String, String) {
    match lang {
        Lang::En => (
            if n == 1 {
                "1 file from your phone".into()
            } else {
                format!("{n} files from your phone")
            },
            format!("Saved in {folder}"),
        ),
        Lang::Pt => (
            if n == 1 {
                "1 arquivo do seu celular".into()
            } else {
                format!("{n} arquivos do seu celular")
            },
            format!("Salvo em {folder}"),
        ),
        Lang::Es => (
            if n == 1 {
                "1 archivo de tu móvil".into()
            } else {
                format!("{n} archivos de tu móvil")
            },
            format!("Guardado en {folder}"),
        ),
    }
}

/// The notice that the weekly report is ready.
pub fn report_text(lang: Lang, active_mins: u64) -> (String, String) {
    let h = hm(active_mins);
    match lang {
        Lang::En => (
            "Your weekly report is ready".into(),
            format!("You were active for {h} last week. See how it went."),
        ),
        Lang::Pt => (
            "Seu relatório semanal está pronto".into(),
            format!("Você ficou ativo por {h} na semana passada. Veja como foi."),
        ),
        Lang::Es => (
            "Tu informe semanal está listo".into(),
            format!("Estuviste activo {h} la semana pasada. Mira cómo fue."),
        ),
    }
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

/// The notification after a scheduled backup, or any backup that failed.
pub fn backup_done(lang: Lang, ok: bool, name: &str, note: &str) -> (String, String) {
    let t = |en: &str, pt: &str, es: &str| match lang {
        Lang::En => en.to_string(),
        Lang::Pt => pt.to_string(),
        Lang::Es => es.to_string(),
    };
    if ok {
        (
            t(
                "Backup finished",
                "Backup concluído",
                "Copia de seguridad terminada",
            ),
            if note.is_empty() {
                name.to_string()
            } else {
                format!("{name}: {note}")
            },
        )
    } else {
        (
            t(
                "Backup didn't finish",
                "O backup não terminou",
                "La copia de seguridad no terminó",
            ),
            format!("{name}: {note}"),
        )
    }
}

/// The notification when a timer ends: its title and body.
pub fn focus_done(
    lang: Lang,
    was_focus: bool,
    block: Option<&crate::focus::Block>,
) -> (String, String) {
    let mins = block.map(|b| b.planned_secs / 60).unwrap_or(0);
    let top = block
        .and_then(|b| b.apps.first())
        .map(|a| a.name.clone())
        .unwrap_or_default();
    match (lang, was_focus) {
        (Lang::En, true) => (
            "Focus block finished".into(),
            if top.is_empty() {
                format!("{mins} minutes done. Time for a break.")
            } else {
                format!("{mins} minutes done, mostly in {top}. Time for a break.")
            },
        ),
        (Lang::En, false) => ("Break is over".into(), "Ready for the next block?".into()),
        (Lang::Pt, true) => (
            "Bloco de foco concluído".into(),
            if top.is_empty() {
                format!("{mins} minutos concluídos. Hora de uma pausa.")
            } else {
                format!("{mins} minutos concluídos, a maior parte em {top}. Hora de uma pausa.")
            },
        ),
        (Lang::Pt, false) => (
            "A pausa acabou".into(),
            "Pronto para o próximo bloco?".into(),
        ),
        (Lang::Es, true) => (
            "Bloque de enfoque terminado".into(),
            if top.is_empty() {
                format!("{mins} minutos hechos. Hora de un descanso.")
            } else {
                format!("{mins} minutos hechos, sobre todo en {top}. Hora de un descanso.")
            },
        ),
        (Lang::Es, false) => (
            "El descanso terminó".into(),
            "¿Listo para el siguiente bloque?".into(),
        ),
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
