# Codebase Audit Report

## Latest Updates
- Removed unused const-interning scaffolding and duplicate macros; contributor guide refreshed.
- Bumpalo-backed allocator now reports capacity/usage from bumpalo and relies on bumpalo’s growth strategy.
- `IdentityHasher` panics on unsupported writes and exposes `write_u64`/`write_usize`.
- FFI entry rejects non-UTF-8 inputs; Miri script installs nightly without changing the default toolchain.
- Added optional `facet` feature for reflection; `Ustr` derives `Facet` when enabled.

## Earlier Notes
- Bumpalo migration preserved layout, sharded locking, and doubling growth; prior Miri runs showed no UB.
- Criterion benches, BLNS/raft fixtures, and serde gating remain the primary validation surface.
