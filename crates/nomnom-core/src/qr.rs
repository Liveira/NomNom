//! QR codes: what goes inside one for joining a Wi-Fi network, and what kind of text a scanned code
//! holds. Drawing and reading the picture is the app's job.

pub const MAX_TEXT: usize = 1800;

/// Escapes a value the way the Wi-Fi code format wants: `\`, `;`, `,`, `:` and `"` get a backslash.
fn escape(v: &str) -> String {
    let mut out = String::new();
    for c in v.chars() {
        if matches!(c, '\\' | ';' | ',' | ':' | '"') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The text a phone reads as "join this network": `WIFI:T:WPA;S:name;P:password;H:true;;`.
/// `security` is "WPA", "WEP" or "none".
pub fn wifi_payload(
    ssid: &str,
    password: &str,
    security: &str,
    hidden: bool,
) -> Result<String, String> {
    if ssid.is_empty() {
        return Err("Type the name of the Wi-Fi network.".into());
    }
    if ssid.chars().count() > 32 {
        return Err("A Wi-Fi name has at most 32 characters.".into());
    }
    if ssid.contains(['\n', '\r', '\0']) || password.contains(['\n', '\r', '\0']) {
        return Err("That can't be written in a code.".into());
    }
    let kind = match security {
        "WPA" | "WEP" => security,
        "none" => "nopass",
        _ => return Err("Choose WPA, WEP or none.".into()),
    };
    if kind != "nopass" && password.is_empty() {
        return Err("Type the password, or choose a network with none.".into());
    }
    if password.chars().count() > 63 {
        return Err("A Wi-Fi password has at most 63 characters.".into());
    }
    let mut out = format!("WIFI:T:{kind};S:{};", escape(ssid));
    if kind != "nopass" {
        out.push_str(&format!("P:{};", escape(password)));
    }
    if hidden {
        out.push_str("H:true;");
    }
    out.push(';');
    Ok(out)
}

/// Only web links and mail links may be opened for the person; everything else is shown as text.
pub fn is_openable_link(text: &str) -> bool {
    let t = text.trim().to_ascii_lowercase();
    !t.contains(char::is_whitespace)
        && (t.starts_with("http://") || t.starts_with("https://") || t.starts_with("mailto:"))
}

/// What a scanned text is, for the label: "Wi-Fi network", "Link" or "Text".
pub fn kind_of(text: &str) -> &'static str {
    if text.starts_with("WIFI:") {
        "Wi-Fi network"
    } else if is_openable_link(text) {
        "Link"
    } else {
        "Text"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wifi_code_has_the_fields_a_phone_expects() {
        assert_eq!(
            wifi_payload("Home", "secret123", "WPA", false).unwrap(),
            "WIFI:T:WPA;S:Home;P:secret123;;"
        );
        assert_eq!(
            wifi_payload("Cafe", "", "none", false).unwrap(),
            "WIFI:T:nopass;S:Cafe;;"
        );
        assert_eq!(
            wifi_payload("Hid", "pw", "WEP", true).unwrap(),
            "WIFI:T:WEP;S:Hid;P:pw;H:true;;"
        );
    }

    #[test]
    fn the_special_characters_are_escaped() {
        assert_eq!(
            wifi_payload("My;Net", "a:b,c\\d\"e", "WPA", false).unwrap(),
            "WIFI:T:WPA;S:My\\;Net;P:a\\:b\\,c\\\\d\\\"e;;"
        );
    }

    #[test]
    fn nonsense_is_refused_with_a_reason() {
        assert!(wifi_payload("", "x", "WPA", false).is_err());
        assert!(wifi_payload(&"n".repeat(33), "x", "WPA", false).is_err());
        assert!(wifi_payload("Net", "", "WPA", false).is_err());
        assert!(wifi_payload("Net", "x", "WPA3", false).is_err());
        assert!(wifi_payload("Net", &"p".repeat(64), "WPA", false).is_err());
        assert!(wifi_payload("Net\nwork", "x", "WPA", false).is_err());
    }

    #[test]
    fn only_web_and_mail_links_may_be_opened() {
        assert!(is_openable_link("https://example.com/a?b=1"));
        assert!(is_openable_link(" mailto:me@example.com "));
        assert!(!is_openable_link("javascript:alert(1)"));
        assert!(!is_openable_link("file:///C:/Windows/System32/cmd.exe"));
        assert!(!is_openable_link("https://a.com b"));
        assert!(!is_openable_link("calc.exe"));
    }

    #[test]
    fn the_kind_is_named() {
        assert_eq!(kind_of("WIFI:T:WPA;S:x;P:y;;"), "Wi-Fi network");
        assert_eq!(kind_of("https://x.org"), "Link");
        assert_eq!(kind_of("hello"), "Text");
    }
}
