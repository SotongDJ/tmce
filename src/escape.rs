//! TMC column-content escaping.
//!
//! Only four characters require escaping inside any column:
//!   TAB (0x09) → `\t`
//!   LF  (0x0A) → `\n`
//!   CR  (0x0D) → `\r`
//!   `\` (0x5C) → `\\`
//!
//! All other bytes pass through unchanged.

use crate::error::{Result, TmceError};

pub fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

pub fn unescape(encoded: &str) -> Result<String> {
    let mut out = String::with_capacity(encoded.len());
    let bytes = encoded.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            if i + 1 >= bytes.len() {
                return Err(TmceError::Escape(format!(
                    "trailing backslash in: {:?}",
                    encoded
                )));
            }
            match bytes[i + 1] {
                b't' => out.push('\t'),
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b'\\' => out.push('\\'),
                other => {
                    return Err(TmceError::Escape(format!(
                        "unknown escape \\{} in: {:?}",
                        other as char, encoded
                    )));
                }
            }
            i += 2;
        } else {
            let ch = encoded[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_round_trip() {
        let s = "hello world";
        assert_eq!(unescape(&escape(s)).unwrap(), s);
    }

    #[test]
    fn special_round_trip() {
        let s = "tab\there\nlf\rcr\\bs";
        let e = escape(s);
        assert!(!e.contains('\t'));
        assert!(!e.contains('\n'));
        assert!(!e.contains('\r'));
        assert_eq!(unescape(&e).unwrap(), s);
    }

    #[test]
    fn utf8_round_trip() {
        let s = "héllo 日本語 🦀";
        assert_eq!(unescape(&escape(s)).unwrap(), s);
    }

    #[test]
    fn escape_tab_is_backslash_t() {
        assert_eq!(escape("\t"), "\\t");
    }

    #[test]
    fn unknown_escape_rejected() {
        assert!(unescape("\\q").is_err());
    }

    #[test]
    fn trailing_backslash_rejected() {
        assert!(unescape("abc\\").is_err());
    }
}
