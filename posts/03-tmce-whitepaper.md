<!--break type:header content-->
title = "tmce Whitepaper"
date = "2026-04-19 08:00:00+08:00"
short = ["tmce-whitepaper"]
categories = ["Whitepaper", "Reference"]
<!--break type:content format:md content-->
`tmce` is a command-line engine for TMC (`.tmc`) files — a tab-separated flat-file container that consolidates many Markdown documents into one UTF-8 text artefact. This whitepaper specifies the CLI: its six subcommands, its decomposition and recomposition behaviour, and its validation rules.

<!--excerpt-->

**Version:** 0.1 Draft
**Binary:** `tmce`
**Usage:** `tmce <subcommand> <target.tmc> [<args>]`

---

## 1. Overview

`tmce` decomposes Markdown sources into TMC content rows and reassembles them on demand. The container is plain text — grep-friendly, diff-friendly, editor-friendly — and `tmce` is the reference implementation for correctness (validation, ordering, compaction, index generation). See the companion TMC whitepaper for the on-disk record grammar.

Design principles:

- **Markdown stays authoritative** — the container is a lossless decomposition; `tmce view` reconstructs the original author-facing document.
- **Two row shapes, one parser** — column 3 (`section_type`) selects shape (6-col header vs. 5-col content). No tokenizer, no state machine.
- **Deterministic output** — save/compact always produce the same bytes for the same logical content, so `.tmc` diffs are readable in git.

---

## 2. Invocation

```
tmce input   <target.tmc> <source.md> <tags> [<base62-Gu-id>]
tmce list    <target.tmc>
tmce remove  <target.tmc> <uuid>
tmce view    <target.tmc> <uuid>
tmce compact <target.tmc>
tmce relate  <target.tmc>
```

| Form | Description |
|------|-------------|
| `tmce input <target.tmc> <source.md> <tags> [<id>]` | Decompose `source.md` and append it as a new member |
| `tmce list <target.tmc>` | Print the uuid column of every header row |
| `tmce remove <target.tmc> <uuid>` | Drop the named member and every matching content row |
| `tmce view <target.tmc> <uuid>` | Reassemble a member back to Markdown on stdout |
| `tmce compact <target.tmc>` | Validate, sort members by uuid, reassign `file_order`, re-validate |
| `tmce relate <target.tmc>` | Regenerate the sibling `.ft.rtv` and `.tf.rtv` indexes |

---

## 3. Markdown decomposition

`tmce input` walks `source.md` line by line and emits a flat sequence of `(section_type, indent, content)` triples:

| Input | `section_type` |
|-------|----------------|
| `# heading` … `###### heading` | `h1` … `h6` |
| fenced code block | `codefence` (interior `\n` preserved via escape) |
| `> quoted line` | `quote` |
| `- item`, `* item`, `+ item` | `ul` |
| `1. item`, `42) item` | `li` (ordered-list item; original digit not preserved) |
| `---`, `***`, `___` (3+ of a kind) | `hr` (empty content) |
| anything else non-blank | `content` |

Blank lines are section separators and not emitted as rows.

## 4. Markdown recomposition

`tmce view` inverts decomposition with one deliberate loss: ordered-list numbering is regenerated (`1.`, `2.`, `3.`) because the original digit is not stored. All other constructs round-trip structurally intact.

---

## 5. Validation

`tmce input`, `tmce remove`, and `tmce compact` validate before writing. Checks:

- Every header row has `file_order = a` at the container's width.
- `member_order` uuids are unique.
- Every content row's `file_order` matches some header row's `member_order`.
- `(file_order, section_order)` pairs are unique across content rows.
- `indent` is in `0..=20`.
- Base-52 orders use only `[a-z][A-Z]`.
- All orders within one column share the same width.

---

## 6. Escaping

Only four bytes require escaping — letter-based, not hex:

| Byte | Escaped |
|------|---------|
| `\t` | `\t` |
| `\n` | `\n` |
| `\r` | `\r` |
| `\` | `\\` |

Spaces, CJK, emoji, `=`, `,` all pass through untouched. An unknown `\?` sequence during unescape is a parse error.

---

## 7. Companion Indexes

`tmce relate` writes two sibling TSV files from scratch:

| File | Sort key | Row shape |
|------|----------|-----------|
| `<stem>.ft.rtv` | uuid | `<uuid>\t<tag1,tag2,…>` |
| `<stem>.tf.rtv` | tag | `<tag>\t<uuid1,uuid2,…>` |

Both are atomic (temp file + `rename`), so concurrent readers always see a complete version.

---

## 8. Workflow

```bash
tmce input notes.tmc article-1.md "ai,news"
tmce input notes.tmc article-2.md "ai,policy" NGk26dhizN01
tmce list  notes.tmc

tmce view notes.tmc article-1.md > restored.md

tmce compact notes.tmc   # renumber members by uuid; widen width if needed
tmce relate  notes.tmc   # refresh ft.rtv / tf.rtv
git add notes.tmc notes.ft.rtv notes.tf.rtv
```

Full specification: the [tmce Whitepaper](/tmce/tmce-whitepaper/) and the [TMC Whitepaper](/tmce/tmc-whitepaper/) on this site.
