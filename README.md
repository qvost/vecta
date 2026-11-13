# Vecta

Vecta shows how a source change affects a compiled binary and enforces size and
runtime budgets in CI. Its Rust core reads ELF, thin Mach-O, and PE directly;
production commands never run `nm`, `objdump`, or other external programs.

```sh
cargo build --release
cargo install --path crates/vecta-cli
vecta inspect ./target/release/app --output base.json
vecta diff base.json ./target/release/app --format markdown --output report.md
vecta gate base.json ./target/release/app --max-growth 65536 \
  --section-budget .text=32768 --forbid-new-libraries --max-glibc 2.28
```

`inspect` always writes a deterministic, versioned JSON snapshot. `diff` and
`gate` accept binaries, snapshots, or one of each. Reports support `--format
terminal`, `markdown`, and `json`; `--top 20` controls human-readable symbol rows.
JSON retains every change. `--output PATH` writes a report without terminal color.
Terminal color is automatic on a TTY and respects `NO_COLOR`.

## Gates

| Flag | Policy |
|---|---|
| `--max-growth BYTES` | File-size growth, inclusive byte budget |
| `--max-growth-percent PERCENT` | File-size growth relative to the base file |
| `--section-budget NAME=BYTES` | Logical section growth; repeat for multiple sections |
| `--forbid-new-libraries` | Reject additional direct dynamic dependencies |
| `--max-glibc VERSION` | Cap the head's required GLIBC version |
| `--max-glibcxx VERSION` | Cap the head's required GLIBCXX version |
| `--forbid-abi-bump` | Reject numeric GLIBC or GLIBCXX requirement increases |

Exit status is **0** on success, **1** on a gate violation, **2** on malformed
input, unsupported input, invalid configuration, or an I/O error. A gate requires
at least one policy; all enabled policies must pass. Negative growth passes
nonnegative budgets. For a zero-byte base, a percentage budget permits zero growth.
GLIBC caps require ELF inputs. ABI policies fail if required version information
cannot be established or includes non-numeric GLIBC/GLIBCXX requirements.

```sh
# Produce a PR comment and keep a failing gate's exit code.
vecta gate base.json head.json --max-growth 4096 --max-glibc 2.28 \
  --format markdown --output vecta-report.md
```

The caller controls posting comments and artifacts; Vecta does not need GitHub
credentials, a running service, or a database.

## What the numbers mean

Snapshots include format, architecture, bitness, byte order, file size, logical
and on-disk section sizes, raw and demangled symbol names, symbol content hashes,
direct dynamic libraries, all GNU version-need names, and maximum numeric GLIBC
and GLIBCXX requirements. No timestamps or host paths are included.

Section deltas use logical sizes, so `.bss` growth is visible even though it does
not occupy file bytes. File growth includes headers, debug data, and padding.
Duplicate section names are summed. Symbols are matched by **raw name + section**;
duplicate keys are aggregated, and their content hashes are sorted. Symbols are
classified as added, removed, or modified and sorted by absolute byte delta, then
raw name and section for stable ties. BLAKE3 hashes detect same-size byte changes
when a symbol has a known size and file-backed data. Layout and relocation
changes can change those hashes without a source-level semantic change.

ELF uses the static symbol table or falls back to dynamic symbols with a warning.
Stripping loses attribution; it does not make a smaller runtime requirement.
Mach-O nlist and many PE symbols have no size: **zero means unknown**, not proof
that a symbol consumes no space. PDBs, DWARF type layout compatibility, C++ vtable
compatibility, exported API compatibility, and static library provenance are not
inferred. Statically linked libraries can be investigated through retained symbol
names and section growth, but their package names cannot reliably be recovered.
Dynamic dependencies are direct dependencies, not a transitive closure. PE DLL
names are normalized to lowercase.

Universal/fat Mach-O and archives are explicitly rejected: inspect one architecture
or executable at a time. Cross-format, cross-architecture, cross-endian, and
cross-bitness comparisons are rejected. GNU ABI requirements come from the linked
string table of `SHT_GNU_VERNEED` records, never a search of arbitrary strings or
the version definitions a shared library provides. They describe versioned imports,
not a guarantee that every kernel, CPU, loader, or external-library requirement is
satisfied on a target host.

## Shareable reports

```sh
vecta diff base.json head.json --viewer-url https://your-viewer.example/view
```

The returned URL uses `#z=` followed by **URL-safe unpadded Base64 of raw DEFLATE**
compressed JSON. The viewer is supplied by you; this repository implements the
payload contract, not a hosted viewer. Decode with `DecompressionStream('deflate-raw')`
or an equivalent raw-DEFLATE implementation, then parse JSON:

```json
{"schema_version":1,"diff":{},"omitted_symbols":0,"omitted_sections":0}
```

`diff` follows the JSON report schema. The payload includes up to 30 largest
symbol changes and changed sections. Warnings are omitted. Lowest-ranked symbols,
then section rows are removed as necessary; omission counters record this. ABI
and dynamic-library changes are retained. The **entire URL is capped at 8,000
ASCII bytes**; if fixed metadata alone cannot fit, sharing returns an error instead
of emitting an oversized or misleading URL. This is a conservative bound, not a
guarantee about every third-party Markdown renderer. Use JSON artifacts for large
reports. URL fragments contain binary-derived metadata; share them accordingly.

## Workspace and validation

- `crates/vecta-core`: parsing, demangling, snapshots, diffs, budgets, compression.
- `crates/vecta-cli`: Clap commands, terminal tables, Markdown and JSON reports.
- `scripts/smoke.py`: actual C binaries, stripped binaries, independent GNU
  `readelf` checks, gate exit codes, and share-payload decoding.
- `scripts/generate_fixtures.py`: reproducible minimal PE64 and Mach-O64 fixtures.

Dependencies: `object` provides the common section/symbol interface; `goblin`
provides ELF dependencies, Mach-O load commands, and PE imports; `rustc-demangle`
and `cpp_demangle` demangle Rust and Itanium C++ names (MSVC names retain their raw form); `blake3` fingerprints symbol bytes; `serde`
and `serde_json` provide snapshots; `flate2` and `base64` encode summaries;
`clap` and `comfy-table` implement the CLI presentation. Runtime version records
are traversed with checked offsets and explicit chain validation in the core.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
python3 scripts/smoke.py  # Linux; requires cc, strip, readelf, Python 3
```

See [VALIDATION.md](VALIDATION.md) for the validation status in the build environment.

Licensed under either MIT or Apache-2.0, at your option.
