<!--break type:header content-->
title = "TMC Whitepaper"
date = "2026-04-19 09:00:00+08:00"
short = ["tmc-whitepaper"]
categories = ["Whitepaper", "Reference"]
<!--break type:content format:md content-->
TMC (Tab-separate Markdown Consolidation container) is the on-disk file format operated on by `tmce`. A `.tmc` file consolidates many Markdown documents into one plain UTF-8 artefact with two row shapes — a 6-column header row per member file and a 5-column content row per Markdown section.

<!--excerpt-->

**Version:** 0.1 Draft
**File extension:** `*.tmc`
**MIME type:** `text/tmc`

---

## 1. Overview

A TMC file is a single-line-per-record, flat-file container written for:

- **Archival consolidation** — many small Markdown notes stored as one git-traceable artefact
- **Lossless round-trip** — decomposition followed by reassembly preserves structure
- **Human readability** — plain UTF-8 text, grep- and diff-friendly
- **Tag-driven retrieval** — companion `.ft.rtv` / `.tf.rtv` indexes let callers look up members by tag without scanning the container
- **Zero-copy parsing** — tab-splitting and line-splitting only; no tokenizer, no state machine

Every tool that operates on text (grep, awk, diff, cat) works on TMC without modification.

---

## 2. Record Format

Two row shapes distinguished by column 3 (`section_type`):

```
<header-row>   ::= a \t <member_order> \t header \t <uuid> \t <tags> \t <title> \n
<content-row>  ::= <file_order> \t <section_order> \t <section_type> \t <indent> \t <content> \n
```

Header rows always begin with the literal character `a`. Content rows use `file_order` values matching some header row's `member_order`.

### Example

```
a	b	header	note.md	ai,news	AI article title
a	c	header	NGk26dhizN01	ai,policy	Another article
b	a	h1	0	AI article title
b	b	content	0	A paragraph of body text.
b	c	ul	0	first bullet
c	a	h1	0	Another article
c	b	content	0	Another body paragraph.
```

---

## 3. Header Row (6 cols)

| Col | Name | Specification |
|-----|------|---------------|
| 1 | `file_order` | Literal `a` (the header pseudo-file) |
| 2 | `member_order` | Base-52 order digits pointing to the member this row describes |
| 3 | `section_type` | Literal string `header` |
| 4 | `uuid` | 12-char base62-Gu UUID **or** the member's filename basename |
| 5 | `tags` | Comma-separated tag list; internal whitespace normalised to `_` at write time |
| 6 | `title` | Single-line human-readable title |

A container may mix UUID and filename forms in the same file.

---

## 4. Content Row (5 cols)

| Col | Name | Specification |
|-----|------|---------------|
| 1 | `file_order` | Base-52 order digits matching a `member_order` in some header row |
| 2 | `section_order` | Base-52 order digits scoped to this member |
| 3 | `section_type` | Section kind (`h1`..`h6`, `content`, `quote`, `ul`, `li`, `codefence`, `hr`, …) |
| 4 | `indent` | Integer `0..=20` — leading-space count |
| 5 | `content` | Single-line section content (interior `\n` preserved via escape for codefences) |

Unknown `section_type` values are preserved verbatim so that round-tripping never drops data.

---

## 5. Base-52 Order Alphabet

```
digits:  a b c d e f g h i j k l m n o p q r s t u v w x y z
         A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
values:  0 1 2 3 4 5 6 7 8 9 …                           … 51
```

- **Width** — all orders in one column share the same width ("full digit"). Single-digit `a`..`Z` covers 52 positions; double-digit `aa`..`ZZ` covers 2704; and so on.
- **Reserved header slot** — value 0 (`a`, `aa`, `aaa`, …) is reserved for the header pseudo-file; members start at value 1.
- **Ordering** — lexicographic byte order equals numeric order.

---

## 6. Escaping

Four bytes require escaping inside any column:

| Byte | Escaped |
|------|---------|
| `\t` | `\t` |
| `\n` | `\n` |
| `\r` | `\r` |
| `\` | `\\` |

All other bytes pass through unchanged — spaces, CJK, emoji, `=`, `,` included. The one caveat: a literal `,` inside a tag collides with the array separator in `.ft.rtv` / `.tf.rtv`; avoid it.

---

## 7. Parsing Strategy

```
for each line in file:
  cols[0..3] = line.splitn(4, '\t')
  if cols[2] == "header":
    rest.splitn(3, '\t')  →  uuid, tags, title     # 6 columns total
  else:
    rest.splitn(2, '\t')  →  indent, content       # 5 columns total
```

One byte check on column 3 decides the row shape. No lookahead, no state machine.

---

## 8. Round-trip Guarantees

Decomposing a Markdown file `M` via `tmce input` and reassembling via `tmce view` produces `M'` such that:

- Every heading level, paragraph, block quote, list item, code fence, and horizontal rule in `M` appears in `M'`.
- Section ordering is preserved.
- Fenced-code content is preserved byte-for-byte (via `\n` escaping).
- Ordered-list numbering is regenerated (`1.`, `2.`, `3.`) — the original digit is not preserved.
- Trailing whitespace on paragraph lines may be trimmed.

`M` and `M'` are not byte-identical in general but are semantically equivalent when rendered.

---

## 9. Companion Files

| File | Sort key | Row shape |
|------|----------|-----------|
| `<stem>.ft.rtv` | uuid | `<uuid>\t<tag1,tag2,…>` |
| `<stem>.tf.rtv` | tag | `<tag>\t<uuid1,uuid2,…>` |

Both use the TMC escape rules and are regenerated from scratch by `tmce relate`.

---

## 10. Related Formats

| Format | Extension | Full name | Role | Created by |
|--------|-----------|-----------|------|------------|
| `tmc` | `*.tmc` | Tab-separate Markdown Consolidation | Consolidation container | `tmce input` |
| `rtv` | `*.ft.rtv` | File-to-tag relation | uuid → tags inverted index | `tmce relate` |
| `rtv` | `*.tf.rtv` | Tag-to-file relation | tag → uuids inverted index | `tmce relate` |

The `.rtv` extension is shared with `tsdb`'s relation format; the `ft` / `tf` infix identifies the TMC-specific variants.
