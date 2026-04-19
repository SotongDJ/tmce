# tmce — Agent Reference for Handling `.tmc` Containers

`tmce` consolidates many Markdown documents into a single tab-separated flat-file container (`.tmc`). This document ships alongside the `tmce` binary and gives another agent everything it needs to operate on `.tmc` files — CLI usage, recipes, structure, error modes, and UUID generation.

The binary is invoked as `tmce` (on PATH) or by its path on disk. No runtime dependencies.

---

## 1. CLI summary

```
tmce input   <target.tmc> <source.md> <tags> [<base62-Gu-id>]
tmce list    <target.tmc>
tmce remove  <target.tmc> <uuid>
tmce view    <target.tmc> <uuid>
tmce compact <target.tmc>
tmce relate  <target.tmc>
```

| Subcommand | Stdin | Stdout | Stderr | Writes | Reads |
|------------|-------|--------|--------|--------|-------|
| `input` | — | — | `Added <uuid> (N content rows) to <target>` | `<target.tmc>` (creates if missing) | `<source.md>` |
| `list` | — | uuids, one per line (header-declaration order) | — | — | `<target.tmc>` |
| `remove` | — | — | `Removed <uuid> from <target>` | `<target.tmc>` | `<target.tmc>` |
| `view` | — | reconstructed Markdown | — | — | `<target.tmc>` |
| `compact` | — | — | `Compacted <target>: N members` | `<target.tmc>` | `<target.tmc>` |
| `relate` | — | — | `Wrote <stem>.ft.rtv (N rows) and <stem>.tf.rtv (M rows)` | `<stem>.ft.rtv`, `<stem>.tf.rtv` | `<target.tmc>` |

**Exit codes**

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Runtime error (parse, validation, missing member, duplicate member, I/O, invalid UUID, unknown escape). Message on stderr as `Error: <detail>` |
| `2` | Usage error (wrong arg count / unknown subcommand). Usage help printed to stderr |

All writes are atomic: temp file written then `rename`d over the target. Readers never see a partial file. `input`, `remove`, `compact` validate before writing and abort with exit 1 on invariant violations.

---

## 2. Recipes

### 2.1 Ingest a single Markdown file

```bash
tmce input notes.tmc article-1.md "ai,news"
```

Uuid defaults to the basename (`article-1.md`). Tags are comma-separated. Title is taken from the first `# heading`; if none, falls back to the uuid.

### 2.2 Ingest with an explicit UUID

```bash
tmce input notes.tmc article-1.md "ai,news" NGk26dhizN01
```

The 4th arg is a 12-char base62-Gu id (see §5). The CLI validates it; an invalid id aborts with exit 1.

### 2.3 Update an existing note (re-ingest)

There is no in-place edit subcommand. To replace a member: remove, then input.

```bash
tmce remove notes.tmc article-1.md
tmce input  notes.tmc article-1.md "ai,news,revised"
tmce relate notes.tmc   # refresh indexes
```

### 2.4 Retrieve a member as Markdown

```bash
tmce view notes.tmc article-1.md > restored.md
```

The reconstruction is semantically equivalent, not byte-identical. See §4.

### 2.5 List all members

```bash
tmce list notes.tmc                  # one uuid per line, declaration order
tmce list notes.tmc | sort           # alphabetical
tmce list notes.tmc | wc -l          # member count
```

### 2.6 Look up notes by tag (without scanning the container)

```bash
tmce relate notes.tmc                # ensure indexes are fresh
grep -P '^policy\t' notes.tf.rtv     # uuids tagged "policy"
grep -P '^article-1\.md\t' notes.ft.rtv  # tags on one uuid
```

The `.tf.rtv` and `.ft.rtv` files are plain TSV and can be parsed directly; see §6.

### 2.7 Bulk ingest and rebuild

```bash
for md in notes/*.md; do
  tmce input notes.tmc "$md" "$(...derive tags...)"
done
tmce compact notes.tmc               # renumber members by uuid
tmce relate  notes.tmc               # regenerate indexes
git add notes.tmc notes.ft.rtv notes.tf.rtv
```

### 2.8 Check whether a uuid exists

```bash
if tmce list notes.tmc | grep -qxF "article-1.md"; then
  echo "present"
fi
```

Or try `tmce view` and check the exit code: exit 1 with `Error: Missing member: ...` if absent.

### 2.9 Widen capacity when the container fills up

`tmce input` chooses the smallest order width that fits. If you hit:

```
Error: member slot N exceeds current width W; run `tmce compact` to widen
```

…run `tmce compact` — it recomputes the width from member count and rewrites all `file_order` / `member_order` digits.

---

## 3. `.tmc` container structure

UTF-8 text. One record per line, `\n`-terminated. Two row shapes distinguished by column 3 (`section_type`).

### 3.1 Header row (6 columns)

```
a \t <member_order> \t header \t <uuid> \t <tags> \t <title>
```

| Col | Name | Meaning |
|-----|------|---------|
| 1 | `file_order` | Literal `a` (at the container's width, so e.g. `aa` for width-2) — the header pseudo-file |
| 2 | `member_order` | Base-52 order pointing to this member; members start at value 1 (`b`, `ab`, …) |
| 3 | `section_type` | Literal `header` |
| 4 | `uuid` | 12-char base62-Gu id **or** source file basename |
| 5 | `tags` | `tag1,tag2,…` — interior whitespace normalised to `_`; commas inside a tag are forbidden |
| 6 | `title` | Single-line human-readable title (first `# ` heading, or the uuid) |

### 3.2 Content row (5 columns)

```
<file_order> \t <section_order> \t <section_type> \t <indent> \t <content>
```

| Col | Name | Meaning |
|-----|------|---------|
| 1 | `file_order` | Matches some header row's `member_order` |
| 2 | `section_order` | Base-52 order scoped to this member; starts at `a` |
| 3 | `section_type` | `h1`..`h6`, `content`, `quote`, `ul`, `li`, `codefence`, `hr`, or unknown (preserved verbatim) |
| 4 | `indent` | Integer `0..=20` — leading-space count |
| 5 | `content` | Single-line section content; interior `\n` preserved via escape (mainly for codefences) |

### 3.3 Concrete example

```
a	b	header	article-1.md	ai,news	AI article title
a	c	header	NGk26dhizN01	ai,policy	Another article
b	a	h1	0	AI article title
b	b	content	0	A paragraph of body text.
b	c	ul	0	first bullet
b	d	ul	0	second bullet
c	a	h1	0	Another article
c	b	content	0	Another body paragraph.
```

Two members. Tabs between columns; each row is one line.

### 3.4 Base-52 order alphabet

```
digits:  a b c d e f g h i j k l m n o p q r s t u v w x y z
         A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
values:  0 1 2 3 4 5 6 7 8 9 …                           … 51
```

- **Width** — all orders in one column share the same width. 1-digit covers 52 positions, 2-digit `aa..ZZ` covers 2704, etc.
- **Reserved** — value 0 (`a`, `aa`, …) is the header pseudo-file's `file_order`. Members start at value 1.
- **Ordering** — lexicographic byte order equals numeric order.

### 3.5 Section-type vocabulary

| Input Markdown | `section_type` |
|----------------|----------------|
| `# heading` … `###### heading` | `h1` … `h6` |
| fenced code block (```` ``` ````) | `codefence` (interior `\n` preserved via escape) |
| `> quoted line` | `quote` |
| `- item`, `* item`, `+ item` | `ul` |
| `1. item`, `42) item` | `li` (ordered-list item; original digit not preserved — `view` regenerates `1.`, `2.`, …) |
| `---`, `***`, `___` (3+ of a kind) | `hr` (empty content) |
| any other non-blank line | `content` |

Blank lines are section separators (not emitted as rows). Unknown `section_type` strings on read are preserved verbatim and emitted by `view` as a fenced block tagged with the name.

### 3.6 Escaping

Only four bytes are escaped, in every column, letter-based (not hex):

| Byte | Escaped |
|------|---------|
| `\t` | `\t` |
| `\n` | `\n` |
| `\r` | `\r` |
| `\` | `\\` |

Everything else — spaces, CJK, emoji, `=`, `,`, quotes — passes through untouched. An unknown `\?` during unescape is a parse error (exit 1).

---

## 4. Round-trip behaviour

`tmce view` reconstructs Markdown that is **semantically equivalent**, not byte-identical, to the original:

- Headings, paragraphs, block quotes, list items, code fences, and horizontal rules all reappear.
- Section ordering is preserved.
- Fenced-code content is byte-for-byte preserved via `\n` escaping.
- Ordered-list numbering is regenerated as `1.`, `2.`, `3.` — original digits are not stored.
- Trailing whitespace on paragraph lines may be trimmed.
- Consecutive `ul` (or consecutive `li`) sections are rendered as a contiguous list with a single `\n` separator; other adjacent sections get `\n\n`.

---

## 5. Base62-Gu UUID generation (for the `<id>` arg of `tmce input`)

A Format-Gu UUID is **12 characters**, fixed-width, filename- and URL-safe. The CLI accepts either a Format-Gu id or a plain filename basename. You only need to generate one if you want programmatic, time-sortable ids rather than filename-based ones.

### 5.1 Structure

```
{C}G{century}{year2d}{month}{day}{hour}{minute}{second}{order}{order}
 1 2  3       4-5    6      7    8     9       10      11     12
```

| Pos | Meaning |
|-----|---------|
| 1 | Class indicator — any uppercase letter (A–Z). Identifies record type (e.g. `N` for note, `E` for expense). User-defined. |
| 2 | Literal `G` — format marker. |
| 3 | Century = `year // 100` → char from the 60-char alphabet (see below). |
| 4–5 | Two decimal digits of `year % 100`, zero-padded. |
| 6 | Month 1–12. Custom table (below). |
| 7 | Day 1–31. Custom table. |
| 8 | Hour 0–23. Custom table. |
| 9 | Minute 0–59. Positional lookup in the 60-char alphabet. |
| 10 | Second 0–59. Positional lookup in the 60-char alphabet. |
| 11–12 | Order number, default `01`. Increment on collision: `01..09`, `0a..0z`, `0A..0Z`, `10..19`, … (62×62 values). |

### 5.2 The 60-char alphabet

```
0123456789abcdefghijkmnopqrstuvwxyzABCDEFGHIJKLMNPQRSTUVWXYZ
```

Formed by removing `l` (ambiguous with `1`) and `O` (ambiguous with `0`) from the standard base62 alphabet. Positional lookup: value *n* maps to the *n*-th char of this string.

### 5.3 Segment tables

**Month (1–12):** `a b c d e f A B C D E F` (Jan=`a`, Jun=`f`, Jul=`A`, Dec=`F`).

**Day (1–31):** 1–10 → `0`–`9`; 11–21 → `a`–`k`; 22–31 → `A`–`J`.

**Hour (0–23):** 0 → `0`; 1–12 → `a`–`l` (note: `l` is valid here — the exclusion applies only to the minute/second alphabet); 13–23 → `A`–`K`.

**Minute and second (0–59):** positional lookup in the 60-char alphabet.

### 5.4 Worked example

Input: `2026-03-29 03:30:00`, class `N` (note), first record.

| Pos | Value | Encoding |
|-----|-------|----------|
| Class | N | `N` |
| Format | — | `G` |
| Century | 2026 // 100 = 20 | alphabet[20] = `k` |
| Year-2d | 2026 % 100 = 26 | `"26"` |
| Month | 3 (Mar) | month-table[3] = `c` |
| Day | 29 | day-table[29] = `H` |
| Hour | 3 | hour-table[3] = `c` |
| Minute | 30 | alphabet[30] = `v` |
| Second | 0 | alphabet[0] = `0` |
| Order | 1 | `01` |

Result: `NGk26cHcv001`.

### 5.5 Regex for validation

```
^[A-Z]G[0-9a-km-zA-NP-Z]{8}[0-9a-zA-Z]{2}$
```

Full spec: `_ref/base62-whitepaper.md` in this repo.

---

## 6. Companion index files (`.ft.rtv` and `.tf.rtv`)

Generated from scratch by `tmce relate`. Plain TSV, same escape rules as `.tmc`. Always sorted.

### 6.1 `<stem>.ft.rtv` — file → tags

Sort key: uuid. One line per member:

```
<uuid>\t<tag1>,<tag2>,...
```

Example:

```
NGk26dhizN01	ai,policy
article-1.md	ai,news
article-2.md	ai,research
```

### 6.2 `<stem>.tf.rtv` — tag → files

Sort key: tag. One line per tag:

```
<tag>\t<uuid1>,<uuid2>,...
```

Example:

```
ai	NGk26dhizN01,article-1.md,article-2.md
news	article-1.md
policy	NGk26dhizN01
research	article-2.md
```

### 6.3 Regenerating

Always regenerated in full — never incrementally patched. Atomic write (temp + rename). Run after any `input`, `remove`, or `compact` if you rely on the indexes.

### 6.4 Caveat: commas in tags

A literal `,` inside a tag would collide with the array separator in both files. The CLI does not reject such tags but will corrupt the indexes. **Do not put `,` inside a tag.** Commas inside `uuid` or `title` are safe (not parsed as separators in `.tmc`).

---

## 7. Validation rules (enforced by `input`, `remove`, `compact`)

- Every header row's `file_order` equals the header pseudo-file at the container's width (all lowest digit).
- Every `member_order` / `uuid` is unique.
- Every content row's `file_order` matches some header row's `member_order`.
- Every `(file_order, section_order)` pair is unique.
- `indent` is in `0..=20`.
- All base-52 orders use only `[a-z][A-Z]`.
- All orders in one column share the same width.

Failures surface as `Error: <detail>` on stderr with exit 1.

---

## 8. Common failure modes

| stderr message | Cause | Fix |
|----------------|-------|-----|
| `Error: Missing member: <uuid>` | `view` / `remove` on a uuid not in the container | Check with `tmce list` |
| `Error: Duplicate member: <uuid>` | `input` with a uuid that already exists | `remove` first, or use a different uuid |
| `Error: Invalid UUID: <id>` | 4th arg to `input` fails the Format-Gu regex | Fix the id or omit to fall back to basename |
| `Error: member slot N exceeds current width W; run \`tmce compact\` to widen` | Adding another member needs a wider order column | Run `tmce compact` |
| `Error: Parse error at line N: ...` | `.tmc` file is malformed | Inspect the offending line; check for unescaped tabs/newlines |
| `Error: Invalid order digit: ...` | Row contains a non-base-52 char in an order column | Inspect / repair the row |
| `Error: I/O error: ...` | File permission, missing path, etc. | Check paths and permissions |
| Usage help printed, exit 2 | Wrong arg count or unknown subcommand | Fix the command line |

---

## 9. Integration patterns

### 9.1 Agent workflow — "append to an archive"

```bash
tmce input archive.tmc "$new_note" "$tags" && tmce relate archive.tmc
```

No-op if `input` exits non-zero (Bash short-circuit on `&&`). If the archive is committed to git, `git diff archive.tmc` shows a clean line-oriented diff.

### 9.2 Agent workflow — "fetch and transform a note"

```bash
tmce view archive.tmc "$uuid" | your-filter > /tmp/processed.md
```

Stream-friendly: no need to load the whole archive into memory (the CLI does that internally, but the output is just the one member's Markdown).

### 9.3 Agent workflow — "answer 'what is tagged X'"

```bash
awk -F'\t' -v tag="$target_tag" '$1 == tag { split($2, a, ","); for (i in a) print a[i] }' archive.tf.rtv
```

The `.tf.rtv` answers in O(members-with-tag) without touching the container.

### 9.4 Concurrency

All writes are atomic via temp + rename; concurrent readers always see a complete version. Concurrent writers are **not** coordinated — last writer wins. If multiple agents write to the same `.tmc`, serialise with an external lock (e.g. `flock`).

---

## 10. Deeper references

Full whitepapers are published at `https://sotongdj.github.io/tmce/` and in the source repo at `github.com/SotongDJ/tmce` under `_ref/`:

- `tmce-whitepaper.md` — full CLI spec
- `TMC-whitepaper.md` — on-disk record grammar
- `base62-whitepaper.md` — Format-A through Format-Gu UUID system
- `DOTSV-whitepaper.md` — sibling `.rtv` format (the `.ft.rtv` / `.tf.rtv` escape conventions)
