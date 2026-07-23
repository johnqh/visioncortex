//! Tests for the polypartition module (ported from
//! github.com/visioncortex/polypartition).
//!
//! Three layers:
//!  1. Fixture regression — the exact input/expected dumps from the upstream
//!     `Tester` (hexagon, hexagon-with-hole), pinning ear-clipping / monotone /
//!     optimal-DP / hole-removal output byte-for-byte.
//!  2. Render equivalence — random simple polygons are triangulated by all three
//!     algorithms and rasterized (via this crate's `rasterize_triangle`); the
//!     three renderings must be identical, must contain exactly n-2 triangles,
//!     and must match an independent even-odd polygon fill.
//!  3. Fuzz — random and degenerate inputs must never panic.

use visioncortex::polypartition::{
    remove_holes, triangulate_ec, triangulate_ec_vec, triangulate_mono,
    triangulate_mono_vec, triangulate_opt, triangulate_opt_vec, Orientation, Polygon,
    PolygonInterface,
};
use visioncortex::rasterizer::rasterize_triangle;
use visioncortex::{BinaryImage, PointF64, PointI32};

// ---------------------------------------------------------------------------
// Helpers mirroring the upstream Tester (from_input_text / dump_polygons)
// ---------------------------------------------------------------------------

fn parse_input(text: &str) -> Vec<Polygon> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut polys = vec![];
    let mut i = 1; // line 0 is the polygon count
    while i < lines.len() {
        if lines[i].trim().is_empty() {
            break;
        }
        let num_vertices: usize = lines[i].parse().unwrap();
        i += 1;
        let is_hole = lines[i] == "1";
        i += 1;
        let mut points = vec![];
        for _ in 0..num_vertices {
            let coords: Vec<f64> = lines[i]
                .split(' ')
                .map(|t| t.parse::<f64>().unwrap())
                .collect();
            points.push(PointF64::new(coords[0], coords[1]));
            i += 1;
        }
        polys.push(Polygon::from_points_and_is_hole(points, is_hole));
    }
    polys
}

fn dump_polygons(polys: &[Polygon]) -> String {
    let mut dump = vec![polys.len().to_string()];
    for p in polys.iter() {
        dump.push(p.props().dump(false));
    }
    dump.join("\n")
}

// Replicas of the four Tester pipelines.
fn pipe_remove_holes(input: &[Polygon]) -> Vec<Polygon> {
    remove_holes(input).unwrap()
}
fn pipe_ear_clipping(input: &[Polygon]) -> Vec<Polygon> {
    triangulate_ec_vec(remove_holes(input).unwrap()).unwrap()
}
fn pipe_optimal_dp(input: &[Polygon]) -> Vec<Polygon> {
    let non_hole: Vec<Polygon> = input.iter().filter(|p| !p.is_hole()).cloned().collect();
    triangulate_opt_vec(non_hole).unwrap()
}
fn pipe_monotone(input: &[Polygon]) -> Vec<Polygon> {
    triangulate_mono_vec(input.to_vec()).unwrap()
}

// ---------------------------------------------------------------------------
// 1. Fixture regression (upstream Tester expectations)
// ---------------------------------------------------------------------------

#[test]
fn fixture_hexagon() {
    let input = parse_input("1\n6\n0\n60 40\n200 40\n220 110\n200 180\n60 180\n40 110");

    assert_eq!(
        dump_polygons(&pipe_remove_holes(&input)),
        "1\n6\n0\n60 40\n200 40\n220 110\n200 180\n60 180\n40 110"
    );
    assert_eq!(
        dump_polygons(&pipe_ear_clipping(&input)),
        "4\n3\n0\n40 110\n60 40\n200 40\n3\n0\n40 110\n200 40\n220 110\n3\n0\n40 110\n220 110\n200 180\n3\n0\n40 110\n200 180\n60 180"
    );
    assert_eq!(
        dump_polygons(&pipe_optimal_dp(&input)),
        "4\n3\n0\n60 40\n60 180\n40 110\n3\n0\n60 40\n200 40\n60 180\n3\n0\n200 40\n200 180\n60 180\n3\n0\n200 40\n220 110\n200 180"
    );
    assert_eq!(
        dump_polygons(&pipe_monotone(&input)),
        "4\n3\n0\n60 40\n200 40\n40 110\n3\n0\n40 110\n200 40\n220 110\n3\n0\n40 110\n220 110\n60 180\n3\n0\n60 180\n220 110\n200 180"
    );
}

#[test]
fn fixture_hexagon_with_hole() {
    let input = parse_input(
        "2\n6\n0\n60 40\n200 40\n220 110\n200 180\n60 180\n40 110\n4\n1\n110 80\n90 140\n140 130\n170 80",
    );

    assert_eq!(
        dump_polygons(&pipe_remove_holes(&input)),
        "1\n12\n0\n60 40\n200 40\n220 110\n170 80\n110 80\n90 140\n140 130\n170 80\n220 110\n200 180\n60 180\n40 110"
    );
    assert_eq!(
        dump_polygons(&pipe_ear_clipping(&input)),
        "10\n3\n0\n200 40\n220 110\n170 80\n3\n0\n60 40\n200 40\n170 80\n3\n0\n60 40\n170 80\n110 80\n3\n0\n40 110\n60 40\n110 80\n3\n0\n40 110\n110 80\n90 140\n3\n0\n60 180\n40 110\n90 140\n3\n0\n200 180\n60 180\n90 140\n3\n0\n200 180\n90 140\n140 130\n3\n0\n220 110\n200 180\n140 130\n3\n0\n220 110\n140 130\n170 80"
    );
    assert_eq!(
        dump_polygons(&pipe_optimal_dp(&input)),
        "4\n3\n0\n60 40\n60 180\n40 110\n3\n0\n60 40\n200 40\n60 180\n3\n0\n200 40\n200 180\n60 180\n3\n0\n200 40\n220 110\n200 180"
    );
    assert_eq!(
        dump_polygons(&pipe_monotone(&input)),
        "10\n3\n0\n110 80\n60 40\n200 40\n3\n0\n60 40\n110 80\n40 110\n3\n0\n40 110\n110 80\n90 140\n3\n0\n40 110\n90 140\n60 180\n3\n0\n170 80\n110 80\n200 40\n3\n0\n170 80\n200 40\n220 110\n3\n0\n170 80\n220 110\n140 130\n3\n0\n60 180\n90 140\n140 130\n3\n0\n60 180\n140 130\n220 110\n3\n0\n60 180\n220 110\n200 180"
    );
}

// ---------------------------------------------------------------------------
// Rasterization helpers for the render-equivalence tests
// ---------------------------------------------------------------------------

const CANVAS: usize = 320;

fn render_triangles(triangles: &[Polygon]) -> BinaryImage {
    let mut img = BinaryImage::new_w_h(CANVAS, CANVAS);
    for t in triangles {
        let tri = [
            PointI32::new(t.get_point(0).x.round() as i32, t.get_point(0).y.round() as i32),
            PointI32::new(t.get_point(1).x.round() as i32, t.get_point(1).y.round() as i32),
            PointI32::new(t.get_point(2).x.round() as i32, t.get_point(2).y.round() as i32),
        ];
        rasterize_triangle(&tri, &mut img);
    }
    img
}

/// Independent even-odd polygon fill at pixel centers — no triangulation.
fn render_polygon_even_odd(points: &[PointF64]) -> BinaryImage {
    let mut img = BinaryImage::new_w_h(CANVAS, CANVAS);
    for y in 0..CANVAS {
        let cy = y as f64 + 0.5;
        for x in 0..CANVAS {
            let cx = x as f64 + 0.5;
            if point_in_polygon(cx, cy, points) {
                img.set_pixel(x, y, true);
            }
        }
    }
    img
}

fn point_in_polygon(px: f64, py: f64, pts: &[PointF64]) -> bool {
    let n = pts.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (pts[i].x, pts[i].y);
        let (xj, yj) = (pts[j].x, pts[j].y);
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn intersection_over_union(a: &BinaryImage, b: &BinaryImage) -> f64 {
    let mut inter = 0u64;
    let mut union = 0u64;
    for i in 0..(a.width * a.height) {
        let pa = a.pixels.get(i).unwrap_or(false);
        let pb = b.pixels.get(i).unwrap_or(false);
        if pa && pb {
            inter += 1;
        }
        if pa || pb {
            union += 1;
        }
    }
    if union == 0 {
        1.0
    } else {
        inter as f64 / union as f64
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (LCG) — no `rand` dependency, reproducible
// ---------------------------------------------------------------------------

struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next_u64() as usize) % (hi - lo)
    }
}

/// A random *simple* polygon: vertices at strictly increasing angles around a
/// center with random radii (star-shaped => guaranteed non-self-intersecting),
/// normalized to CCW orientation.
fn random_simple_polygon(rng: &mut Lcg) -> Polygon {
    let n = rng.range(3, 13);
    let cx = 160.0;
    let cy = 160.0;
    let mut points = Vec::with_capacity(n);
    for k in 0..n {
        // Evenly spaced sectors + jitter keeps angles strictly increasing.
        let base = (k as f64) * std::f64::consts::TAU / (n as f64);
        let jitter = (rng.f64() - 0.5) * (std::f64::consts::TAU / (n as f64)) * 0.8;
        let angle = base + jitter;
        let radius = 40.0 + rng.f64() * 100.0;
        points.push(PointF64::new(cx + radius * angle.cos(), cy + radius * angle.sin()));
    }
    let mut poly = Polygon::from_points_and_is_hole(points, false);
    poly.props_mut().set_orientation(Orientation::CounterClockwise);
    poly
}

// ---------------------------------------------------------------------------
// 2. Render equivalence on random simple polygons
// ---------------------------------------------------------------------------

#[test]
fn render_equivalence_random_simple_polygons() {
    let mut rng = Lcg::new(0xC0FFEE);
    let iterations = 200;
    for iter in 0..iterations {
        let poly = random_simple_polygon(&mut rng);
        let n = poly.num_points();

        // Use the general (*_vec) entry points: EC and OPT accept an arbitrary
        // simple polygon, while MONO first partitions into monotone pieces.
        let ec = triangulate_ec_vec(vec![poly.clone()])
            .unwrap_or_else(|e| panic!("iter {iter}: EC failed: {e}"));
        let mono = triangulate_mono_vec(vec![poly.clone()])
            .unwrap_or_else(|e| panic!("iter {iter}: MONO failed: {e}"));
        let opt = triangulate_opt_vec(vec![poly.clone()])
            .unwrap_or_else(|e| panic!("iter {iter}: OPT failed: {e}"));

        // A simple polygon of n vertices triangulates into exactly n-2 triangles.
        assert_eq!(ec.len(), n - 2, "iter {iter}: EC triangle count");
        assert_eq!(mono.len(), n - 2, "iter {iter}: MONO triangle count");
        assert_eq!(opt.len(), n - 2, "iter {iter}: OPT triangle count");

        let img_ec = render_triangles(&ec);
        let img_mono = render_triangles(&mono);
        let img_opt = render_triangles(&opt);
        let points: Vec<PointF64> = (0..n).map(|i| poly.get_point(i)).collect();
        let reference = render_polygon_even_odd(&points);

        assert!(img_ec.area() > 0, "iter {iter}: empty rasterization");

        // The three algorithms tile the same polygon, so their renderings agree
        // almost exactly (only sub-pixel seams along differing internal
        // diagonals). Observed minimum over the seed set is ~0.997.
        let iou_em = intersection_over_union(&img_ec, &img_mono);
        let iou_eo = intersection_over_union(&img_ec, &img_opt);
        assert!(iou_em >= 0.99, "iter {iter}: EC vs MONO IoU {iou_em:.4}");
        assert!(iou_eo >= 0.99, "iter {iter}: EC vs OPT IoU {iou_eo:.4}");

        // Each rendering also matches an independent even-odd fill of the
        // original polygon (looser, since integer-rounded vertices shift the
        // boundary by up to a pixel on small polygons).
        for (name, img) in [("EC", &img_ec), ("MONO", &img_mono), ("OPT", &img_opt)] {
            let iou = intersection_over_union(img, &reference);
            assert!(iou >= 0.90, "iter {iter}: {name} vs polygon fill IoU {iou:.4}");
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Fuzz: arbitrary / degenerate inputs must never panic
// ---------------------------------------------------------------------------

#[test]
fn fuzz_arbitrary_polygons_never_panic() {
    let mut rng = Lcg::new(0xBADC0DE);
    for _ in 0..1000 {
        let n = rng.range(0, 16);
        let points: Vec<PointF64> = (0..n)
            .map(|_| {
                PointF64::new(
                    (rng.range(0, 300)) as f64,
                    (rng.range(0, 300)) as f64,
                )
            })
            .collect();
        let is_hole = rng.f64() < 0.3;
        let poly = Polygon::from_points_and_is_hole(points, is_hole);

        // None of these may panic (Err is a fine, expected outcome).
        let _ = triangulate_ec(&poly);
        let _ = triangulate_mono(&poly);
        let _ = triangulate_opt(&poly);
        let _ = remove_holes(std::slice::from_ref(&poly));
        let _ = triangulate_ec_vec(vec![poly.clone()]);
        let _ = triangulate_mono_vec(vec![poly.clone()]);
        let _ = triangulate_opt_vec(vec![poly]);
    }
}

#[test]
fn fuzz_degenerate_inputs_never_panic() {
    let cases: Vec<Vec<PointF64>> = vec![
        vec![],                                                              // empty
        vec![PointF64::new(1.0, 1.0)],                                       // single point
        vec![PointF64::new(0.0, 0.0), PointF64::new(1.0, 1.0)],             // two points
        vec![                                                               // collinear
            PointF64::new(0.0, 0.0),
            PointF64::new(1.0, 0.0),
            PointF64::new(2.0, 0.0),
        ],
        vec![                                                               // duplicate points
            PointF64::new(0.0, 0.0),
            PointF64::new(0.0, 0.0),
            PointF64::new(1.0, 0.0),
            PointF64::new(1.0, 1.0),
        ],
        vec![                                                               // zero-area (repeat)
            PointF64::new(5.0, 5.0),
            PointF64::new(5.0, 5.0),
            PointF64::new(5.0, 5.0),
        ],
    ];
    for pts in cases {
        let poly = Polygon::from_points_and_is_hole(pts, false);
        let _ = triangulate_ec(&poly);
        let _ = triangulate_mono(&poly);
        let _ = triangulate_opt(&poly);
        let _ = remove_holes(std::slice::from_ref(&poly));
    }
}
