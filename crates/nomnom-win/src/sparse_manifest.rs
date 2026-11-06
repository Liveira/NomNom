//! Fills in the sparse package manifest template.

use crate::shell_menu::{all_exts, SHELL_CLSID};

/// `template` is `packaging/sparse/AppxManifest.template.xml`.
pub fn render(template: &str, publisher: &str, version: &str) -> String {
    let items: String = all_exts()
        .iter()
        .map(|e| {
            format!(
                "            <desktop5:ItemType Type=\".{e}\">\n              <desktop5:Verb Id=\"NomNom\" Clsid=\"{SHELL_CLSID}\" />\n            </desktop5:ItemType>"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    template
        .replace("{{PUBLISHER}}", &xml_escape(publisher))
        .replace("{{VERSION}}", version)
        .replace("{{CLSID}}", SHELL_CLSID)
        .replace("{{ITEM_TYPES}}", &items)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Package versions are four numbers; "0.1.0" becomes "0.1.0.0".
pub fn package_version(app_version: &str) -> String {
    let mut parts: Vec<&str> = app_version.split('.').take(4).collect();
    while parts.len() < 4 {
        parts.push("0");
    }
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = include_str!("../../../packaging/sparse/AppxManifest.template.xml");

    #[test]
    fn the_manifest_lists_every_menu_extension_once_and_no_placeholder_is_left() {
        let xml = render(TEMPLATE, "CN=Test & Co", "0.1.0.0");
        assert!(!xml.contains("{{"), "a placeholder was not filled in");
        for e in all_exts() {
            assert_eq!(xml.matches(&format!("Type=\".{e}\"")).count(), 1, "{e}");
        }
        assert!(xml.contains("Publisher=\"CN=Test &amp; Co\""));
        assert_eq!(xml.matches(SHELL_CLSID).count(), all_exts().len() + 1);
        assert!(!xml.contains("Type=\".exe\""));
        assert!(!xml.contains("Type=\".ts\""));
    }

    #[test]
    fn the_manifest_is_well_formed_enough_to_have_balanced_tags() {
        let xml = render(TEMPLATE, "CN=NOMNOM Dev", "0.1.0.0");
        for tag in [
            "Package",
            "Applications",
            "Application",
            "Extensions",
            "desktop4:FileExplorerContextMenus",
            "com:ComServer",
        ] {
            // An opening tag is "<tag" followed by whitespace or ">" (not a longer name).
            let open = xml
                .match_indices(&format!("<{tag}"))
                .filter(|(i, _)| {
                    xml[i + tag.len() + 1..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_whitespace() || c == '>')
                })
                .count();
            let close = xml.matches(&format!("</{tag}>")).count();
            assert_eq!(open, close, "{tag}");
        }
    }

    #[test]
    fn versions_have_four_parts() {
        assert_eq!(package_version("0.1.0"), "0.1.0.0");
        assert_eq!(package_version("1.2"), "1.2.0.0");
        assert_eq!(package_version("1.2.3.4.5"), "1.2.3.4");
    }
}
