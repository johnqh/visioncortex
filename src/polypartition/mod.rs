//! Polygon triangulation (ear-clipping, monotone, optimal dynamic programming)
//! and hole removal.
//!
//! Rust port of PolyPartition by Ivan Fratric
//! (<https://github.com/ivanfratric/polypartition>), via
//! <https://github.com/visioncortex/polypartition>. Licensed MIT; see the
//! `polypartition` section of `Attributions.md` for the original copyright.

mod hole;
mod enums;
mod polygon;
mod triangulation;
mod util;
mod vertex;

pub use hole::*;
pub use enums::*;
pub use polygon::*;
pub use triangulation::*;
pub use util::*;
pub use vertex::*;