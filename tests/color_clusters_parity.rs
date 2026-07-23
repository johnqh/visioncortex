//! Byte-for-byte parity snapshots for the color-clustering pipeline.
//!
//! The clustering pipeline (`Runner::run`) is fully deterministic (no RNG, no
//! threads), so its output can be pinned with golden snapshots. These snapshots
//! were generated from the pre-refactor code and must remain identical after the
//! `Box<dyn Fn>` -> generics refactor.
//!
//! Everything here goes through the crate's public API only, so the tests stay
//! valid across the refactor.
//!
//! Regenerate the goldens (only ever from known-good code) with:
//!     VISIONCORTEX_BLESS_PARITY=1 cargo test --test color_clusters_parity
//! Otherwise the test asserts each live snapshot equals its committed golden,
//! byte for byte.

use std::fmt::Write as _;
use std::path::PathBuf;

use visioncortex::color_clusters::{
    ClustersView, KeyingAction, Runner, RunnerConfig, HIERARCHICAL_MAX,
};
use visioncortex::{Color, ColorImage, PathSimplifyMode, PointI32};

// ---------------------------------------------------------------------------
// Deterministic input images (built procedurally; the crate has no image decoder)
// ---------------------------------------------------------------------------

/// A smooth-ish RGB gradient across the image.
fn img_gradient(w: usize, h: usize) -> ColorImage {
    let mut im = ColorImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            let r = (x * 255 / w.max(1)) as u8;
            let g = (y * 255 / h.max(1)) as u8;
            let b = ((x + y) * 255 / (w + h).max(1)) as u8;
            im.set_pixel(x, y, &Color::new(r, g, b));
        }
    }
    im
}

/// A grid of flat-colored blocks.
fn img_blocks(w: usize, h: usize, block: usize) -> ColorImage {
    let palette = [
        Color::new(220, 30, 30),
        Color::new(30, 220, 30),
        Color::new(30, 30, 220),
        Color::new(220, 220, 30),
        Color::new(30, 220, 220),
        Color::new(220, 30, 220),
        Color::new(240, 240, 240),
        Color::new(20, 20, 20),
    ];
    let mut im = ColorImage::new_w_h(w, h);
    let cols = w.div_ceil(block);
    for y in 0..h {
        for x in 0..w {
            let bi = (y / block) * cols + (x / block);
            im.set_pixel(x, y, &palette[bi % palette.len()]);
        }
    }
    im
}

/// A two-color checkerboard of `cell`-sized squares.
fn img_checker(w: usize, h: usize, cell: usize) -> ColorImage {
    let a = Color::new(15, 15, 15);
    let b = Color::new(235, 235, 235);
    let mut im = ColorImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            let on = ((x / cell) + (y / cell)).is_multiple_of(2);
            im.set_pixel(x, y, if on { &a } else { &b });
        }
    }
    im
}

/// Diagonal stripes (exercises the `diagonal` connectivity flag).
fn img_diagonal(w: usize, h: usize, period: usize) -> ColorImage {
    let a = Color::new(200, 40, 40);
    let b = Color::new(40, 40, 200);
    let mut im = ColorImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            let on = ((x + y) / period).is_multiple_of(2);
            im.set_pixel(x, y, if on { &a } else { &b });
        }
    }
    im
}

/// A pseudo-random color field from a fixed-seed LCG (no `rand` dependency).
fn img_random(w: usize, h: usize, seed: u64) -> ColorImage {
    let mut state = seed;
    let mut next = || {
        // LCG (Knuth MMIX constants); use high byte for each channel.
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 56) as u8
    };
    let mut im = ColorImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            // Quantize to a few levels so clusters actually form.
            let q = |v: u8| (v / 64) * 64;
            im.set_pixel(x, y, &Color::new(q(next()), q(next()), q(next())));
        }
    }
    im
}

/// An image with a solid key-color background around a colored square.
fn img_keyed(w: usize, h: usize, key: Color) -> ColorImage {
    let mut im = ColorImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            im.set_pixel(x, y, &key);
        }
    }
    let (x0, y0, x1, y1) = (w / 4, h / 4, 3 * w / 4, 3 * h / 4);
    for y in y0..y1 {
        for x in x0..x1 {
            let c = if (x + y).is_multiple_of(2) {
                Color::new(200, 120, 40)
            } else {
                Color::new(40, 120, 200)
            };
            im.set_pixel(x, y, &c);
        }
    }
    im
}

/// A single flat color (degenerate: collapses to one cluster).
fn img_solid(w: usize, h: usize, c: Color) -> ColorImage {
    let mut im = ColorImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            im.set_pixel(x, y, &c);
        }
    }
    im
}

/// Two solid halves split down the middle.
fn img_halves(w: usize, h: usize) -> ColorImage {
    let a = Color::new(200, 60, 60);
    let b = Color::new(60, 60, 200);
    let mut im = ColorImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            im.set_pixel(x, y, if x < w / 2 { &a } else { &b });
        }
    }
    im
}

/// Concentric square rings — produces nested clusters and holes (hollow path).
fn img_nested_squares(w: usize, h: usize, ring: usize) -> ColorImage {
    let a = Color::new(230, 230, 230);
    let b = Color::new(25, 25, 25);
    let mut im = ColorImage::new_w_h(w, h);
    let (cx, cy) = (w as i32 / 2, h as i32 / 2);
    for y in 0..h {
        for x in 0..w {
            let d = ((x as i32 - cx).abs()).max((y as i32 - cy).abs()) as usize;
            im.set_pixel(x, y, if (d / ring).is_multiple_of(2) { &a } else { &b });
        }
    }
    im
}

/// Concentric circular rings.
fn img_radial(w: usize, h: usize, ring: usize) -> ColorImage {
    let a = Color::new(40, 200, 120);
    let b = Color::new(200, 40, 120);
    let mut im = ColorImage::new_w_h(w, h);
    let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
    for y in 0..h {
        for x in 0..w {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let r = (dx * dx + dy * dy).sqrt() as usize;
            im.set_pixel(x, y, if (r / ring).is_multiple_of(2) { &a } else { &b });
        }
    }
    im
}

/// Vertical stripes.
fn img_stripes_v(w: usize, h: usize, period: usize) -> ColorImage {
    let a = Color::new(210, 210, 40);
    let b = Color::new(40, 40, 40);
    let mut im = ColorImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            im.set_pixel(x, y, if (x / period).is_multiple_of(2) { &a } else { &b });
        }
    }
    im
}

/// A background with thin 1px cross-hatch lines (exercises the thread-like /
/// perimeter>=area branch of `patch_good`).
fn img_thin_lines(w: usize, h: usize, spacing: usize) -> ColorImage {
    let bg = Color::new(20, 20, 20);
    let line = Color::new(230, 230, 230);
    let mut im = ColorImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            let on = x.is_multiple_of(spacing) || y.is_multiple_of(spacing);
            im.set_pixel(x, y, if on { &line } else { &bg });
        }
    }
    im
}

/// A gradient with additive pseudo-random noise.
fn img_gradient_noise(w: usize, h: usize, seed: u64) -> ColorImage {
    let mut state = seed;
    let mut jitter = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 58) as i32) - 16 // roughly [-16, 15]
    };
    let mut im = ColorImage::new_w_h(w, h);
    let clamp = |v: i32| v.clamp(0, 255) as u8;
    for y in 0..h {
        for x in 0..w {
            let base = (x * 255 / w.max(1)) as i32;
            im.set_pixel(
                x,
                y,
                &Color::new(
                    clamp(base + jitter()),
                    clamp((y * 255 / h.max(1)) as i32 + jitter()),
                    clamp(128 + jitter()),
                ),
            );
        }
    }
    im
}

// ---------------------------------------------------------------------------
// Deterministic FNV-1a hash for the large per-pixel arrays
// ---------------------------------------------------------------------------

struct Fnv(u64);
impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf29ce484222325)
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// Canonical, human-diffable snapshot of a clustering result
// ---------------------------------------------------------------------------

fn snapshot(name: &str, view: &ClustersView, with_svg: bool) -> String {
    let mut out = String::new();
    writeln!(out, "case {}", name).unwrap();
    writeln!(out, "size {}x{}", view.width, view.height).unwrap();
    writeln!(out, "output_len {}", view.clusters_output.len()).unwrap();

    // Per-pixel cluster assignment: the tightest raw check (hashed for size).
    let mut h_idx = Fnv::new();
    for ci in view.cluster_indices {
        h_idx.write(&ci.0.to_le_bytes());
    }
    writeln!(out, "cluster_indices_fnv {:016x}", h_idx.finish()).unwrap();

    // Full rendered raster (residue colors, output order).
    let raster = view.to_color_image();
    let mut h_ras = Fnv::new();
    h_ras.write(&raster.pixels);
    writeln!(out, "raster_fnv {:016x}", h_ras.finish()).unwrap();

    // Per output-cluster aggregates, in output iteration order.
    writeln!(out, "clusters:").unwrap();
    for (i, c) in view.iter().enumerate() {
        let col = c.color();
        let res = c.residue_color();
        writeln!(
            out,
            "  {}: area={} color={},{},{},{} residue={},{},{},{} rect={},{},{},{} depth={} holes={} num_holes={}",
            i,
            c.area(),
            col.r, col.g, col.b, col.a,
            res.r, res.g, res.b, res.a,
            c.rect.left, c.rect.top, c.rect.right, c.rect.bottom,
            c.depth,
            c.holes.len(),
            c.num_holes,
        )
        .unwrap();
    }

    // For one representative case, also snapshot the downstream integer path
    // output (exercises hole rendering + path walking, exact-byte deterministic).
    if with_svg {
        writeln!(out, "svg:").unwrap();
        for (i, c) in view.iter().enumerate() {
            let paths = c.to_compound_path(
                view,
                true,
                PathSimplifyMode::Polygon,
                60.0,
                4.0,
                10,
                45.0,
            );
            let (svg, _) = paths.to_svg_string(true, PointI32 { x: 0, y: 0 }, None);
            writeln!(out, "  {}: {}", i, svg).unwrap();
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Config matrix
// ---------------------------------------------------------------------------

fn base_config() -> RunnerConfig {
    RunnerConfig::default()
}

/// Build all (name, image, config, with_svg) cases exercising every closure.
#[allow(clippy::vec_init_then_push)] // one push per case keeps the matrix readable
fn cases() -> Vec<(String, ColorImage, RunnerConfig, bool)> {
    let mut v: Vec<(String, ColorImage, RunnerConfig, bool)> = Vec::new();

    // gradient with the default config (representative case: also snapshot SVG).
    v.push(("gradient_default".into(), img_gradient(48, 40), base_config(), true));

    // gradient, diagonal connectivity on.
    v.push((
        "gradient_diagonal".into(),
        img_gradient(48, 40),
        RunnerConfig { diagonal: true, ..base_config() },
        false,
    ));

    // blocks, default.
    v.push(("blocks_default".into(), img_blocks(48, 48, 8), base_config(), false));

    // blocks with small hierarchical cap (exercises deepen==false branch heavily).
    v.push((
        "blocks_hier4".into(),
        img_blocks(48, 48, 8),
        RunnerConfig { hierarchical: 4, ..base_config() },
        false,
    ));

    // blocks with mid hierarchical cap.
    v.push((
        "blocks_hier64".into(),
        img_blocks(48, 48, 8),
        RunnerConfig { hierarchical: 64, ..base_config() },
        false,
    ));

    // checkerboard, default (many tiny clusters -> deepen/hollow pressure).
    v.push(("checker_default".into(), img_checker(48, 48, 4), base_config(), false));

    // checkerboard, tuned deepen/hollow thresholds.
    v.push((
        "checker_tuned".into(),
        img_checker(48, 48, 4),
        RunnerConfig {
            deepen_diff: 16,
            hollow_neighbours: 2,
            good_min_area: 4,
            good_max_area: 64,
            ..base_config()
        },
        false,
    ));

    // diagonal stripes, diagonal on.
    v.push((
        "diagonal_on".into(),
        img_diagonal(48, 48, 3),
        RunnerConfig { diagonal: true, ..base_config() },
        false,
    ));

    // diagonal stripes, diagonal off.
    v.push((
        "diagonal_off".into(),
        img_diagonal(48, 48, 3),
        RunnerConfig { diagonal: false, ..base_config() },
        false,
    ));

    // random field, default (exercises `same`/`diff` broadly).
    v.push(("random_default".into(), img_random(48, 48, 0x1234_5678), base_config(), false));

    // random field, looser same-color threshold.
    v.push((
        "random_loose".into(),
        img_random(48, 48, 0x1234_5678),
        RunnerConfig { is_same_color_a: 5, is_same_color_b: 2, ..base_config() },
        false,
    ));

    // keyed image, Keep the key pixels.
    v.push((
        "keyed_keep".into(),
        img_keyed(48, 48, Color::new(10, 200, 10)),
        RunnerConfig {
            key_color: Color::new(10, 200, 10),
            keying_action: KeyingAction::Keep,
            ..base_config()
        },
        false,
    ));

    // keyed image, Discard the key pixels.
    v.push((
        "keyed_discard".into(),
        img_keyed(48, 48, Color::new(10, 200, 10)),
        RunnerConfig {
            key_color: Color::new(10, 200, 10),
            keying_action: KeyingAction::Discard,
            ..base_config()
        },
        false,
    ));

    // no hierarchy at all (stage-2 skipped): pure stage-1 output path.
    v.push((
        "blocks_flat".into(),
        img_blocks(48, 48, 8),
        RunnerConfig { hierarchical: 0, ..base_config() },
        false,
    ));

    // --- additional synthetic coverage (procedural inputs are free) ---

    // solid color: degenerate single-cluster path.
    v.push(("solid".into(), img_solid(40, 40, Color::new(90, 140, 190)), base_config(), false));

    // two halves.
    v.push(("halves".into(), img_halves(48, 40), base_config(), false));

    // nested squares: strong hole / hollow coverage (also snapshot SVG).
    v.push(("nested_squares".into(), img_nested_squares(48, 48, 4), base_config(), true));

    // nested squares with hollow disabled.
    v.push((
        "nested_squares_no_hollow".into(),
        img_nested_squares(48, 48, 4),
        RunnerConfig { hollow_neighbours: 0, ..base_config() },
        false,
    ));

    // radial rings, diagonal on (curved boundaries).
    v.push((
        "radial_diagonal".into(),
        img_radial(48, 48, 5),
        RunnerConfig { diagonal: true, ..base_config() },
        false,
    ));

    // vertical stripes.
    v.push(("stripes_v".into(), img_stripes_v(48, 40, 3), base_config(), false));

    // thin cross-hatch lines: thread-like clusters exercise patch_good's
    // perimeter>=area branch.
    v.push(("thin_lines".into(), img_thin_lines(48, 48, 6), base_config(), false));

    // thin lines with good_min_area=0 (patch_good's good_min_area==0 branch).
    v.push((
        "thin_lines_minarea0".into(),
        img_thin_lines(48, 48, 6),
        RunnerConfig { good_min_area: 0, ..base_config() },
        false,
    ));

    // gradient with noise.
    v.push(("gradient_noise".into(), img_gradient_noise(48, 40, 0xdead_beef), base_config(), false));

    // small batch_size forces multiple stage-1 batches (must not change output).
    v.push((
        "blocks_batch7".into(),
        img_blocks(48, 48, 8),
        RunnerConfig { batch_size: 7, ..base_config() },
        false,
    ));

    // batch_size of 1 (pathological chunking; output must be unchanged).
    v.push((
        "checker_batch1".into(),
        img_checker(32, 32, 4),
        RunnerConfig { batch_size: 1, ..base_config() },
        false,
    ));

    // very large deepen_diff => deepen almost always false.
    v.push((
        "random_high_deepen".into(),
        img_random(48, 48, 0x0bad_f00d),
        RunnerConfig { deepen_diff: 100_000, ..base_config() },
        false,
    ));

    // tight same-color threshold => many small clusters.
    v.push((
        "random_tight".into(),
        img_random(48, 48, 0x0bad_f00d),
        RunnerConfig { is_same_color_a: 2, is_same_color_b: 0, ..base_config() },
        false,
    ));

    // larger random field.
    v.push(("random_64".into(), img_random(64, 64, 0xfeed_face), base_config(), false));

    // explicit HIERARCHICAL_MAX (default, but pinned for clarity).
    let _ = HIERARCHICAL_MAX;

    v
}

// ---------------------------------------------------------------------------
// Golden file plumbing
// ---------------------------------------------------------------------------

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity")
}

fn golden_path(name: &str) -> PathBuf {
    golden_dir().join(format!("{}.snap", name))
}

#[test]
fn color_clusters_byte_for_byte_parity() {
    let bless = std::env::var_os("VISIONCORTEX_BLESS_PARITY").is_some();
    if bless {
        std::fs::create_dir_all(golden_dir()).expect("create tests/parity");
    }

    let mut failures: Vec<String> = Vec::new();

    for (name, image, config, with_svg) in cases() {
        let clusters = Runner::new(config, image).run();
        let view = clusters.view();
        let got = snapshot(&name, &view, with_svg);

        let path = golden_path(&name);
        if bless {
            std::fs::write(&path, got.as_bytes())
                .unwrap_or_else(|e| panic!("write golden {}: {}", path.display(), e));
            continue;
        }

        match std::fs::read_to_string(&path) {
            Ok(want) => {
                if want != got {
                    failures.push(format!(
                        "MISMATCH {}\n--- golden ({}) ---\n{}\n--- got ---\n{}",
                        name,
                        path.display(),
                        want,
                        got
                    ));
                }
            }
            Err(e) => failures.push(format!(
                "MISSING golden for {} at {} ({}). Run with VISIONCORTEX_BLESS_PARITY=1 to generate.",
                name,
                path.display(),
                e
            )),
        }
    }

    assert!(
        failures.is_empty(),
        "color clustering parity broken in {} case(s):\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
