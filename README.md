# tmce

**Tab-separate Markdown Consolidation Engine** — a command-line tool that consolidates many Markdown documents into a single UTF-8 flat-file container (`.tmc`) and maintains it over time.

---

## Overview

`tmce` decomposes Markdown sources into tab-separated records, stores them in a `.tmc` container alongside per-member metadata, and reassembles them on demand. The container is plain text: grep-friendly, diff-friendly, editor-friendly.

```
tmce input   <target.tmc> <source.md> <tags> [<base62-Gu-id>]   append a member
tmce list    <target.tmc>                                       list member uuids
tmce remove  <target.tmc> <uuid>                                drop a member
tmce view    <target.tmc> <uuid>                                reassemble to Markdown
tmce compact <target.tmc>                                       renumber + widen members
tmce relate  <target.tmc>                                       regenerate .ft.rtv / .tf.rtv
```

---

## TMC File Format

A `.tmc` file has two row shapes, distinguished by column 3 (`section_type`).

### Header row (6 columns)

```
a\t<member_order>\theader\t<uuid>\t<tags>\t<title>\n
```

One per member. `file_order` is always literal `a`.

### Content row (5 columns)

```
<file_order>\t<section_order>\t<section_type>\t<indent>\t<content>\n
```

One per Markdown section.

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

### `section_type` values

| Value       | Markdown construct                                |
|-------------|---------------------------------------------------|
| `h1`..`h6`  | ATX heading                                       |
| `content`   | Paragraph / plain text                            |
| `quote`     | Block quote (`>` stripped)                        |
| `ul`        | Unordered list item (`-` / `*` / `+` stripped)    |
| `li`        | Ordered list item (numbering regenerated on view) |
| `codefence` | Fenced code block; interior `\n` preserved        |
| `hr`        | Horizontal rule (empty content)                   |
| `header`    | Reserved — marks a 6-column header row            |

Unknown values are preserved verbatim, so round-tripping never drops data.

### Base-52 orders

`file_order`, `member_order`, `section_order` use a big-endian base-52 alphabet `[a-z][A-Z]`. Value 0 (`a`, `aa`, …) is reserved for the header pseudo-file; member files start at value 1 (`b`, `ab`, …). All orders in one column share the same width; `tmce compact` widens cleanly when needed.

### Escaping

Only four bytes require escaping — letter-based, not hex:

| Byte | Escaped |
|------|---------|
| `\t` | `\t`    |
| `\n` | `\n`    |
| `\r` | `\r`    |
| `\`  | `\\`    |

Spaces, CJK, emoji, `=`, `,` all pass through untouched.

---

## Common Workflows

### First ingest

```bash
tmce input notes.tmc article-1.md "ai,news"
tmce input notes.tmc article-2.md "ai,policy" NGk26dhizN01
tmce list  notes.tmc
# article-1.md
# NGk26dhizN01
```

### Round-trip one member

```bash
tmce view notes.tmc article-1.md > restored.md
diff article-1.md restored.md
# structurally equivalent (may differ in trailing whitespace / OL digits)
```

### Replace a member

```bash
tmce remove notes.tmc article-2.md
tmce input  notes.tmc article-2.md "ai,policy,draft"
```

### Periodic housekeeping

```bash
tmce compact notes.tmc   # sort members by uuid, widen order width if needed
tmce relate  notes.tmc   # refresh companion indexes
git add notes.tmc notes.ft.rtv notes.tf.rtv
git commit -m "refresh notes"
```

### Tag-driven retrieval

```bash
awk -F'\t' '$1 == "policy" { print $2 }' notes.tf.rtv | tr ',' '\n'
```

---

## Companion Files

`tmce relate` produces two sibling inverted indexes:

| File              | Sort key | Row shape                             |
|-------------------|----------|---------------------------------------|
| `<stem>.ft.rtv`   | uuid     | `<uuid>\t<tag1,tag2,…>`              |
| `<stem>.tf.rtv`   | tag      | `<tag>\t<uuid1,uuid2,…>`             |

Both are plain TSV using the same escape rules as the `.tmc` itself. Writes are atomic (temp file + `rename`), so concurrent readers always see a complete version.

---

## Round-trip Guarantees

`tmce view` reconstructs Markdown that is **semantically equivalent** — not byte-identical — to the ingested source:

- Heading levels, paragraphs, block quotes, list items, code fences, and horizontal rules all reappear.
- Section ordering is preserved.
- Fenced-code content is preserved byte-for-byte.
- Ordered-list numbering is regenerated (`1.`, `2.`, `3.`) — the original digit is not stored.
- Trailing whitespace on paragraph lines may be trimmed.

---

## Building

Requires Rust 1.94+.

```bash
source "$HOME/.cargo/env"
cargo build --release
# binary: target/release/tmce
```

No runtime dependencies beyond the Rust standard library.

---

## Whitepapers and References

Full specifications are in `_ref/`:

- `_ref/TMC-whitepaper.md` — on-disk record grammar, order alphabet, escaping, companion files
- `_ref/tmce-whitepaper.md` — CLI subcommands, decomposition/recomposition, validation
- `_ref/base62-whitepaper.md` — base62-Gu UUID encoding
