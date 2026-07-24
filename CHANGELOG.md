# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

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