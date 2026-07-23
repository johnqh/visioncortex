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
    let cols = (w + block - 1) / block;
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
            let on = ((x / cell) + (y / cell)) % 2 == 0;
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
            let on = ((x + y) / period) % 2 == 0;
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
            let c = if (x + y) % 2 == 0 {
                Color::new(200, 120, 40)
            } else {
                Color::new(40, 120, 200)
            };
            im.set_pixel(x, y, &c);
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
