# Vecta

[![CI](https://github.com/qvost/vecta/actions/workflows/ci.yml/badge.svg)](https://github.com/qvost/vecta/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Web Viewer](https://img.shields.io/badge/viewer-vecta--gate.vercel.app-informational.svg)](https://vecta-gate.vercel.app)

Binary regression profiler and ABI gatekeeper for compiled software (C, C++, Rust, Zig, Go).

Source diffs in pull requests show line-by-line changes, but hide compiled artifact regressions. Compiler flag adjustments, dependency bumps, or template instantiations can silently bloat code sections, introduce unexpected dynamic library dependencies, or raise the minimum required GLIBC version.

Vecta inspects compiled binaries (ELF, Mach-O, PE), extracts section sizes and demangled symbols, tracks runtime requirements, and acts as a CI quality gate with shareable, client-side visual reports.

---

## Installation

### Building from source

Prerequisites: Rust 1.75+ with Cargo.

```sh
git clone https://github.com/qvost/vecta.git
cd vecta
cargo install --path crates/vecta-cli
```

This builds and installs the `vecta` binary into `~/.cargo/bin`.

---

## CLI Usage

### `vecta inspect`

Generates a deterministic JSON snapshot of a binary's internal structure:

```sh
vecta inspect ./target/release/app --output base.json
```

- Analyzes sections (`.text`, `.rodata`, `.data`, `.bss`).
- Demangles Rust and Itanium C++ symbols without external tools (`nm`, `objdump`, `c++filt`).
- Records direct dynamic dependencies (`DT_NEEDED` on ELF, `LC_LOAD_DYLIB` on Mach-O, PE imports).
- Extracts minimum required GLIBC and GLIBCXX version requirements from `.gnu.version_r` records on Linux ELF binaries.

### `vecta diff`

Compares two binaries, two JSON snapshots, or one of each:

```sh
# Human-readable terminal table
vecta diff base.json ./target/release/app

# Generate GitHub-flavored Markdown with a shareable viewer link
vecta diff base.json ./target/release/app \
  --format markdown \
  --viewer-url "https://vecta-gate.vercel.app"
```

Supported output formats: `terminal` (default), `markdown`, and `json`.

### `vecta gate`

Evaluates binary differences against configured budgets and returns standard CI exit codes:

```sh
vecta gate base.json ./target/release/app \
  --max-growth 65536 \
  --section-budget .text=32768 \
  --forbid-new-libraries \
  --max-glibc 2.31 \
  --format markdown \
  --output report.md
```

Exit status:
- `0`: All policy checks passed.
- `1`: One or more policies were violated.
- `2`: Malformed input, unsupported architecture, or configuration error.

---

## Policy Options

| Option | Description |
|---|---|
| `--max-growth <BYTES>` | Maximum allowed file size growth in bytes |
| `--max-growth-percent <PERCENT>` | Maximum file size growth as a percentage of the base file |
| `--section-budget <NAME=BYTES>` | Per-section growth budget (e.g., `--section-budget .text=4096`; repeat for multiple sections) |
| `--forbid-new-libraries` | Reject addition of any new direct dynamic library dependencies |
| `--max-glibc <VERSION>` | Cap the head binary's minimum required GLIBC version (ELF only) |
| `--max-glibcxx <VERSION>` | Cap the head binary's minimum required GLIBCXX version (ELF only) |
| `--forbid-abi-bump` | Reject any increase in GLIBC or GLIBCXX version requirements |

---

## GitHub Action

Vecta provides a composite action for CI workflows. It runs `vecta gate` against your build artifacts and automatically creates or updates a sticky comment on the pull request:

```yaml
name: Binary Regression Gate

on:
  pull_request:
    branches: [main]

permissions:
  contents: read
  pull-requests: write

jobs:
  gate:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout PR branch
        uses: actions/checkout@v4
        with:
          path: head

      - name: Checkout base branch
        uses: actions/checkout@v4
        with:
          ref: ${{ github.base_ref }}
          path: base

      - name: Build binaries
        run: |
          cd base && cargo build --release
          cd ../head && cargo build --release

      - name: Evaluate regression gate
        uses: qvost/vecta@main
        with:
          base: base/target/release/app
          head: head/target/release/app
          max-growth-percent: 10.0
          forbid-new-libraries: "true"
          max-glibc: "2.31"
          viewer-url: "https://vecta-gate.vercel.app"
          comment-on-pr: "true"
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

---

## Web Viewer

A live instance of the visual diff explorer is available at [vecta-gate.vercel.app](https://vecta-gate.vercel.app).

The viewer operates entirely client-side:
- When `--viewer-url` is passed to `vecta diff` or `vecta gate`, the CLI compresses a diff summary using raw DEFLATE, base64url-encodes it, and appends it as a `#z=` URL hash.
- The browser decompresses the hash using `DecompressionStream`.
- No server-side database or storage is involved; binary metadata is never transmitted to the host.

### Running locally

```sh
cd web
npm install
npm run dev
```

---

## Technical Notes

- **Symbol attribution:** Stripped binaries lack static symbol tables (`.symtab`). Vecta falls back to dynamic symbols (`.dynsym`) and flags the report with a warning. For full attribution, run `vecta inspect` before running `strip`.
- **Logical vs. physical sizes:** Section budgets evaluate logical memory sizes, ensuring `.bss` changes are accounted for even when they occupy zero file bytes.
- **GLIBC extraction:** Version requirements are read directly from `SHT_GNU_VERNEED` sections in ELF binaries, extracting numeric version records without string matching heuristics.

---

## Workspace Layout

- `crates/vecta-core`: Parsing engine, symbol demangling, ABI extraction, diff calculation, and compression.
- `crates/vecta-cli`: CLI interface, table formatting, and exit code handling.
- `web/`: Client-side visual diff viewer (Vite, GSAP, Lenis).
- `action.yml`: Composite GitHub Action definition.
- `scripts/smoke.py`: Integration test harness against real compiled binaries.

---

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
