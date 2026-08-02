//! Pure expansion of `<cb-file>` app tags into self-contained custom elements.
//!
//! The server shell supplies a source lookup. This module never reads files: in
//! production the lookup uses `include_str!`, while dev mode reads from the
//! checked-out repository.

use color_eyre::eyre::{Result, eyre};
use courses_core::escape_html;

/// Replaces every empty `<cb-file>` tag with one that carries the referenced
/// source as an escaped `data-content` attribute.
///
/// Paths are repository-relative. A lookup receives the normalized form, so
/// both `./buildspec.yml` and `buildspec.yml` resolve to `buildspec.yml`.
pub fn expand_file_apps(
    markdown: &str,
    mut source_for: impl FnMut(&str) -> Option<String>,
) -> Result<String> {
    let mut rendered = String::with_capacity(markdown.len());
    let mut remaining = markdown;

    while let Some(start) = remaining.find("<cb-file") {
        rendered.push_str(&remaining[..start]);
        let tag_start = &remaining[start..];
        let end = tag_start
            .find('>')
            .ok_or_else(|| eyre!("unterminated <cb-file> tag"))?;
        let tag = &tag_start[..=end];
        let source_path = quoted_attribute(tag, "path")
            .ok_or_else(|| eyre!("<cb-file> is missing a quoted path attribute"))?;
        let path = normalize_path(source_path)?;
        let file_type = quoted_attribute(tag, "type").unwrap_or("text");
        if file_type.trim().is_empty() {
            return Err(eyre!("<cb-file> type must not be empty"));
        }
        let source = source_for(&path)
            .ok_or_else(|| eyre!("<cb-file> references an unknown file: {path}"))?;

        rendered.push_str("<cb-file path=\"");
        rendered.push_str(&escape_html(source_path));
        rendered.push_str("\" type=\"");
        rendered.push_str(&escape_html(file_type));
        rendered.push('"');
        if has_boolean_attribute(tag, "toggleable") {
            rendered.push_str(" toggleable");
        }
        if has_boolean_attribute(tag, "open") {
            rendered.push_str(" open");
        }
        if has_boolean_attribute(tag, "full-path") {
            rendered.push_str(" full-path");
        }
        rendered.push_str(" data-content=\"");
        rendered.push_str(&escape_html(&source));
        rendered.push_str("\"></cb-file>");

        remaining = &tag_start[end + 1..];
        if !tag.trim_end().ends_with("/>") {
            let close = "</cb-file>";
            if !remaining.starts_with(close) {
                return Err(eyre!("<cb-file> must be empty and closed with </cb-file>"));
            }
            remaining = &remaining[close.len()..];
        }
    }
    rendered.push_str(remaining);
    Ok(rendered)
}

fn quoted_attribute<'a>(tag: &'a str, wanted: &str) -> Option<&'a str> {
    let mut rest = tag.strip_prefix("<cb-file")?;
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

fn has_boolean_attribute(tag: &str, wanted: &str) -> bool {
    let Some(mut rest) = tag.strip_prefix("<cb-file") else {
        return false;
    };
    loop {
        rest = rest.trim_start();
        if rest.is_empty() || rest.starts_with('>') || rest.starts_with("/>") {
            return false;
        }
        let Some(key_end) =
            rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        else {
            return false;
        };
        let key = &rest[..key_end];
        rest = rest[key_end..].trim_start();
        if !rest.starts_with('=') {
            if key == wanted {
                return true;
            }
            continue;
        }
        rest = rest[1..].trim_start();
        let Some(quote) = rest.chars().next() else {
            return false;
        };
        if quote != '\'' && quote != '"' {
            return false;
        }
        rest = &rest[quote.len_utf8()..];
        let Some(value_end) = rest.find(quote) else {
            return false;
        };
        rest = &rest[value_end + quote.len_utf8()..];
    }
}

fn normalize_path(raw: &str) -> Result<String> {
    let path = raw.trim().trim_start_matches("./");
    if path.is_empty()
        || path.starts_with('/')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(eyre!(
            "<cb-file> path must be repository-relative and cannot traverse directories: {raw:?}"
        ));
    }
    Ok(path.to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn expands_a_file_tag_with_escaped_source() {
        let expanded = expand_file_apps(
            ":::app\n<cb-file path=\"./buildspec.yml\" type=\"yaml\"></cb-file>\n:::\n",
            |path| (path == "buildspec.yml").then(|| "answer: <yes>".to_owned()),
        )
        .unwrap();
        assert!(expanded.contains("path=\"./buildspec.yml\""));
        assert!(expanded.contains("type=\"yaml\""));
        assert!(expanded.contains("data-content=\"answer: &lt;yes&gt;\""));
    }

    #[test]
    fn rejects_a_path_outside_the_repository() {
        let err = expand_file_apps("<cb-file path=\"../.env\"></cb-file>", |_| None).unwrap_err();
        assert!(err.to_string().contains("cannot traverse directories"));
    }

    #[test]
    fn rejects_an_unknown_source() {
        let err =
            expand_file_apps("<cb-file path=\"missing.yml\"></cb-file>", |_| None).unwrap_err();
        assert!(err.to_string().contains("unknown file: missing.yml"));
    }

    #[test]
    fn preserves_file_viewer_boolean_attributes() {
        let expanded = expand_file_apps(
            "<cb-file toggleable path=\"Dockerfile\" type=\"dockerfile\" open full-path></cb-file>",
            |path| (path == "Dockerfile").then(|| "FROM scratch".to_owned()),
        )
        .unwrap();
        assert!(expanded.contains("type=\"dockerfile\" toggleable open full-path"));
    }
}
