//! Pure expansion of `<cb-goto>` navigation tags.
//!
//! A `<cb-goto path="…">` inside an `:::app` block renders as a button that
//! jumps to a heading on the session's guide page (leaving the slide deck when
//! pressed from a slide). The author names the target by its visible heading
//! text — or by a raw anchor with a leading `#` — and this module resolves it
//! to the heading's slug id at parse time, so a typo fails the build instead
//! of shipping a dead button.

use std::collections::HashSet;

use crate::error::{Error, Result};
use crate::markdown::slugify;
use crate::render::escape_html;

/// Adds every `id="…"` attribute value in `html` to `ids`.
///
/// The scan is textual, so ids quoted inside code samples are collected too.
/// That only widens the set of accepted targets; a resolved target still has
/// to exist as a real element id for the button to land anywhere.
pub(crate) fn collect_ids(html: &str, ids: &mut HashSet<String>) {
    let mut remaining = html;
    while let Some(start) = remaining.find("id=\"") {
        let rest = &remaining[start + 4..];
        let Some(end) = rest.find('"') else { return };
        ids.insert(rest[..end].to_owned());
        remaining = &rest[end + 1..];
    }
}

/// Replaces every empty `<cb-goto>` tag with one that carries its resolved
/// target anchor as a `data-target` attribute, validated against `ids`.
pub(crate) fn expand_goto_apps(html: &str, ids: &HashSet<String>) -> Result<String> {
    if !html.contains("<cb-goto") {
        return Ok(html.to_owned());
    }
    let mut rendered = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(start) = remaining.find("<cb-goto") {
        rendered.push_str(&remaining[..start]);
        let tag_start = &remaining[start..];
        let end = tag_start
            .find('>')
            .ok_or_else(|| Error::MalformedGoto("unterminated tag".to_owned()))?;
        let tag = &tag_start[..=end];
        let path = quoted_attribute(tag, "path")
            .ok_or_else(|| Error::MalformedGoto("missing a quoted path attribute".to_owned()))?;
        let target = match path.strip_prefix('#') {
            Some(anchor) => anchor.to_owned(),
            None => slugify(path),
        };
        if !ids.contains(&target) {
            return Err(Error::UnknownGotoTarget(path.to_owned()));
        }

        rendered.push_str("<cb-goto path=\"");
        rendered.push_str(&escape_html(path));
        rendered.push('"');
        if let Some(label) = quoted_attribute(tag, "label") {
            rendered.push_str(" label=\"");
            rendered.push_str(&escape_html(label));
            rendered.push('"');
        }
        rendered.push_str(" data-target=\"");
        rendered.push_str(&escape_html(&target));
        rendered.push_str("\"></cb-goto>");

        remaining = &tag_start[end + 1..];
        if !tag.trim_end().ends_with("/>") {
            let close = "</cb-goto>";
            if !remaining.starts_with(close) {
                return Err(Error::MalformedGoto(
                    "must be empty and closed with </cb-goto>".to_owned(),
                ));
            }
            remaining = &remaining[close.len()..];
        }
    }
    rendered.push_str(remaining);
    Ok(rendered)
}

fn quoted_attribute<'a>(tag: &'a str, wanted: &str) -> Option<&'a str> {
    let mut rest = tag.strip_prefix("<cb-goto")?;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() || rest.starts_with('>') || rest.starts_with("/>") {
            return None;
        }
        let key_end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))?;
        let key = &rest[..key_end];
        rest = rest[key_end..].trim_start();
        if !rest.starts_with('=') {
            if key == wanted {
                return None;
            }
            continue;
        }
        rest = rest[1..].trim_start();
        let quote = rest.chars().next()?;
        if quote != '\'' && quote != '"' {
            return None;
        }
        rest = &rest[quote.len_utf8()..];
        let value_end = rest.find(quote)?;
        let value = &rest[..value_end];
        rest = &rest[value_end + quote.len_utf8()..];
        if key == wanted {
            return Some(value);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> HashSet<String> {
        values.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn collect_ids_finds_heading_ids() {
        let mut found = HashSet::new();
        collect_ids(
            "<h2 id=\"practica-guiada\"><a href=\"#practica-guiada\">P</a></h2>\n\
             <h3 id=\"otra\">O</h3>",
            &mut found,
        );
        assert!(found.contains("practica-guiada"));
        assert!(found.contains("otra"));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn resolves_heading_text_to_data_target() {
        let expanded = expand_goto_apps(
            "<cb-goto path=\"Práctica guiada: escalar\"></cb-goto>",
            &ids(&["practica-guiada-escalar"]),
        )
        .unwrap();
        assert_eq!(
            expanded,
            "<cb-goto path=\"Práctica guiada: escalar\" \
             data-target=\"practica-guiada-escalar\"></cb-goto>"
        );
    }

    #[test]
    fn accepts_a_raw_anchor_path() {
        let expanded = expand_goto_apps(
            "<cb-goto path=\"#mi-ancla\"></cb-goto>",
            &ids(&["mi-ancla"]),
        )
        .unwrap();
        assert!(expanded.contains("data-target=\"mi-ancla\""));
    }

    #[test]
    fn preserves_a_label_attribute() {
        let expanded = expand_goto_apps(
            "<cb-goto path=\"Cierre\" label=\"Seguir en la guía\"></cb-goto>",
            &ids(&["cierre"]),
        )
        .unwrap();
        assert!(expanded.contains("label=\"Seguir en la guía\""));
    }

    #[test]
    fn unknown_target_is_an_error() {
        let err = expand_goto_apps("<cb-goto path=\"No existe\"></cb-goto>", &ids(&["otra"]))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "cb-goto references an unknown heading: No existe"
        );
    }

    #[test]
    fn missing_path_is_an_error() {
        let err = expand_goto_apps("<cb-goto></cb-goto>", &ids(&[])).unwrap_err();
        assert!(err.to_string().contains("missing a quoted path attribute"));
    }

    #[test]
    fn non_empty_body_is_an_error() {
        let err = expand_goto_apps(
            "<cb-goto path=\"Cierre\">texto</cb-goto>",
            &ids(&["cierre"]),
        )
        .unwrap_err();
        assert!(err.to_string().contains("must be empty"));
    }

    #[test]
    fn html_without_tags_passes_through() {
        let html = "<p>sin botones</p>";
        assert_eq!(expand_goto_apps(html, &ids(&[])).unwrap(), html);
    }
}
