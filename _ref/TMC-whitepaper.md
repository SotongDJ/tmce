# TMC — Tab-separate Markdown Consolidation container

**Version:** 0.1 Draft
**File extension:** `*.tmc`
**MIME type:** `text/tmc`

**Revision history:**
- 0.1 — initial release; two-shape schema (6-col header, 5-col content); letter-based escaping; `.ft.rtv` / `.tf.rtv` companion indexes

---

## 1. Overview

TMC is a single-line-per-record, flat-file container that consolidates many Markdown documents into one plain-text file. A TMC file is written for:

- **Archival consolidation** — many small Markdown notes stored as one git-traceable artefact
- **Lossless round-trip** — a Markdown file decomposed into TMC rows can be reassembled to Markdown without loss of structure
- **Human readability** — plain UTF-8 text, grep-friendly, diff-friendly
- **Tag-driven retrieval** — companion inverted indexes (`.ft.rtv`, `.tf.rtv`) let callers look up members by tag without scanning the container
- **Zero-copy parsing** — tab-splitting and line-splitting only; no tokenizer, no state machine

A `.tmc` file is a valid UTF-8 text file. Every tool that operates on text (grep, awk, diff, cat) works on TMC without modification.

---

## 2. Record Format

A TMC file contains two row shapes distinguished by the third column (`section_type`). The parser reads column 3 and dispatches on its value.

```
<header-row>   ::= a \t <member_order> \t header \t <uuid> \t <tags> \t <title> \n
<content-row>  ::= <file_order> \t <section_order> \t <section_type> \t <indent> \t <content> \n
```

Header rows carry metadata about member files; content rows carry the Markdown sections themselves. The two shapes are visually distinguishable: header rows always begin with the literal character `a`, followed by a tab.

### Example

```
a	b	header	note.md	ai,news	AI article title
a	c	header	NGk26dhizN01	ai,policy	Another article
b	a	h1	0	AI article title
b	b	content	0	A paragraph of body text.
b	c	ul	0	first bullet
b	d	ul	0	second bullet
c	a	h1	0	Another article
c	b	content	0	Another body paragraph.
```

---

## 3. Header Row (6 columns)

A header row describes one member file of the container. Every header row has `section_type = header` and `file_order = a`.

```
a \t <member_order> \t header \t <uuid> \t <tags> \t <title>
```

| Col | Name           | Specification                                                                 |
|-----|----------------|-------------------------------------------------------------------------------|
| 1   | `file_order`   | Literal character `a` (the header pseudo-file)                                |
| 2   | `member_order` | Base-52 order digits pointing to the member this row describes (see §5)       |
| 3   | `section_type` | Literal string `header`                                                       |
| 4   | `uuid`         | Member identifier — either a 12-char base62-Gu UUID or the member's filename  |
| 5   | `tags`         | Comma-separated tag list; spaces inside tags are normalised to `_` at write time |
| 6   | `title`        | Human-readable title, single line                                             |

### 3.1 `uuid` column

Two valid forms:

- **Base62-Gu UUID** — a 12-character time-sortable identifier (see Base62 whitepaper). Preferred when the member was generated programmatically.
- **Filename** — the basename of the source Markdown file (no directory path). Used when a member was ingested by filename.

A TMC container may mix both forms in the same file.

### 3.2 `tags` column

Tags are stored as a comma-separated string. At input time:

1. The user-supplied tag string is split on `,`.
2. Each tag has its internal whitespace collapsed to `_`.
3. The tags are re-joined with `,`.

`  draft, hello world,   design note ` → `draft,hello_world,design_note`

### 3.3 `title` column

Single-line human-readable title. Multi-line titles must use the tab/LF escapes described in §6.

---

## 4. Content Row (5 columns)

A content row carries one section of a member file's Markdown decomposition.

```
<file_order> \t <section_order> \t <section_type> \t <indent> \t <content>
```

| Col | Name           | Specification                                                                 |
|-----|----------------|-------------------------------------------------------------------------------|
| 1   | `file_order`   | Base-52 order digits; matches a `member_order` in some header row             |
| 2   | `section_order`| Base-52 order digits, scoped to this member file                              |
| 3   | `section_type` | Section kind (see §4.1)                                                       |
| 4   | `indent`       | Integer `0..=20` — leading-space count                                        |
| 5   | `content`      | Single-line section content                                                   |

### 4.1 Known `section_type` values

| Value       | Markdown construct                              |
|-------------|-------------------------------------------------|
| `h1`..`h6`  | ATX heading                                     |
| `content`   | Paragraph / plain text line                     |
| `quote`     | Block quote (`>` prefix stripped)               |
| `ul`        | Unordered list item (`-`, `*`, `+` prefix stripped) |
| `li`        | Ordered list item (numbering regenerated on view) |
| `codefence` | Fenced code block; `content` may span lines via `\n` escape |
| `hr`        | Horizontal rule (content is empty)              |

Unrecognised values are preserved verbatim so that round-tripping never drops data.

### 4.2 `indent` column

Integer in the range `0..=20`. Represents raw leading spaces observed in the source Markdown. A source file using 4-space indents will record `4`, `8`, `12`; a 2-space file will record `2`, `4`, `6`. The container does not normalise indent units.

### 4.3 `content` column

Single-line UTF-8 text with the escapes described in §6. For multi-line constructs (notably fenced code blocks) the interior newlines are represented as `\n` escape sequences, keeping the TMC record strictly single-line.

---

## 5. Order Alphabet (base-52)

Both `file_order`/`member_order` and `section_order` use a base-52 alphabet with big-endian digits.

```
digits:  a b c d e f g h i j k l m n o p q r s t u v w x y z
         A B C D E F G H I J K L M N O P Q R S T U V W X Y Z

values:  0 1 2 3 4 5 6 7 8 9 …                           … 51
```

### 5.1 Width

All orders within a single container column use the same width ("full digit"). If 52 or fewer positions are required, a single digit suffices (`a`..`Z`). Beyond 52, two digits are used (`aa`..`ZZ`, 2704 positions), and so on.

Header rows always use `file_order = a` at whatever width the container is using (single-digit `a`, double-digit `aa`, etc.).

### 5.2 Reserved header slot

Value 0 (all-lowest-digit: `a`, `aa`, `aaa`, …) is reserved for the header pseudo-file. Member files occupy positions starting at value 1 (`b`, `ab`, `aab`, …).

### 5.3 Ordering semantics

Lexicographic byte order over the 52-char alphabet matches numeric order: `a < b < … < z < A < B < … < Z`. Sorting TMC rows by `(file_order, section_order)` as byte strings is equivalent to sorting by numeric position.

---

## 6. Escaping

Four bytes require escaping inside any column's content. All other bytes — including spaces, CJK, emoji, `=`, `,` — pass through unchanged.

| Byte   | Escaped form |
|--------|--------------|
| `\t`   | `\t`         |
| `\n`   | `\n`         |
| `\r`   | `\r`         |
| `\`    | `\\`         |

The escape character is the ASCII backslash (`0x5C`). No other escape sequences are defined; an unknown `\?` sequence during unescape is an error.

**Note:** `,` is not escaped even though it is used as the array separator in `.ft.rtv` / `.tf.rtv` companion files. A tag containing a literal comma is stored verbatim in the TMC row but will collide with the array separator when the relate indexes are generated. Callers should avoid commas in tag values.

---

## 7. Round-trip Guarantees

A container `C` built by decomposing a Markdown file `M` via `tmce input` and then reassembling via `tmce view` produces Markdown text `M'` such that:

- Every heading level, paragraph, block quote, list item, code fence, and horizontal rule in `M` appears in `M'`.
- Section ordering is preserved.
- Fenced-code content is preserved byte-for-byte (via `\n` escaping).
- Numbering of ordered-list items is regenerated (`1.`, `2.`, `3.`) — the original digit is not preserved.
- Trailing whitespace on paragraph lines may be trimmed.

`M` and `M'` are not byte-identical in general but are semantically equivalent when rendered.

---

## 8. Companion Files

### 8.1 `<target>.ft.rtv` — file → tags

One row per member file, sorted by `uuid`.

```
<uuid> \t <tag1,tag2,...>
```

### 8.2 `<target>.tf.rtv` — tag → files

One row per distinct tag, sorted by tag.

```
<tag> \t <uuid1,uuid2,...>
```

Both files use the same escape rules as TMC proper (§6). They are regenerated from scratch by `tmce relate`; they are not hand-authored.

---

## 9. Parsing Strategy

```
for each line in file:
  cols[0..3] = line.splitn(4, '\t')       # file_order, member|section_order, section_type, rest
  if cols[2] == "header":
    rest.splitn(3, '\t')  →  uuid, tags, title     # 6 columns total
  else:
    rest.splitn(2, '\t')  →  indent, content       # 5 columns total
```

One byte check on column 3 decides the row shape. No lookahead, no state machine.

---

## 10. Design Rationale

| Goal                       | Mechanism                                                             |
|----------------------------|-----------------------------------------------------------------------|
| Plain-text readability     | UTF-8, tab-separated, line-oriented                                   |
| Human-editable             | Minimal escape set (4 bytes); no hex, no base64                       |
| Git-traceable              | One row per section; re-consolidation produces deterministic output   |
| Two shapes, one grammar    | Column 3 dispatch; same `splitn` machinery for both                   |
| Fast parsing               | No tokenizer; O(line_length) per row                                  |
| Tag-indexed lookup         | Companion `.ft.rtv` / `.tf.rtv` inverted indexes                      |
| Mixed identifier policy    | UUID or filename in the same `uuid` column — callers pick per member  |

---

## 11. Related Formats

| Format | Extension     | Full name                              | Role                                    | Created by     |
|--------|---------------|----------------------------------------|-----------------------------------------|----------------|
| `tmc`  | `*.tmc`       | Tab-separate Markdown Consolidation    | Consolidation container                 | `tmce input`   |
| `rtv`  | `*.ft.rtv`    | File-to-tag relation                   | uuid → tags inverted index              | `tmce relate`  |
| `rtv`  | `*.tf.rtv`    | Tag-to-file relation                   | tag → uuids inverted index              | `tmce relate`  |

The `.rtv` extension is shared with `tsdb`'s relation format; the `ft` / `tf` infix identifies the TMC-specific variants.
