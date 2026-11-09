<div align="center">

  <img src="docs/visioncortex-banner.png">
  <h1>Semantic Computer Vision</h1>

  <p>
    <strong>Core library for computer graphics & vision applications</strong>
  </p>

</div>

`visioncortex` is a Rust library providing building blocks for computer graphics
and vision: color/binary image representations, color clustering into a tree of
regions, path tracing and simplification (line/spline fitting), shape analysis,
polygon triangulation, and related geometry/numeric utilities. It is the core
library behind tools such as [VTracer](https://github.com/visioncortex/vtracer)
(raster-to-vector image tracing).

This repository is a fork of the upstream
[visioncortex/visioncortex](https://github.com/visioncortex/visioncortex) crate
with additional work (see `CHANGELOG.md`): a ported `polypartition` module,
performance improvements to the color-clustering pipeline, a type-state
`color_clusters::Builder`, and bug fixes (cubic Bezier fitting, `u32` cluster
labels). The library is still in an early stage and continually evolving.

For motivation and goals, see [www.visioncortex.org](http://www.visioncortex.org/).
API documentation for the upstream crate is at [docs.rs/visioncortex](http://docs.rs/visioncortex/).

## Install

Add the crate to your project. From crates.io (upstream release):

```toml
[dependencies]
visioncortex = "0.9"
```

To use this fork, depend on it via git:

```toml
[dependencies]
visioncortex = { git = "https://github.com/johnqh/visioncortex" }
```

## Build / Test

```sh
cargo build                 # build the library
cargo build --release       # optimized build
cargo test                  # run the test suite
cargo doc --open            # build and view API docs
```

The library itself has no decoder dependencies; the `image` crate is a
dev-dependency used only by tests and examples. The test suite includes
byte-for-byte parity snapshots for the color-clustering pipeline
(`tests/color_clusters_parity.rs`, goldens in `tests/parity/`) and
polypartition regression/fuzz tests (`tests/polypartition.rs`).

To regenerate the color-cluster golden snapshots (only ever from known-good
code):

```sh
VISIONCORTEX_BLESS_PARITY=1 cargo test --test color_clusters_parity
```

## Example

This is a library, not a binary. The one bundled example profiles the
color-clustering hierarchical-merge stage and requires the optional
`profile-stage2` feature (off by default; it compiles in operation counters and
timers):

```sh
cargo run --release --features profile-stage2 --example profile_stage2 -- <image> ...
```

Typical library usage: decode an image into a `ColorImage` (or `BinaryImage`),
run the clustering/tracing pipeline, and consume the resulting clusters or
vector paths. See `tests/color_clusters_parity.rs` for end-to-end usage of the
public `color_clusters::Runner` API, and `docs.rs` for per-type documentation.

## Crate layout

Single library crate (`src/lib.rs`). Key modules:

- `color_clusters` — hierarchical color-image clustering into a tree of regions
  (`Runner`, `Builder`, `RunnerConfig`); the main pipeline.
- `clusters` — connected-component clustering of binary images
  (`BinaryImage::to_clusters`).
- `image` — image containers (`ColorImage`, `BinaryImage`), formats, and summed
  area tables.
- `path` — path containers, line simplification, spline/cubic-Bezier fitting,
  smoothing, and the contour walker.
- `shape` — shape encoding, geometry, morphology, skeletonization, circular
  arcs, and the public `rasterizer`.
- `polypartition` — polygon triangulation (ear-clipping, monotone, optimal DP)
  and hole removal, ported from
  [PolyPartition](https://github.com/ivanfratric/polypartition).
- `numeric` — matrix and perspective-transform math.
- `color`, `color_stat`, `field`, `point`, `polar`, `sampler`, `statistic`,
  `transform`, `bound`, `disjoint_sets`, `tile_map` — supporting primitives.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at
your option. Third-party attributions are in `Attributions.md`.
