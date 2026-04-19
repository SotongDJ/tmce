//! Markdown → TMC content-row decomposition.
//!
//! Produces a flat list of (section_type, indent, content) triples in source
//! order. The caller assigns `file_order` and `section_order` when placing
//! them into a `Container`.
//!
//! This is an initial scaffold: it handles ATX headings (`#` … `######`),
//! fenced code blocks, block quotes, horizontal rules, unordered-list
//! bullets, ordered-list items rendered as `li`, and paragraph text as
//! `content`. Nested constructs and tables are left as TODO.

use crate::section::SectionType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDraft {
    pub section_type: SectionType,
    pub indent: u8,
    pub content: String,
}

/// Render a sequence of content sections back to Markdown.
///
/// A blank line is inserted between sections, except between consecutive
/// unordered-list bullets or consecutive ordered-list items (rendered as
/// `1. `). Ordered-list numbering is regenerated because the original
/// digit is not preserved in the TMC row.
pub fn recompose(drafts: &[ContentDraft]) -> String {
    let mut out = String::new();
    let mut ol_counter: u32 = 0;
    let mut prev: Option<&SectionType> = None;

    for (i, d) in drafts.iter().enumerate() {
        let is_li = matches!(d.section_type, SectionType::Li);
        let prev_li = matches!(prev, Some(SectionType::Li));
        let is_ul = matches!(d.section_type, SectionType::Ul);
        let prev_ul = matches!(prev, Some(SectionType::Ul));

        if i > 0 {
            let keep_together = (is_li && prev_li) || (is_ul && prev_ul);
            if keep_together {
                out.push('\n');
            } else {
                out.push_str("\n\n");
            }
        }

        if !(is_li && prev_li) {
            if is_li {
                ol_counter = 0;
            }
        }

        let indent: String = std::iter::repeat(' ').take(d.indent as usize).collect();

        match &d.section_type {
            SectionType::H1 => out.push_str(&format!("# {}", d.content)),
            SectionType::H2 => out.push_str(&format!("## {}", d.content)),
            SectionType::H3 => out.push_str(&format!("### {}", d.content)),
            SectionType::H4 => out.push_str(&format!("#### {}", d.content)),
            SectionType::H5 => out.push_str(&format!("##### {}", d.content)),
            SectionType::H6 => out.push_str(&format!("###### {}", d.content)),
            SectionType::Quote => {
                for (j, l) in d.content.split('\n').enumerate() {
                    if j > 0 {
                        out.push('\n');
                    }
                    out.push_str(&format!("{}> {}", indent, l));
                }
            }
            SectionType::Ul => out.push_str(&format!("{}- {}", indent, d.content)),
            SectionType::Li => {
                ol_counter += 1;
                out.push_str(&format!("{}{}. {}", indent, ol_counter, d.content));
            }
            SectionType::Hr => out.push_str("---"),
            SectionType::Codefence => {
                out.push_str("```\n");
                out.push_str(&d.content);
                out.push_str("\n```");
            }
            SectionType::Content => out.push_str(&format!("{}{}", indent, d.content)),
            SectionType::Header => {
                // Should not appear in content; skip defensively.
            }
            SectionType::Other(tag) => {
                // Preserve unknown types as a fenced block tagged with the name.
                out.push_str(&format!("```{}\n{}\n```", tag, d.content));
            }
        }

        prev = Some(&d.section_type);
    }

    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Decompose a markdown document into content-row drafts in source order.
pub fn decompose(md: &str) -> Vec<ContentDraft> {
    let mut out = Vec::new();
    let mut lines = md.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_end();

        if trimmed.is_empty() {
            continue;
        }

        // Horizontal rule: `---`, `***`, `___` (3+ of a kind).
        if is_hr(trimmed) {
            out.push(ContentDraft {
                section_type: SectionType::Hr,
                indent: 0,
                content: String::new(),
            });
            continue;
        }

        // Fenced code block.
        if let Some(fence) = fence_marker(trimmed) {
            let mut body = String::new();
            for l in lines.by_ref() {
                if l.trim_end() == fence {
                    break;
                }
                if !body.is_empty() {
                    body.push('\n');
                }
                body.push_str(l);
            }
            out.push(ContentDraft {
                section_type: SectionType::Codefence,
                indent: 0,
                content: body,
            });
            continue;
        }

        // ATX heading.
        if let Some((level, text)) = atx_heading(trimmed) {
            let section_type = match level {
                1 => SectionType::H1,
                2 => SectionType::H2,
                3 => SectionType::H3,
                4 => SectionType::H4,
                5 => SectionType::H5,
                _ => SectionType::H6,
            };
            out.push(ContentDraft {
                section_type,
                indent: 0,
                content: text.to_string(),
            });
            continue;
        }

        // Block quote.
        if let Some(text) = trimmed.strip_prefix("> ").or_else(|| trimmed.strip_prefix(">")) {
            out.push(ContentDraft {
                section_type: SectionType::Quote,
                indent: 0,
                content: text.to_string(),
            });
            continue;
        }

        // Unordered-list item: `- text`, `* text`, `+ text`.
        let indent = leading_spaces(line);
        let after_indent = &line[indent as usize..];
        if let Some(item) = ul_item(after_indent) {
            out.push(ContentDraft {
                section_type: SectionType::Ul,
                indent,
                content: item.to_string(),
            });
            continue;
        }

        // Ordered-list item rendered as `li`.
        if let Some(item) = ol_item(after_indent) {
            out.push(ContentDraft {
                section_type: SectionType::Li,
                indent,
                content: item.to_string(),
            });
            continue;
        }

        // Fallback: paragraph content.
        out.push(ContentDraft {
            section_type: SectionType::Content,
            indent,
            content: after_indent.trim_end().to_string(),
        });
    }

    out
}

fn is_hr(s: &str) -> bool {
    let s = s.trim();
    if s.len() < 3 {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !matches!(first, '-' | '*' | '_') {
        return false;
    }
    s.chars().all(|c| c == first || c == ' ')
}

fn fence_marker(s: &str) -> Option<String> {
    let s = s.trim_start();
    if s.starts_with("```") {
        Some("```".to_string())
    } else if s.starts_with("~~~") {
        Some("~~~".to_string())
    } else {
        None
    }
}

fn atx_heading(s: &str) -> Option<(usize, &str)> {
    let hashes: usize = s.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &s[hashes..];
    // Must be followed by space or end-of-line per CommonMark.
    let text = rest.strip_prefix(' ')?;
    Some((hashes, text.trim_end_matches('#').trim()))
}

fn ul_item(s: &str) -> Option<&str> {
    for prefix in ["- ", "* ", "+ "] {
        if let Some(rest) = s.strip_prefix(prefix) {
            return Some(rest.trim_end());
        }
    }
    None
}

fn ol_item(s: &str) -> Option<&str> {
    // Match `<digits>. ` or `<digits>) `.
    let digits: usize = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let rest = &s[digits..];
    let rest = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))?;
    Some(rest.trim_end())
}

fn leading_spaces(s: &str) -> u8 {
    let n = s.chars().take_while(|&c| c == ' ').count();
    n.min(crate::tmc::MAX_INDENT as usize) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_levels() {
        let drafts = decompose("# one\n## two\n### three");
        assert_eq!(drafts.len(), 3);
        assert_eq!(drafts[0].section_type.as_str(), "h1");
        assert_eq!(drafts[0].content, "one");
        assert_eq!(drafts[1].section_type.as_str(), "h2");
        assert_eq!(drafts[2].section_type.as_str(), "h3");
    }

    #[test]
    fn paragraph_falls_through() {
        let drafts = decompose("A line of text.");
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].section_type.as_str(), "content");
        assert_eq!(drafts[0].content, "A line of text.");
    }

    #[test]
    fn fenced_code_captured_as_single_section() {
        let md = "```\nlet x = 1;\nlet y = 2;\n```";
        let drafts = decompose(md);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].section_type.as_str(), "codefence");
        assert_eq!(drafts[0].content, "let x = 1;\nlet y = 2;");
    }

    #[test]
    fn ul_with_indent() {
        let md = "  - nested bullet";
        let drafts = decompose(md);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].section_type.as_str(), "ul");
        assert_eq!(drafts[0].indent, 2);
        assert_eq!(drafts[0].content, "nested bullet");
    }

    #[test]
    fn ol_rendered_as_li() {
        let drafts = decompose("1. first\n2. second");
        assert_eq!(drafts.len(), 2);
        assert!(drafts.iter().all(|d| d.section_type.as_str() == "li"));
    }

    #[test]
    fn hr_detected() {
        let drafts = decompose("---");
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].section_type.as_str(), "hr");
    }

    #[test]
    fn blockquote_prefix_stripped() {
        let drafts = decompose("> quoted line");
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].section_type.as_str(), "quote");
        assert_eq!(drafts[0].content, "quoted line");
    }

    #[test]
    fn recompose_round_trip_simple() {
        let md = "# Title\n\nA paragraph.\n\n## Sub\n\n- one\n- two\n";
        let rendered = recompose(&decompose(md));
        assert_eq!(rendered, md);
    }

    #[test]
    fn recompose_fenced_code() {
        let drafts = decompose("```\nfn main() {}\n```");
        let rendered = recompose(&drafts);
        assert_eq!(rendered, "```\nfn main() {}\n```\n");
    }

    #[test]
    fn recompose_numbers_li_sequentially() {
        let drafts = decompose("1. first\n2. second\n3. third");
        let rendered = recompose(&drafts);
        assert_eq!(rendered, "1. first\n2. second\n3. third\n");
    }
}
