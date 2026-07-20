# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`tmce` is the **Tab-separate Markdown Consolidation Engine** — a Rust CLI tool that consolidates many Markdown documents into a single tab-separated flat-file container (`.tmc`) and maintains it over time. The repository scaffolding was seeded from the `tsdb` project (DOTSV database runner) and then fully rewritten; TMC shares the plain-text, zero-copy-parser philosophy but defines its own record grammar (two row shapes), escape rules (letter-based), and companion index formats (`.ft.rtv` / `.tf.rtv`).

## Environment Setup

Rust 1.94.1 is installed but requires sourcing before use:

```bash
source "$HOME/.cargo/env"
```

Add `. "$HOME/.cargo/env"` to `~/.bashrc` to make this permanent.

## Build Commands

```bash
cargo build                        # debug build
cargo build --release              # production build → target/release/tmce
cargo test                         # run all tests
cargo test <test_name>             # run a single test
cargo fmt                          # format code
cargo clippy                       # lint
cargo mutants                      # mutation testing (gitignore includes mutants.out/)
```

Runtime dependencies: none beyond the Rust standard library.

## Architecture

### TMC File Format (`_ref/TMC-whitepaper.md`)

Plain UTF-8 flat files (`.tmc` extension). Two row shapes distinguished by column 3 (`section_type`):

- **Header row (6 cols)** — `a\t<member_order>\theader\t<uuid>\t<tags>\t<title>\n`. One per member file.
- **Content row (5 cols)** — `<file_order>\t<section_order>\t<section_type>\t<indent>\t<content>\n`. One per Markdown section.

Orders use a base-52 `[a-z][A-Z]` alphabet; value 0 (`a`, `aa`, …) is reserved for the header pseudo-file. Only 4 bytes require escaping: `\t`, `\n`, `\r`, `\` — letter-based, not hex.

### tmce CLI (`_ref/tmce-whitepaper.md`)

Six subcommands: `input`, `list`, `remove`, `view`, `compact`, `relate`. `input` decomposes a Markdown source into content rows and appends a header. `view` reassembles a member back to Markdown (ordered-list numbering is regenerated). `compact` sorts members by uuid and reassigns `file_order`. `relate` writes `<stem>.ft.rtv` (uuid → tags) and `<stem>.tf.rtv` (tag → uuids).

### Base62-Gu UUID System (`_ref/base62-whitepaper.md`)

12-character time-sortable identifiers using a 60-char alphabet (standard base62 minus ambiguous `l` and `O`). Structure: `{class}{G}{century}{year}{month}{day}{hour}{minute}{second}{order2}`. The `uuid` column of a TMC header row accepts either a base62-Gu id or a plain filename basename.

### Source Layout

| File | Responsibility |
|------|---------------|
| `src/main.rs` | CLI entry point, subcommand dispatch, orchestration |
| `src/tmc.rs` | TMC row enum, container load/save/validate, tag normalisation |
| `src/markdown.rs` | Markdown decomposition / recomposition |
| `src/section.rs` | `SectionType` enum + parse |
| `src/order.rs` | Base-52 order arithmetic |
| `src/escape.rs` | Letter-based TMC escape / unescape |
| `src/base62.rs` | Base62-Gu UUID validation |
| `src/error.rs` | Unified `TmceError` enum |

### Key Design Principles

- **Two shapes, one parser:** column 3 dispatch; `splitn` for the rest — no tokenizer, no state machine
- **Stream processing:** constant memory regardless of file size
- **Deterministic output:** stable row ordering + canonical tag normalisation + atomic writes = readable git diffs
- **Lossless round-trip:** every Markdown block construct is covered by a `section_type`; unknowns are preserved verbatim

## Shorthand Commands

| Command | Meaning |
|---------|---------|
| `kk` | (1) Find every `?vTimestamp` or `?v{14-digit-timestamp}` in the project (e.g. `include_files/head.html`) and replace with `?v{YYYYDDMMhhmmss}` using current time. (2) `git add -A`. (3) `git commit -S` with a descriptive message. |

## MalTHA Website Files

MalTHA is the static site generator used for this project. Page files are in `page_files/`, layouts in `layout_files/`, includes in `include_files/`, site config in `config.toml`.

Whitepaper pages are live mirrors of the `_ref/` sources. Always read the current source before creating or updating:

- `_ref/DOTSV-whitepaper.md` → `page_files/05-dotsv-whitepaper.html`
- `_ref/tmce-whitepaper.md` → `page_files/06-tmce-whitepaper.html`
- `_ref/TMC-whitepaper.md`  → `page_files/07-tmc-whitepaper.html`

## Build discipline

- Build with `./build.sh`. On the maintainer's host it routes through a
  machine-wide build guard that serialises Rust/Android builds (one at a time)
  and registers build intermediates for a 24-hour cache expiry; compiled
  executables are never removed by that cleanup. On any other machine the
  script is equivalent to `cargo build --release`.
- Prefer `./build.sh` over invoking cargo directly so the serialisation holds
  wherever the guard is present.
