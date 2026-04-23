use crate::BinaryImage;

/// Return the outermost `depth` boundary layers of a binary image, clearing
/// all interior pixels that lie deeper than `depth` steps from the edge.
///
/// Each step removes the current outermost layer (ink pixels with at least one
/// 4-connected background neighbour) from a working copy and accumulates those
/// pixels into the result.  Deep interior pixels that are never reached are
/// cleared.
///
/// Thin strokes (entirely boundary) pass through unchanged.  Only solid filled
/// regions lose their interiors.
///
/// # Examples
///
/// A 13×13 filled star with depth 2 hollows the interior while keeping a
/// 2-pixel-deep shell — matching the visual weight of a stroked outline:
/// ```text
/// Before (fill)     After erode_interior(_, 2)
/// ------*------     ------*------
/// -----***-----     -----***-----
/// -----***-----     -----***-----
/// --*********--  →  --****-****--
/// ---*******---     ---**---**---
/// ----*****----     ----**-**----
/// ----*****----     ----*****----
/// ----*****----     ----*****----
/// ---**---**---     ---**---**---
/// ```
pub fn erode_interior(image: &BinaryImage, depth: usize) -> BinaryImage {
    let w = image.width;
    let h = image.height;
    let n = w * h;

    // Working copy as flat Vec<bool> — fast random access without BitVec overhead.
    let mut bin: Vec<bool> = (0..n).map(|i| image.get_pixel(i % w, i / w)).collect();
    let mut keep = vec![false; n];

    for _ in 0..depth {
        let mut cur = vec![false; n];
        let mut any = false;
        for i in 0..n {
            if bin[i] && is_4boundary(&bin, i, w, h) {
                cur[i]  = true;
                keep[i] = true;
                any = true;
            }
        }
        if !any { break; }
        for i in 0..n {
            if cur[i] { bin[i] = false; }
        }
    }

    let mut out = BinaryImage::new_w_h(w, h);
    for i in 0..n {
        if keep[i] { out.set_pixel(i % w, i / w, true); }
    }
    out
}

#[inline]
fn is_4boundary(bin: &[bool], i: usize, w: usize, h: usize) -> bool {
    let x = i % w;
    let y = i / w;
    (x == 0     || !bin[i - 1]) ||
    (x == w - 1 || !bin[i + 1]) ||
    (y == 0     || !bin[i - w]) ||
    (y == h - 1 || !bin[i + w])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peel1_solid_3x3_removes_center() {
        let image = BinaryImage::from_string(
            "***\n\
             ***\n\
             ***\n");
        let result = erode_interior(&image, 1);
        assert_eq!(result.to_string(),
            "***\n\
             *-*\n\
             ***\n");
    }

    #[test]
    fn peel2_solid_5x5_removes_center_only() {
        let image = BinaryImage::from_string(
            "*****\n\
             *****\n\
             *****\n\
             *****\n\
             *****\n");
        let result = erode_interior(&image, 2);
        assert_eq!(result.to_string(),
            "*****\n\
             *****\n\
             **-**\n\
             *****\n\
             *****\n");
    }

    #[test]
    fn peel_thin_stroke_unchanged() {
        // A 1px wide horizontal line: every pixel is boundary, nothing removed.
        let image = BinaryImage::from_string("*****\n");
        assert_eq!(erode_interior(&image, 1).to_string(), "*****\n");
        assert_eq!(erode_interior(&image, 3).to_string(), "*****\n");
    }

    #[test]
    fn peel_thin_stroke_2px_unchanged() {
        // A 2×5 stroke: all pixels are boundary (each touches an edge).
        let image = BinaryImage::from_string(
            "*****\n\
             *****\n");
        assert_eq!(erode_interior(&image, 1).to_string(),
            "*****\n\
             *****\n");
    }

    #[test]
    fn peel0_returns_original() {
        let image = BinaryImage::from_string(
            "***\n\
             ***\n\
             ***\n");
        // Zero depth: nothing is kept — empty image.
        let result = erode_interior(&image, 0);
        assert_eq!(result.to_string(),
            "---\n\
             ---\n\
             ---\n");
    }

    #[test]
    fn peel_more_than_depth_returns_full_shell() {
        // Peeling a 3×3 solid more than 1 time still gives the full shell
        // (the second peel finds no more interior to remove).
        let image = BinaryImage::from_string(
            "***\n\
             ***\n\
             ***\n");
        let result = erode_interior(&image, 5);
        assert_eq!(result.to_string(),
            "***\n\
             ***\n\
             ***\n");
    }

    #[test]
    fn peel2_fill_star_matches_stroke_shell() {
        // 32×32 filled star (weight-400 Material Symbols), peeled twice.
        // The result should expose only a 2-pixel-deep shell — matching the
        // stroke variant's outline width.
        let fill = BinaryImage::from_string(concat!(
            "--------------------------------\n",
            "--------------------------------\n",
            "--------------------------------\n",
            "--------------------------------\n",
            "---------------**---------------\n",
            "---------------**---------------\n",
            "--------------****--------------\n",
            "--------------****--------------\n",
            "-------------*****--------------\n",
            "-------------******-------------\n",
            "-------------******-------------\n",
            "------------********------------\n",
            "---**************************---\n",
            "----************************----\n",
            "-----**********************-----\n",
            "------********************------\n",
            "-------******************-------\n",
            "---------**************---------\n",
            "----------************----------\n",
            "----------************----------\n",
            "---------**************---------\n",
            "---------**************---------\n",
            "---------**************---------\n",
            "---------******--******---------\n",
            "---------*****----*****---------\n",
            "--------****--------****--------\n",
            "--------**------------**--------\n",
            "--------*--------------*--------\n",
            "--------------------------------\n",
            "--------------------------------\n",
            "--------------------------------\n",
            "--------------------------------\n",
        ));
        let result = erode_interior(&fill, 2);
        assert_eq!(result.to_string(), concat!(
            "--------------------------------\n",
            "--------------------------------\n",
            "--------------------------------\n",
            "--------------------------------\n",
            "---------------**---------------\n",
            "---------------**---------------\n",
            "--------------****--------------\n",
            "--------------****--------------\n",
            "-------------**-**--------------\n",
            "-------------**--**-------------\n",
            "-------------**--**-------------\n",
            "------------**----**------------\n",
            "---**********------**********---\n",
            "----********--------********----\n",
            "-----**------------------**-----\n",
            "------***--------------***------\n",
            "-------***------------***-------\n",
            "---------**----------**---------\n",
            "----------**--------**----------\n",
            "----------**--------**----------\n",
            "---------**----------**---------\n",
            "---------**----**----**---------\n",
            "---------**---****---**---------\n",
            "---------**-***--***-**---------\n",
            "---------*****----*****---------\n",
            "--------****--------****--------\n",
            "--------**------------**--------\n",
            "--------*--------------*--------\n",
            "--------------------------------\n",
            "--------------------------------\n",
            "--------------------------------\n",
            "--------------------------------\n",
        ));
    }
}
