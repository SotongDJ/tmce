//! TMC container: rows, parse, serialize, and in-memory model.
//!
//! # File format
//!
//! A `.tmc` file is plain UTF-8, line-oriented (LF-terminated). Column 3
//! (`section_type`) decides the row shape:
//!
//! **Header rows (6 columns)** — `section_type == "header"`:
//! ```text
//! a \t <member_order> \t header \t <uuid> \t <tags> \t <title>
//! ```
//! One per member file. `file_order` is always literal `a`. `member_order`
//! is the target member's `file_order`. `uuid` is a 12-char base62-Gu UUID
//! or the member's filename.
//!
//! **Content rows (5 columns)** — everything else:
//! ```text
//! <file_order> \t <section_order> \t <section_type> \t <indent> \t <content>
//! ```
//!
//! All column values use the escapes defined in `escape.rs`.

use std::fs;
use std::path::Path;

use crate::error::{Result, TmceError};
use crate::escape::{escape, unescape};
use crate::order::Order;
use crate::section::SectionType;

pub const MAX_INDENT: u8 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Header {
        /// Always `a`. Kept in the struct for clean round-tripping.
        file_order: Order,
        /// The `file_order` of the member file this row describes.
        member_order: Order,
        /// Base62-Gu UUID (12 chars) or filename.
        uuid: String,
        /// Canonical tags: comma-separated, spaces already replaced with `_`.
        tags: String,
        title: String,
    },
    Content {
        file_order: Order,
        section_order: Order,
        section_type: SectionType,
        indent: u8,
        content: String,
    },
}

impl Row {
    pub fn is_header(&self) -> bool {
        matches!(self, Row::Header { .. })
    }

    pub fn render(&self) -> String {
        match self {
            Row::Header {
                file_order,
                member_order,
                uuid,
                tags,
                title,
            } => format!(
                "{}\t{}\theader\t{}\t{}\t{}",
                file_order.render(),
                member_order.render(),
                escape(uuid),
                escape(tags),
                escape(title),
            ),
            Row::Content {
                file_order,
                section_order,
                section_type,
                indent,
                content,
            } => format!(
                "{}\t{}\t{}\t{}\t{}",
                file_order.render(),
                section_order.render(),
                section_type.as_str(),
                indent,
                escape(content),
            ),
        }
    }

    fn parse(line: &str, line_no: usize) -> Result<Self> {
        // Peek at column 3 without committing to a column count yet.
        let mut it = line.splitn(4, '\t');
        let c0 = it.next().ok_or_else(|| TmceError::Parse {
            line: line_no,
            message: "missing file_order column".into(),
        })?;
        let c1 = it.next().ok_or_else(|| TmceError::Parse {
            line: line_no,
            message: "missing section_order/member_order column".into(),
        })?;
        let c2 = it.next().ok_or_else(|| TmceError::Parse {
            line: line_no,
            message: "missing section_type column".into(),
        })?;
        let rest = it.next().ok_or_else(|| TmceError::Parse {
            line: line_no,
            message: "missing column 4 onwards".into(),
        })?;

        if c2 == "header" {
            // Header rows: 3 more tab-separated columns after col 3.
            let rest_cols: Vec<&str> = rest.splitn(3, '\t').collect();
            if rest_cols.len() != 3 {
                return Err(TmceError::Parse {
                    line: line_no,
                    message: format!(
                        "header row must have 6 columns (uuid, tags, title); got {} extra",
                        rest_cols.len()
                    ),
                });
            }
            let file_order = parse_order(c0, "file_order", line_no)?;
            let member_order = parse_order(c1, "member_order", line_no)?;
            if !file_order.is_header() {
                return Err(TmceError::Parse {
                    line: line_no,
                    message: format!(
                        "header row must have file_order = all-`a`; got {:?}",
                        file_order.render()
                    ),
                });
            }
            let uuid = unescape_col(rest_cols[0], "uuid", line_no)?;
            let tags = unescape_col(rest_cols[1], "tags", line_no)?;
            let title = unescape_col(rest_cols[2], "title", line_no)?;
            Ok(Row::Header {
                file_order,
                member_order,
                uuid,
                tags,
                title,
            })
        } else {
            // Content rows: 2 more tab-separated columns after col 3.
            let rest_cols: Vec<&str> = rest.splitn(2, '\t').collect();
            if rest_cols.len() != 2 {
                return Err(TmceError::Parse {
                    line: line_no,
                    message: format!(
                        "content row must have 5 columns (indent, content); got {} extra",
                        rest_cols.len()
                    ),
                });
            }
            let file_order = parse_order(c0, "file_order", line_no)?;
            let section_order = parse_order(c1, "section_order", line_no)?;
            let section_type = SectionType::parse(c2);
            let indent = parse_indent(rest_cols[0], line_no)?;
            let content = unescape_col(rest_cols[1], "content", line_no)?;
            Ok(Row::Content {
                file_order,
                section_order,
                section_type,
                indent,
                content,
            })
        }
    }
}

fn parse_order(raw: &str, field: &str, line_no: usize) -> Result<Order> {
    Order::parse(raw).map_err(|e| TmceError::Parse {
        line: line_no,
        message: format!("{}: {}", field, e),
    })
}

fn parse_indent(raw: &str, line_no: usize) -> Result<u8> {
    let n: u8 = raw.parse().map_err(|_| TmceError::Parse {
        line: line_no,
        message: format!("indent must be 0..={}, got {:?}", MAX_INDENT, raw),
    })?;
    if n > MAX_INDENT {
        return Err(TmceError::Parse {
            line: line_no,
            message: format!("indent {} exceeds max {}", n, MAX_INDENT),
        });
    }
    Ok(n)
}

fn unescape_col(raw: &str, field: &str, line_no: usize) -> Result<String> {
    unescape(raw).map_err(|e| TmceError::Parse {
        line: line_no,
        message: format!("{}: {}", field, e),
    })
}

/// An in-memory TMC container: an ordered sequence of rows.
#[derive(Debug, Clone, Default)]
pub struct Container {
    pub rows: Vec<Row>,
}

impl Container {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        Self::parse(&text)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = self.render();
        let tmp = path.with_extension("tmc.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut rows = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            rows.push(Row::parse(line, i + 1)?);
        }
        Ok(Container { rows })
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for row in &self.rows {
            out.push_str(&row.render());
            out.push('\n');
        }
        out
    }

    pub fn header_rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter().filter(|r| r.is_header())
    }

    /// Find a header row by uuid.
    pub fn find_header(&self, uuid: &str) -> Option<&Row> {
        self.rows.iter().find(|r| match r {
            Row::Header { uuid: u, .. } => u == uuid,
            _ => false,
        })
    }

    /// Validate container invariants:
    ///  - every header row has `file_order = a…a` (all lowest digit)
    ///  - each content row's `file_order` matches some header's `member_order`
    ///  - no duplicate uuid in header rows
    ///  - no duplicate (file_order, section_order) pair in content rows
    pub fn validate(&self) -> Result<()> {
        let mut uuids: Vec<&str> = Vec::new();
        let mut known_members: Vec<String> = Vec::new();

        for r in &self.rows {
            if let Row::Header {
                file_order,
                member_order,
                uuid,
                ..
            } = r
            {
                if !file_order.is_header() {
                    return Err(TmceError::Other(format!(
                        "header row has non-header file_order {:?}",
                        file_order.render()
                    )));
                }
                if uuids.contains(&uuid.as_str()) {
                    return Err(TmceError::DuplicateMember(uuid.clone()));
                }
                uuids.push(uuid.as_str());
                known_members.push(member_order.render());
            }
        }

        let mut seen_positions: Vec<(String, String)> = Vec::new();
        for r in &self.rows {
            if let Row::Content {
                file_order,
                section_order,
                ..
            } = r
            {
                let fo = file_order.render();
                if !known_members.contains(&fo) {
                    return Err(TmceError::Other(format!(
                        "content row references unknown member file_order {:?}",
                        fo
                    )));
                }
                let key = (fo, section_order.render());
                if seen_positions.contains(&key) {
                    return Err(TmceError::Other(format!(
                        "duplicate (file_order, section_order): ({}, {})",
                        key.0, key.1
                    )));
                }
                seen_positions.push(key);
            }
        }

        Ok(())
    }
}

/// Normalise a user-supplied tags argument: comma-separated, each tag has
/// whitespace replaced by `_`.
pub fn normalise_tags(raw: &str) -> String {
    raw.split(',')
        .map(|t| {
            t.split_whitespace()
                .collect::<Vec<_>>()
                .join("_")
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        "\
a\tb\theader\tnote.md\ttag1,tag2\tFirst doc
a\tc\theader\tNGk26dhizN01\tai,news\tAI article
b\ta\th1\t0\tFirst heading
b\tb\tcontent\t0\tParagraph text.
c\ta\th1\t0\tSecond file heading
"
    }

    #[test]
    fn parse_render_round_trip() {
        let c = Container::parse(sample()).unwrap();
        assert_eq!(c.rows.len(), 5);
        assert_eq!(c.render(), sample());
    }

    #[test]
    fn header_row_has_six_columns() {
        let c = Container::parse(sample()).unwrap();
        let first = &c.rows[0];
        match first {
            Row::Header { uuid, tags, title, member_order, .. } => {
                assert_eq!(uuid, "note.md");
                assert_eq!(tags, "tag1,tag2");
                assert_eq!(title, "First doc");
                assert_eq!(member_order.render(), "b");
            }
            _ => panic!("expected Header"),
        }
    }

    #[test]
    fn content_row_has_five_columns() {
        let c = Container::parse(sample()).unwrap();
        match &c.rows[2] {
            Row::Content { file_order, section_type, indent, content, .. } => {
                assert_eq!(file_order.render(), "b");
                assert_eq!(section_type.as_str(), "h1");
                assert_eq!(*indent, 0);
                assert_eq!(content, "First heading");
            }
            _ => panic!("expected Content"),
        }
    }

    #[test]
    fn validate_rejects_content_without_header() {
        let bad = "b\ta\th1\t0\torphan\n";
        let c = Container::parse(bad).unwrap();
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_rejects_duplicate_uuid() {
        let bad = "\
a\tb\theader\tsame.md\t\tA
a\tc\theader\tsame.md\t\tB
";
        let c = Container::parse(bad).unwrap();
        assert!(c.validate().is_err());
    }

    #[test]
    fn parse_rejects_non_header_file_order_for_header_row() {
        // The parser catches this before validate() ever runs.
        let bad = "b\tc\theader\tx.md\t\tT\n";
        assert!(Container::parse(bad).is_err());
    }

    #[test]
    fn content_row_rejects_non_numeric_indent() {
        assert!(Container::parse("b\ta\th1\tnot\ttext\n").is_err());
    }

    #[test]
    fn header_row_must_have_six_columns() {
        // Only 5 columns — missing title.
        assert!(Container::parse("a\tb\theader\tu\ttags\n").is_err());
    }

    #[test]
    fn escape_round_trip_in_title() {
        let row = Row::Header {
            file_order: Order::parse("a").unwrap(),
            member_order: Order::parse("b").unwrap(),
            uuid: "x.md".into(),
            tags: "one,two".into(),
            title: "has\ttab and\nlf".into(),
        };
        let line = row.render();
        let parsed = Row::parse(&line, 1).unwrap();
        match parsed {
            Row::Header { title, .. } => assert_eq!(title, "has\ttab and\nlf"),
            _ => panic!("expected Header"),
        }
    }

    #[test]
    fn normalise_tags_replaces_spaces_with_underscore() {
        assert_eq!(
            normalise_tags("one, two words, three"),
            "one,two_words,three"
        );
    }
}
