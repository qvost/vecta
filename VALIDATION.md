# Validation status

Implementation and validation were performed in `/home/qvost/Projects/vecta`.
Full-workspace validation is **incomplete** because the environment cannot resolve
`index.crates.io`. The required analysis, formatting, serialization and compression
crates are not cached. `cargo check --workspace` fails during dependency resolution;
`cargo test --workspace --offline` reports that `comfy-table` is unavailable.
No successful full build, CLI invocation, Clippy run, or complete test-suite run is
claimed. A workspace `Cargo.lock` cannot be generated until resolution succeeds.

Completed checks:

- `cargo metadata --no-deps --format-version 1`: workspace and crate targets parse.
- `cargo fmt --all --check`: passes.
- `python3 -m py_compile scripts/generate_fixtures.py scripts/smoke.py`: passes.
- `scripts/generate_fixtures.py`: generates deterministic fixtures; `file` identifies
  them as Mach-O 64-bit x86_64 and PE32+ x86-64 with two sections.
- The actual `elf_versions.rs` source compiled in an isolated Cargo harness under
  ignored `target/validation-harness/`, using cached `anyhow`. Both embedded tests
  pass (little/big endian required-only extraction; malformed chains/truncation).
- A locally compiled C executable's version records were fed into that same core
  parser. Its output exactly matched GNU `readelf --version-info`:
  `GLIBC_2.2.5` and `GLIBC_2.34`. This validates that parser alone, not the full CLI.

Run these checks once dependency downloads are available:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
python3 scripts/smoke.py
```

The smoke script compiles base/head C binaries, compares every section size and
runtime requirement against `readelf`, checks symbol-size deltas and added dynamic
libraries, checks binary/snapshot report equivalence, exercises exit codes 0/1/2,
checks Markdown output, verifies stripped-binary fallback, inspects cross-format
fixtures, and independently decodes the compressed URL payload using Python zlib.
These checks are prepared but have not executed against a built Vecta CLI here.

Git milestone commits are local. Push attempts to `origin main` fail with
`Could not resolve host: github.com`; remote publication is unverified.
