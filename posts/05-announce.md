<!--break type:header content-->
title = "Announcing tmce v0.1"
date = "2026-04-19 12:00:00+08:00"
short = ["announce"]
categories = ["Announcement"]
<!--break type:content format:md content-->
`tmce` is the Tab-separate Markdown Consolidation Engine — a command-line tool that consolidates many Markdown documents into one UTF-8 flat-file container (`.tmc`) and maintains it over time. Today we publish the v0.1 specification along with two whitepapers: one for the TMC file format and one for the `tmce` CLI.

<!--excerpt-->

## What is tmce?

`tmce` decomposes Markdown sources into tab-separated records and stores them in a `.tmc` container alongside per-member metadata (uuid, tags, title). A single `.tmc` file can hold dozens or thousands of notes while remaining plain text that any editor, `grep`, `awk`, or `diff` can work on directly.

```
tmce input   notes.tmc article-1.md "ai,news"
tmce list    notes.tmc
tmce view    notes.tmc article-1.md
tmce compact notes.tmc
tmce relate  notes.tmc
```

## Why TMC?

The TMC container is built for three goals:

- **Archival consolidation** — many small Markdown notes stored as one git-traceable artefact. One commit per batch of changes, not one per file.
- **Lossless round-trip** — every block-level Markdown construct (`h1`..`h6`, paragraphs, block quotes, unordered lists, ordered lists, code fences, horizontal rules) is covered by a named `section_type`. Unknown types are preserved verbatim so round-tripping never drops data.
- **Tag-driven retrieval** — `tmce relate` regenerates two sibling inverted indexes (`.ft.rtv` for uuid → tags and `.tf.rtv` for tag → uuids) so callers can answer "which notes are tagged `policy`?" without scanning the container.

## Design Highlights

### Two row shapes, one parser

A `.tmc` file contains two row shapes distinguished by column 3 (`section_type`):

- **Header row (6 cols)** — one per member file: `a \t member_order \t header \t uuid \t tags \t title`
- **Content row (5 cols)** — one per Markdown section: `file_order \t section_order \t section_type \t indent \t content`

The parser splits on tab and dispatches on column 3's value. No tokenizer, no state machine, no lookahead.

### Letter-based escaping

Only four bytes ever need escaping — and they take their own letter, not a hex code:

| Byte | Escaped |
|------|---------|
| `\t` | `\t` |
| `\n` | `\n` |
| `\r` | `\r` |
| `\` | `\\` |

Spaces, CJK, emoji, `=`, `,` pass through untouched. Most real-world Markdown — including fenced code blocks that carry interior newlines — escapes at most one or two bytes per row.

### Base-52 orders

`file_order`, `member_order`, and `section_order` use a big-endian base-52 `[a-z][A-Z]` alphabet. Value 0 (`a`, `aa`, …) is reserved for the header pseudo-file; members start at value 1 (`b`, `ab`, …). All orders in one column share the same width — `tmce compact` widens cleanly when the alphabet runs out.

### Flexible identity

The `uuid` column accepts either a 12-character base62-Gu identifier or the source file's basename. A container may mix both forms freely, so callers can ingest by filename today and migrate to programmatic ids later without breaking compatibility.

## Round-trip Guarantees

`tmce view` reconstructs Markdown that is **semantically equivalent** — not byte-identical — to the source. Heading levels, paragraphs, block quotes, list items, code fences, and horizontal rules all reappear; section ordering is preserved; fenced-code content is preserved byte-for-byte via escape sequences. Ordered-list numbering is regenerated (`1.`, `2.`, `3.`) because the original digit is not stored. Trailing whitespace on paragraph lines may be trimmed.

## Build Instructions

Requires Rust 1.94+.

```bash
source "$HOME/.cargo/env"
cargo build --release
# binary: target/release/tmce
```

No runtime dependencies beyond the Rust standard library.

## Documentation

The full technical specifications are published as whitepapers on this site:

- [tmce Whitepaper](/tmce/tmce-whitepaper/) — CLI subcommands, decomposition/recomposition, validation
- [TMC Whitepaper](/tmce/tmc-whitepaper/) — on-disk record grammar, order alphabet, escaping, companion files
- [Base62 Encoding System Whitepaper](/tmce/base62-whitepaper/) — the UUID encoding system

The [Docs](/tmce/docs/) page is a concise technical reference for quick lookup.
