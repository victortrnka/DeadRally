//! The race's triangles, filled row by row as the original fills them: lit or shaded through a
//! table (`sub_43D530`), in one colour (`setTriangleValue` 0x43CD50) or shaded from a colour
//! at each corner (`sub_43D050`). All three sort the corners by height, follow the edges from
//! the top corner in f32 steps (one edge through the middle corner, the other straight to the
//! bottom) and fill each row from the left edge + 0.4 to the right edge + 0.6 rounded down,
//! rows 1 to 199 only and at most 512 pixels from the left.

use super::buffer::{Buffer, LEFT, STRIDE};

/// The rows triangles fill.
const ROWS: i32 = 200;
/// The widest a row is filled, from the view's left edge.
const ROW_LIMIT: i32 = 512;

/// MSVC's `_ftol`: towards zero, and 0x80000000 for what does not fit.
fn ftol(value: f64) -> i32 {
    if value.is_nan() || value >= 2_147_483_648.0 || value <= -2_147_483_649.0 {
        i32::MIN
    } else {
        value as i32
    }
}

/// A row's span from its edges: from the left + 0.4 to the right + 0.6, rounded down.
fn span(left: f32, right: f32) -> (i32, i32) {
    let round = |edge: f32, by: f32| ftol((f64::from(edge) + f64::from(by)).floor());
    (round(left, 0.4), round(right, 0.6))
}

/// Walks a triangle's rows from the top: `row(y, a, b)` with `a` on the edges through the
/// middle corner and `b` on the edge from the top to the bottom, each an x and a value (a
/// colour, for the shaded triangles) followed in f32 steps.
fn walk(corners: [(i32, i32, i32); 3], mut row: impl FnMut(i32, (f32, f32), (f32, f32))) {
    let x = corners.map(|(x, _, _)| x);
    let y = corners.map(|(_, y, _)| y);
    let value = corners.map(|(_, _, value)| value);
    let top = if y[0] < y[1] {
        if y[0] < y[2] { 0 } else { 2 }
    } else if y[1] >= y[2] {
        2
    } else {
        1
    };
    let bottom = if y[0] > y[1] {
        if y[0] > y[2] { 0 } else { 2 }
    } else if y[1] <= y[2] {
        2
    } else {
        1
    };
    if y[top] == y[bottom] {
        return;
    }
    let middle = 3 - top - bottom;
    let slope = |of: [i32; 3], from: usize, to: usize| {
        (f64::from(of[to] - of[from]) / f64::from(y[to] - y[from])) as f32
    };
    let step = |(x, value): (f32, f32), (dx, dvalue): (f32, f32)| {
        (
            (f64::from(x) + f64::from(dx)) as f32,
            (f64::from(value) + f64::from(dvalue)) as f32,
        )
    };
    let top_bottom = (slope(x, top, bottom), slope(value, top, bottom));
    let mut at = y[top];
    let (mut a, mut b);
    if y[middle] == y[top] {
        a = (x[middle] as f32, value[middle] as f32);
        b = (x[top] as f32, value[top] as f32);
    } else {
        let top_middle = (slope(x, top, middle), slope(value, top, middle));
        a = (x[top] as f32, value[top] as f32);
        b = a;
        while at < y[middle] {
            row(at, a, b);
            a = step(a, top_middle);
            b = step(b, top_bottom);
            at += 1;
        }
        if y[middle] == y[bottom] {
            return;
        }
    }
    let middle_bottom = (slope(x, middle, bottom), slope(value, middle, bottom));
    while at < y[bottom] {
        row(at, a, b);
        a = step(a, middle_bottom);
        b = step(b, top_bottom);
        at += 1;
    }
}

/// The buffer offset of column `x` on row `y` of the view.
fn offset(y: i32, x: i32) -> i64 {
    i64::from(y) * STRIDE as i64 + i64::from(x) + LEFT as i64
}

/// Fills a triangle's rows (the two fills that share `sub_43D530`'s shape), `plot` at each
/// pixel's buffer offset.
fn fill(buffer: &mut Buffer, points: [(i32, i32); 3], mut plot: impl FnMut(&mut Buffer, i64)) {
    walk(points.map(|(x, y)| (x, y, 0)), |y, (a, _), (b, _)| {
        if y <= 0 || y >= ROWS {
            return;
        }
        let (left, right) = if a > b { (b, a) } else { (a, b) };
        let (left, right) = span(left, right);
        for x in left.max(0)..right.min(ROW_LIMIT) {
            plot(buffer, offset(y, x));
        }
    });
}

/// `sub_43D530`: the triangle lit or shaded, every pixel turned through `table`.
pub(crate) fn light_triangle(buffer: &mut Buffer, points: [(i32, i32); 3], table: &[u8; 256]) {
    fill(buffer, points, |buffer, at| buffer.turn(at, table));
}

/// `setTriangleValue` (0x43CD50): the triangle in `colour`.
pub(crate) fn flat_triangle(buffer: &mut Buffer, points: [(i32, i32); 3], colour: u8) {
    fill(buffer, points, |buffer, at| buffer.put(at, colour));
}

/// `sub_43D050`: the triangle shaded from a colour at each corner (`x`, `y`, colour). Rows
/// whose edges meet are left out, and a row cut at the left keeps its left edge's colour
/// where the cut starts.
pub(crate) fn shaded_triangle(buffer: &mut Buffer, corners: [(i32, i32, i32); 3]) {
    walk(corners, |y, a, b| {
        if y <= 0 || y >= ROWS || a.0 == b.0 {
            return;
        }
        let ((left, left_colour), (right, right_colour)) = if a.0 > b.0 { (b, a) } else { (a, b) };
        let (left, right) = span(left, right);
        let step = ((f64::from(right_colour) - f64::from(left_colour))
            / f64::from(right.wrapping_sub(left))) as f32;
        // The original starts a cut row from its left edge's colour less the step times 0.
        let mut colour = f64::from(left_colour) - f64::from(step) * 0.0;
        for x in left.max(0)..right.min(ROW_LIMIT) {
            buffer.put(offset(y, x), ftol(colour) as u8);
            colour += f64::from(step);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit_rows(points: [(i32, i32); 3]) -> Vec<(i32, Vec<i32>)> {
        let mut buffer = Buffer::default();
        let mut table = [0; 256];
        table[0] = 1;
        light_triangle(&mut buffer, points, &table);
        (0..200)
            .map(|y| {
                let lit = (0..320).filter(|&x| buffer.pixel(x, y as usize) == 1);
                (y, lit.map(|x| x as i32).collect::<Vec<_>>())
            })
            .filter(|(_, lit)| !lit.is_empty())
            .collect()
    }

    /// A headlight's rows run from its left edge + 0.4 to its right edge + 0.6, rounded down:
    /// a cone from (10, 5) widening 0.45 a row each side is empty at its tip, and a row on its
    /// edges at 9.55 and 10.45 light 9 and 10.
    #[test]
    fn a_light_s_rows_round_its_edges_as_the_original_does() {
        let rows = lit_rows([(10, 5), (19, 25), (1, 25)]);
        assert_eq!(rows[0], (6, vec![9, 10]));
        assert_eq!(rows.last().unwrap().0, 24);
    }

    /// The original never lights the race's first row.
    #[test]
    fn the_first_row_is_never_lit() {
        let rows = lit_rows([(10, -5), (20, 5), (0, 5)]);
        assert_eq!(rows[0].0, 1);
    }

    /// A shaded row runs its colour evenly from its left edge's to its right edge's: from 100
    /// to 110 over ten pixels, a colour a pixel.
    #[test]
    fn a_shaded_row_steps_its_colour_evenly() {
        let mut buffer = Buffer::default();
        shaded_triangle(&mut buffer, [(0, 10, 100), (10, 10, 110), (0, 20, 100)]);
        let row: Vec<u8> = (0..11).map(|x| buffer.pixel(x, 10)).collect();
        assert_eq!(row, [100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 0]);
    }

    /// Where a shaded triangle's edges meet on a row (a sliver, its corners in a line) the
    /// original leaves the row out, where a flat one fills the pixel the edges round to.
    #[test]
    fn a_shaded_sliver_is_left_out() {
        let sliver = [(1, 10), (3, 14), (5, 18)];
        let mut shaded = Buffer::default();
        shaded_triangle(&mut shaded, sliver.map(|(x, y)| (x, y, 100)));
        let mut flat = Buffer::default();
        flat_triangle(&mut flat, sliver, 9);
        assert_eq!(flat.pixel(1, 11), 9);
        assert_eq!(shaded.pixel(1, 11), 0);
    }

    /// A shaded row cut at the view's left keeps its left edge's colour where the cut starts,
    /// as the original does (it steps it on by the colour step times 0).
    #[test]
    fn a_shaded_row_cut_at_the_left_starts_from_its_edge_colour() {
        let mut buffer = Buffer::default();
        shaded_triangle(&mut buffer, [(-10, 10, 100), (10, 10, 120), (-10, 20, 100)]);
        assert_eq!(buffer.pixel(0, 10), 100);
        assert_eq!(buffer.pixel(1, 10), 101);
    }
}
