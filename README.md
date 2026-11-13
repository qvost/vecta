# Vecta 🛡️

Binary regression profiler and ABI gatekeeper for compiled software teams (C, C++, Rust, Zig, Go).

## The Problem
Source diffs in Git pull requests hide what actually happens to compiled artifacts. A single dependency update or compiler flag tweak can silently:
- Bloat binary size by megabytes.
- Accidentally vendor heavy libraries statically.
- Bump minimum GLIBC requirements, causing production binaries to crash on older Linux servers.

## How Vecta Solves It
1. **`vecta inspect <binary>`**: Generates a deterministic JSON snapshot of binary sections, demangled symbols, dynamic library dependencies, and required runtime/GLIBC versions.
2. **`vecta diff <base> <head>`**: Compares two binaries or snapshots, computing exact byte deltas per section/symbol and catching ABI version bumps.
3. **`vecta gate <base> <head>`**: Acts as an automated CI gate enforcing size budgets and ABI compatibility rules (exiting with code 0 on pass, 1 on violation).
4. **Interactive Diff Viewer**: Generates compressed `#z=` URLs for visual treemap inspection.

## License
MIT OR Apache-2.0
