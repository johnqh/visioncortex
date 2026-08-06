# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## 0.9.2 - 2026-08-06

* Fixed `BinaryImage::to_clusters` aborting with `panic!("overflow")` on masks
  needing more than 65535 provisional labels. The scanline pass takes a label
  every time a pixel starts a run with no labelled neighbour, and only reclaims
  one when a merge frees the most recently issued label, so the peak tracks the
  number of runs rather than the number of connected components — a finely
  fragmented mask can need one label per set pixel, far beyond what the 16-bit
  `MonoImageItem` held. The label map is now `u32`, a range the pixel count
  already bounds, so the counter can no longer run out. `MonoImageItem` itself
  is unchanged, only the labels internal to `to_clusters` widened, and
  clustering output is byte-for-byte unaffected.

## 0.9.1 - 2026-07-27

* Fixed cubic Bezier fitting swinging far away from the input on sparse,
  unevenly spaced slices (e.g. a 3 px jog followed by a 160 px straight leg —
  real walker output): the fit error was only measured at the sample points,
  so a lone cubic could interpolate every sample while ballooning between
  them. `fit_points_with_beziers` densifies the slice with witness points and
  returns the full multi-curve chain (welded and endpoint-pinned) instead of
  truncating to the first fragment; `Spline::from_path_f64` now keeps every
  cubic of the chain. The single-curve `fit_points_with_bezier` is kept for
  compatibility but its truncation caveat is documented.

## 0.9.0 - 2026-07-24

* Added the `polypartition` module: polygon triangulation (ear-clipping,
  monotone, optimal dynamic programming) and hole removal, ported from
  [PolyPartition](https://github.com/ivanfratric/polypartition) by Ivan Fratric
* Made `SubdivideSmooth` public and added open-path (open polyline) variants of
  `find_corners`, `find_splice_points`, and `subdivide_keep_corners`;
  `fit_points_with_bezier` now takes a configurable `max_error` (closed-path
  behaviour unchanged)
* **Breaking:** `color_clusters::Builder`, `BuilderImpl`, and
  `IncrementalBuilder` are now generic over their four closure types instead of
  storing `Box<dyn Fn>`, so the hot `same`/`diff` closures are monomorphised and
  inlined
* **Breaking:** the `deepen`/`hollow` closures now receive `&ClustersView`
  instead of `&BuilderImpl`
* **Breaking:** `color_clusters::Builder` is now a type-state builder — every
  closure must be set before `run()`/`start()`, which is now enforced at compile
  time instead of panicking at runtime

## 0.8.9 - 2025-10-17

* Fixes potential panics

## 0.8.8 - 2024-03-29

* Now uses Rust 2021 edition
* Fixed compiler warnings
* Added `BoundingRectF64`
* Refactored implementation of `Forests`
* Make public the `rasterizer` module

## 0.8.6 - 2023-11-17

* Added `Shape::is_isosceles_triangle`

## 0.8.5 - 2023-11-13

* Improve `Matrix` API

## 0.8.4 - 2023-11-12

* Introduce circular arc functions

## 0.8.3 - 2023-11-10

* Introduce `PointType` to cast between `PointI32` and `PointF64`

## 0.8.2 - 2023-10-07

* Impl `Clone` for `CompoundPath` / `Spline`

## 0.8.1 - 2023-09-17

* Fixed "The two lines are parallel!"

## 0.8.0 - 2022-10-09

* Added `KeyingAction` to `color_clusters::Builder` (#6)
* Remove `ClustersView` from `color_clusters::Builder` (#8)