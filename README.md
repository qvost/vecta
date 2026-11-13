# ◈ Vecta

[![CI](https://github.com/qvost/vecta/actions/workflows/ci.yml/badge.svg)](https://github.com/qvost/vecta/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Live Viewer](https://img.shields.io/badge/web%20viewer-vecta--gate.vercel.app-ffae00.svg)](https://vecta-gate.vercel.app)

> **Binary regression profiler and ABI gatekeeper for compiled software (C, C++, Rust, Zig, Go).**

Git pull requests show line-by-line source changes, but hide what happens to your compiled artifact. A single dependency update or compiler flag change can silently bloat `.text` by megabytes, accidentally vendor OpenSSL statically, or bump your minimum required GLIBC version so production binaries crash on older Linux servers.

**Vecta** inspects compiled binaries (ELF, Mach-O, PE), calculates exact section and demangled symbol deltas, tracks dynamic libraries, and enforces strict CI regression gates with shareable visual diffs.

---

## ⚡ Quick Start

### 1. Installation

Build and install the static Rust CLI:

```sh
# Clone and install globally into ~/.cargo/bin
git clone https://github.com/qvost/vecta.git
cd vecta
cargo install --path crates/vecta-cli
```

*(Ensure `~/.cargo/bin` is in your `$PATH`)*

---

## 🛠️ CLI Usage

Vecta operates through three core commands:

### `vecta inspect`
Dissects a binary and generates a deterministic, machine-readable JSON snapshot:
```sh
vecta inspect ./target/release/my-app --output base.json
```
- Extracts sections (`.text`, `.rodata`, `.data`, `.bss`).
- Demangles Rust and C++ symbols without shelling out to `nm` or `c++filt`.
- Records dynamic library dependencies (`DT_NEEDED` on ELF, `LC_LOAD_DYLIB` on Mach-O, PE import tables).
- Detects the exact minimum required GLIBC / GLIBCXX runtime versions from `.gnu.version_r`.

### `vecta diff`
Compares two binaries, two JSON snapshots, or one of each:
```sh
# Print a colored terminal diff
vecta diff base.json ./target/release/my-app

# Generate GitHub-flavored Markdown with a live web viewer URL
vecta diff base.json ./target/release/my-app \
  --format markdown \
  --viewer-url "https://vecta-gate.vercel.app"
```

### `vecta gate`
Runs as an automated CI quality gate. Evaluates diffs against strict budgets and **exits with code 0 on pass or code 1 on violation**:
```sh
vecta gate base.json ./target/release/my-app \
  --max-growth 65536 \
  --section-budget .text=32768 \
  --forbid-new-libraries \
  --max-glibc 2.31 \
  --format markdown \
  --output report.md
```

---

## 🛡️ Policy Gate Flags

| Flag | Policy Description |
|---|---|
| `--max-growth <BYTES>` | Maximum total binary growth in bytes |
| `--max-growth-percent <PERCENT>` | Maximum binary growth relative to the base file (e.g. `5.0` for 5%) |
| `--section-budget <NAME=BYTES>` | Per-section growth budget (e.g. `--section-budget .text=4096`; repeat for multiple) |
| `--forbid-new-libraries` | Fails if any new direct dynamic library (`DT_NEEDED`, dylib, DLL) is introduced |
| `--max-glibc <VERSION>` | Fails if head requires a GLIBC version higher than allowed (e.g. `2.31`) |
| `--max-glibcxx <VERSION>` | Fails if head requires a GLIBCXX version higher than allowed |
| `--forbid-abi-bump` | Fails if any required runtime version increased relative to base |

**Exit Codes:**
- `0`: All policies passed.
- `1`: Policy violation detected (budget exceeded, forbidden library added, etc.).
- `2`: Invalid arguments, unsupported binary format, or I/O failure.

---

## 🤖 Official GitHub Action

You can add Vecta directly to your repository's PR checks using our official composite action:

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
      - name: Checkout PR head
        uses: actions/checkout@v4
        with:
          path: head

      - name: Checkout base branch
        uses: actions/checkout@v4
        with:
          ref: ${{ github.base_ref }}
          path: base

      - name: Setup Rust / Build Binaries
        run: |
          cd base && cargo build --release
          cd ../head && cargo build --release

      - name: Run Vecta Gate
        uses: qvost/vecta@main
        with:
          base: base/target/release/my-app
          head: head/target/release/my-app
          max-growth-percent: 10.0
          forbid-new-libraries: "true"
          max-glibc: "2.31"
          viewer-url: "https://vecta-gate.vercel.app"
          comment-on-pr: "true"
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

When enabled, Vecta will automatically create or update a **sticky PR comment** containing the full Markdown diff table, status badges, and a direct link to the visual viewer.

---

## 🌐 The Web Viewer (`#z=` Zero-Backend Architecture)

Explore the live web viewer at **[vecta-gate.vercel.app](https://vecta-gate.vercel.app)**.

```
[vecta diff] ──(Deflate + Base64URL)──► URL Hash: https://vecta-gate.vercel.app#z=...
                                                            │
                                              [Browser Client Decompression]
                                                            │
                                              [Interactive 60fps Visualizer]
```

- **Zero Database / Zero Cloud Writes:** The diff summary is compressed using DEFLATE and encoded directly into the `#z=` URL fragment.
- **Privacy by Design:** URL hashes are never sent over the network to the server; your symbol names and memory addresses are decoded purely client-side inside the browser.
- **Editorial Motion Design:** Built with Vite, GSAP, Lenis momentum scrolling, Bellefair typography, and responsive scroll-pinned section cards.

### Running the Web Viewer Locally

```sh
cd web
npm install
npm run dev
```

Open `http://localhost:5173` in your browser.

---

## 📂 Repository Structure

- `crates/vecta-core`: Pure Rust binary parsing, symbol demangling, ABI extraction, diff calculation, and `#z=` compression.
- `crates/vecta-cli`: CLI interface (`vecta`), formatting terminal tables, Markdown reports, and exit code handling.
- `web/`: Motion-led static visual diff explorer deployed on Vercel.
- `action.yml`: Composite GitHub Action for CI gating and sticky PR comments.
- `scripts/smoke.py`: End-to-end integration test suite against real C binaries and `readelf`.

---

## 🧪 Testing & Validation

```sh
# Run code formatting and clippy checks
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings

# Run all unit and integration test suites
cargo test --workspace

# Run web viewer payload tests
cd web && node --test
```

---

## 📜 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
