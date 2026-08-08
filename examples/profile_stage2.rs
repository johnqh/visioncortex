//! Where does the hierarchical-merge stage spend its time?
//!
//!   cargo run --release --features profile-stage2 --example profile_stage2 -- <image> ...
//!
//! Runs the color-cluster Runner with the default VTracer config and dumps the
//! stage-2 operation counters and coarse timers gathered by the
//! `profile-stage2` instrumentation. Requires that feature; without it the
//! counters are compiled out and everything reads zero.

use visioncortex::color_clusters::{Runner, RunnerConfig, KeyingAction, HIERARCHICAL_MAX};
use visioncortex::{Color, ColorImage};

#[cfg(feature = "profile-stage2")]
use visioncortex::color_clusters::prof;

fn load(path: &str) -> ColorImage {
    let img = image::open(path)
        .unwrap_or_else(|e| panic!("open {path}: {e}"))
        .to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    ColorImage { pixels: img.into_raw(), width: w, height: h }
}

fn default_config(img: &ColorImage) -> RunnerConfig {
    // Mirrors ColorClusterFrontend::prepare for a default vtracer Config
    // (color_precision 6 -> loss 2, filter_speckle 4 -> good_min_area 16,
    // layer_difference 16), minus transparency keying.
    RunnerConfig {
        diagonal: false,
        hierarchical: HIERARCHICAL_MAX,
        batch_size: 25600,
        good_min_area: 16,
        good_max_area: img.width * img.height,
        is_same_color_a: 2,
        is_same_color_b: 1,
        deepen_diff: 16,
        hollow_neighbours: 1,
        key_color: Color::default(),
        keying_action: KeyingAction::Discard,
    }
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: profile_stage2 <image> [<image> ...]");
        std::process::exit(2);
    }

    #[cfg(not(feature = "profile-stage2"))]
    eprintln!("WARNING: built without --features profile-stage2; counters will be zero.\n");

    for path in &paths {
        let img = load(path);
        let mp = img.width as f64 * img.height as f64 / 1e6;
        let config = default_config(&img);

        #[cfg(feature = "profile-stage2")]
        prof::reset();

        let _clusters = Runner::new(config, img).run();

        let name = std::path::Path::new(path).file_name().unwrap().to_string_lossy();
        println!("== {name}  ({mp:.2} MP) ==");

        #[cfg(feature = "profile-stage2")]
        {
            let snap: std::collections::HashMap<_, _> = prof::snapshot().into_iter().collect();
            let g = |k: &str| *snap.get(k).unwrap_or(&0);

            let stage2 = g("stage2_ns") as f64;
            let neighbour = g("neighbour_ns") as f64;
            let bookkeep = g("bookkeep_ns") as f64;
            let merge = g("merge_ns") as f64;
            // Everything not inside a timed section is the per-bucket skip-scan.
            let scan = (stage2 - neighbour - bookkeep - merge).max(0.0);
            let ms = |ns: f64| ns / 1e6;
            let pct = |ns: f64| if stage2 > 0.0 { ns / stage2 * 100.0 } else { 0.0 };

            println!("  stage 2 total      {:>9.1} ms", ms(stage2));
            println!("    skip-scan        {:>9.1} ms  ({:>4.1}%)   <- for-index-in-0..clusters.len()", ms(scan), pct(scan));
            println!("    neighbours+sort  {:>9.1} ms  ({:>4.1}%)", ms(neighbour), pct(neighbour));
            println!("    merge (combine)  {:>9.1} ms  ({:>4.1}%)", ms(merge), pct(merge));
            println!("    area bookkeeping {:>9.1} ms  ({:>4.1}%)", ms(bookkeep), pct(bookkeep));
            println!();
            let buckets = g("buckets");
            let merges = g("merges");
            let scan_iters = g("scan_iters");
            let matched = g("matched");
            println!("  buckets (distinct areas)  {:>12}", buckets);
            println!("  productive merges         {:>12}", merges);
            println!("  matched cluster-visits    {:>12}", matched);
            println!("  skip-scan iterations      {:>12}   <- buckets x peak clusters", scan_iters);
            if merges > 0 {
                println!("  scan iters / merge        {:>12.0}   <- wasted cluster visits per useful merge", scan_iters as f64 / merges as f64);
            }
            if buckets > 0 {
                println!("  avg clusters / bucket     {:>12.0}", scan_iters as f64 / buckets as f64);
            }
            println!("  neighbour pixels scanned  {:>12}", g("neighbour_pix"));
        }
        println!();
    }
}
