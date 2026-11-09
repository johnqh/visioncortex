# CLAUDE.md

Guidance for working in this repository.

## What this is

`visioncortex` — a Rust library (single crate, `src/lib.rs`) of computer
graphics/vision building blocks: color/binary images, hierarchical color
clustering, path tracing + simplification, shape analysis, polygon
triangulation. Core library behind VTracer.

This repo is a **fork** of upstream `visioncortex/visioncortex` (Cargo
`repository`/`homepage` still point upstream). Local additions over upstream
live in `CHANGELOG.md`: `polypartition` module, color-cluster perf work,
type-state `color_clusters::Builder`, Bezier-fit and `u32`-label fixes.

## Stack / toolchain

- Rust, edition 2021. Verified with cargo/rustc 1.98.
- Deps: `bit-vec`, `num-traits`, `flo_curves` (curve math). `image` is a
  **dev-dependency only** (tests/examples); the library has no image decoder.

## Commands (verified)

```sh
cargo build                 # build
cargo test                  # full test suite
cargo clippy                # lint (clippy 0.1.98 available)
cargo fmt                   # format (no rustfmt.toml; defaults)
cargo doc                   # API docs
```

CI (`.github/workflows/rust.yml`) runs `cargo build --verbose` and
`cargo test --verbose` on push/PR to `main`.

## Gotchas

- **Parity snapshots**: `tests/color_clusters_parity.rs` asserts the clustering
  pipeline output byte-for-byte against goldens in `tests/parity/`. The pipeline
  is fully deterministic (no RNG, no threads). Any change to clustering logic
  that alters output breaks these. Regenerate goldens ONLY from known-good code:
  `VISIONCORTEX_BLESS_PARITY=1 cargo test --test color_clusters_parity`.
- **`profile-stage2` feature** (off by default): compiles operation counters +
  timers into color_clusters stage 2 for profiling only. Needed by
  `examples/profile_stage2.rs`; without it those counters read zero.
- `color_clusters::Builder` is a **type-state builder** — all closures must be
  set before `run()`/`start()`, enforced at compile time (was a runtime panic).
- This is a **library, not a binary**: the only `cargo run` target is the
  `profile_stage2` example (needs `--features profile-stage2` and an image arg).
- When changing public API, keep it consistent with upstream naming where
  practical; `docs.rs/visioncortex` documents the upstream surface.

## Architecture (high level)

`color_clusters` components follow a state-machine model (`new` → `init` →
`tick() -> bool` → `result()`) to support incremental/interactive use.

Pipeline-ish flow: image container (`ColorImage`/`BinaryImage`, `src/image/`) →
clustering (`color_clusters/` for color, `clusters.rs` for binary
connected-components) → path extraction/walking + simplify/spline fitting
(`src/path/`) → shape analysis (`src/shape/`, incl. public `rasterizer`).
`polypartition/` (triangulation + hole removal) and `numeric/` (matrix,
perspective) are supporting. Primitives: `color`, `point`, `polar`, `field`,
`sampler`, `statistic`, `transform`, `bound`, `disjoint_sets`, `tile_map`.

Module re-exports are in `src/lib.rs`; most modules are flattened via
`pub use <mod>::*`, with `bound`, `clusters`, `color_clusters`, `disjoint_sets`,
`polypartition`, `tile_map` kept as named `pub mod`.

## Conventions

- Dual-licensed MIT OR Apache-2.0; third-party ports attributed in
  `Attributions.md`. Keep attribution when porting external code.
- Update `CHANGELOG.md` (Keep a Changelog + SemVer) for user-visible changes;
  mark breaking API changes explicitly.
