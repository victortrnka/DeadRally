//! The track's 3D scene drawn over the track: the buildings, walls and their pictures, each
//! object's points seen in perspective from above the view's middle. Once a race the original
//! lights the objects' points (`processSceFile` 0x40A360) and finds each triangle's picture
//! (`calculateSceTextureStructure` 0x40A880); each frame it places the objects
//! (`recalculatePolygonsInScreeenPosition` 0x40D6B0), sorts them by distance (`sub_4156B0`)
//! and draws them farthest first (`draw3dElements` 0x4116D0).

use deadrally_gamedata::race::{MAX_TRIANGLES, Scene, SceneObject, SceneTexture, SceneTriangle};

use super::buffer::{Buffer, LEFT, STRIDE};
use super::raster;

/// The view's half height, which the scene's perspective centres on, and its height; its
/// width and half width change as the status bar slides (0x445010, 0x445014).
const HALF_HEIGHT: i32 = 100;
const VIEW_HEIGHT: i32 = 200;
/// The kinds of triangles beyond plain colours: shaded by the corners' places (0x80), darkened
/// to black through one of three tables the original leaves all 0 (0x81 to 0x83), and shaded
/// by the corners' light (0x8A).
const SHADED: u32 = 0;
const BLACK: [u32; 3] = [1, 2, 3];
const LIT: u32 = 10;
const ZEROS: [u8; 256] = [0; 256];

/// What the original works out once a race: each object's points' light, and the picture each
/// triangle carries (none is -1), one slot before the first kept for the original's write
/// there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Setup {
    lights: Vec<Vec<i32>>,
    pictures: Vec<i32>,
}

impl Setup {
    pub(crate) fn new(scene: &Scene) -> Setup {
        Setup {
            lights: scene.objects.iter().map(lights).collect(),
            pictures: pictures(scene),
        }
    }

    fn picture(&self, object: usize, triangle: usize) -> i32 {
        self.pictures[1 + object * MAX_TRIANGLES + triangle]
    }
}

/// `calculateSceTextureStructure` (0x40A880): each triangle's picture, the one whose object and
/// triangle are its own; a second such picture lands on the triangle before, as the original
/// writes it there.
fn pictures(scene: &Scene) -> Vec<i32> {
    let mut pictures = vec![0; 1 + scene.objects.len() * MAX_TRIANGLES];
    for (object, shape) in scene.objects.iter().enumerate() {
        for triangle in 0..shape.triangles.len() {
            pictures[1 + object * MAX_TRIANGLES + triangle] = -1;
        }
    }
    for (object, shape) in scene.objects.iter().enumerate() {
        for triangle in 0..shape.triangles.len() {
            let at = 1 + object * MAX_TRIANGLES + triangle;
            for (index, picture) in scene.textures.iter().enumerate() {
                if picture.object == object as i32 && picture.triangle == triangle as i32 {
                    let slot = if pictures[at] == -1 { at } else { at - 1 };
                    pictures[slot] = index as i32;
                }
            }
        }
    }
    pictures
}

/// `processSceFile` (0x40A360): each point's light, 0 to 8, from the normals of the triangles
/// it is a corner of, averaged and lit from up and to the left; in the original's order of
/// operations, its floats kept as f32 where it keeps them.
fn lights(object: &SceneObject) -> Vec<i32> {
    const SCALE: f64 = 0.003_906_25;
    let points = object.points.len();
    let mut corners = vec![0i32; points];
    for triangle in &object.triangles {
        for &corner in &triangle.corners {
            corners[corner] += 1;
        }
    }
    let (mut nx, mut ny, mut nz) = (vec![0f32; points], vec![0f32; points], vec![0f32; points]);
    let sum = |value: f32, by: f64| (f64::from(value) + by) as f32;
    for triangle in &object.triangles {
        let [a, b, c] = triangle.corners;
        let point = |corner: usize| object.points[corner];
        let x = |corner: usize| f64::from(point(corner)[0]) * SCALE;
        let y = |corner: usize| f64::from(point(corner)[1]) * SCALE;
        let z = |corner: usize| f64::from(point(corner)[2] - 256);
        let (dz_c, dy_b, dz_b, dy_c) = (z(c) - z(a), y(b) - y(a), z(b) - z(a), y(c) - y(a));
        let n1 = dy_b * dz_c - dy_c * dz_b;
        let (dx_c, dx_b) = (x(c) - x(a), x(b) - x(a));
        let n2 = dx_c * dz_b - dx_b * dz_c;
        let n3 = dx_b * dy_c - dx_c * dy_b;
        let length = n3 * n3 + n2 * n2 + n1 * n1;
        let (p1, p2, p3) = if length > 1.0 {
            let scale = (10_000.0 / length).sqrt();
            (scale * n1, scale * n2, scale * n3)
        } else {
            (100.0, 100.0, 100.0)
        };
        for corner in [a, b, c] {
            nx[corner] = sum(nx[corner], p1);
        }
        for corner in [a, b, c] {
            ny[corner] = sum(ny[corner], p2);
        }
        for corner in [a, b, c] {
            nz[corner] = sum(nz[corner], p3);
        }
    }
    let root_350 = 350f64.sqrt();
    let root_425 = 425f64.sqrt();
    (0..points)
        .map(|v| {
            if corners[v] >= 1 {
                let share = 1.0 / f64::from(corners[v]);
                nx[v] = (share * f64::from(nx[v])) as f32;
                ny[v] = (share * f64::from(ny[v])) as f32;
                nz[v] = (share * f64::from(nz[v])) as f32;
            }
            let (x, y, z) = (f64::from(nx[v]), f64::from(ny[v]), f64::from(nz[v]));
            let mut length = z * z + y * y + x * x;
            if length < 1.0 {
                length = 1.0;
            }
            let scale = (10_000.0 / length).sqrt();
            let x1 = (x * scale) as f32;
            let y1 = (y * scale) as f32;
            let z1 = scale * z;
            nx[v] = x1;
            let flat = (z1 * z1 + f64::from(y1) * f64::from(y1)) as f32;
            if f64::from(x1) * f64::from(x1) + f64::from(flat) < 1.0 {
                nx[v] = (f64::from(x1) + 1.0) as f32;
            }
            let towards = (f64::from(y1) * -10.0 - z1 * 15.0) - f64::from(nx[v]) * 10.0;
            let length = (f64::from(nx[v]) * f64::from(nx[v]) + f64::from(flat)).sqrt();
            let cosine = towards / (length * root_350);
            if cosine > 0.0 {
                (cosine * 5.0 + 3.0) as i32
            } else {
                ((towards / (root_425 * length) + 1.0) * 3.0) as i32
            }
        })
        .collect()
}

/// `sub_4115C0`: the original's quicksort of (distance, object) by distance, which leaves
/// equal distances in its own order.
fn sort(slots: &mut [(i32, usize)], mut low: usize, high: usize) {
    loop {
        let pivot = slots[(low + high) / 2].0;
        let (mut i, mut j) = (low, high);
        loop {
            while slots[i].0 < pivot {
                i += 1;
            }
            while pivot < slots[j].0 {
                j -= 1;
            }
            if i > j {
                break;
            }
            slots.swap(i, j);
            i += 1;
            if j == 0 {
                break;
            }
            j -= 1;
            if i >= j {
                break;
            }
        }
        if low < j {
            sort(slots, low, j);
        }
        if i >= high {
            return;
        }
        low = i;
    }
}

/// `sub_411530`: a point `at` (256ths, relative to the view's middle) at `depth` on the screen.
fn project(at: i32, depth: i32, middle: i32) -> i32 {
    let scaled = (at << 8).checked_div(depth).unwrap_or(0);
    middle + (scaled.wrapping_add(0x80) >> 8)
}

/// `draw3dTexture` (0x43B2F0): a picture's rows at (`x`, `y`) of the view, its 0 bytes left
/// out; rows above the buffer skipped, and the picture stopped past its end.
fn draw_picture(buffer: &mut Buffer, pixels: &[u8], (x, y): (i32, i32), picture: &SceneTexture) {
    let mut at = i64::from(y) * STRIDE as i64 + i64::from(x) + LEFT as i64;
    let mut from = i64::from(picture.offset);
    let width = i64::from(picture.width);
    for _ in 0..picture.height {
        if at > (VIEW_HEIGHT as i64) * STRIDE as i64 {
            return;
        }
        if at < 0 {
            from += width;
            at += STRIDE as i64;
            continue;
        }
        for column in 0..width.max(0) {
            let byte = usize::try_from(from + column)
                .ok()
                .and_then(|index| pixels.get(index))
                .copied()
                .unwrap_or(0);
            if byte != 0 {
                buffer.put(at + column, byte);
            }
        }
        from += width.max(0);
        at += width.max(0) + STRIDE as i64 - width;
    }
}

/// The scene over the frame, the view's top left at `camera` on the track, the view `left`
/// pixels from the screen's left and `width` wide; `cull` leaves out objects far off the view
/// (every track but the first), and the triangles' pictures are drawn only with `pictures`
/// (F4, 0x44502C).
pub(crate) fn draw(
    buffer: &mut Buffer,
    scene: &Scene,
    setup: &Setup,
    (camera_x, camera_y): (i32, i32),
    cull: bool,
    (left, width): (i32, i32),
    pictures: bool,
) {
    let half = width >> 1;
    let objects = &scene.objects;
    let places: Vec<(i32, i32)> = objects
        .iter()
        .map(|object| {
            (
                object.position.0 - half - camera_x,
                object.position.1 - HALF_HEIGHT - camera_y,
            )
        })
        .collect();
    let far = |object: &SceneObject, (x, y): (i32, i32)| {
        let [least_x, most_x, least_y, most_y] = object.bounds;
        (x << 8) > most_x.wrapping_add(half << 8)
            || (x << 8) < least_x.wrapping_sub(half << 8)
            || (y << 8) > most_y.wrapping_add(HALF_HEIGHT << 8)
            || (y << 8) < least_y.wrapping_sub(HALF_HEIGHT << 8)
    };
    let mut slots: Vec<(i32, usize)> = places
        .iter()
        .enumerate()
        .map(|(index, &(x, y))| (y.wrapping_mul(y).wrapping_add(x.wrapping_mul(x)), index))
        .collect();
    if !slots.is_empty() {
        let last = slots.len() - 1;
        sort(&mut slots, 0, last);
    }
    for &(_, index) in slots.iter().rev() {
        let object = &objects[index];
        let (x, y) = places[index];
        if cull && far(object, (x, y)) {
            continue;
        }
        let on_screen: Vec<(i32, i32)> = object
            .points
            .iter()
            .map(|&[px, py, depth]| {
                (
                    project(px.wrapping_add(x << 8), depth, half),
                    project(py.wrapping_add(y << 8), depth, HALF_HEIGHT),
                )
            })
            .collect();
        for (number, triangle) in object.triangles.iter().enumerate() {
            let [a, b, c] = triangle.corners.map(|corner| on_screen[corner]);
            let facing = (c.1 - a.1)
                .wrapping_mul(b.0 - a.0)
                .wrapping_sub((b.1 - a.1).wrapping_mul(c.0 - a.0));
            let across = |v: i32| (v - half).wrapping_abs() < half;
            let down = |v: i32| (v - HALF_HEIGHT).wrapping_abs() < HALF_HEIGHT;
            if facing <= 0
                && (across(a.0) || across(b.0) || across(c.0))
                && (down(a.1) || down(b.1) || down(c.1))
            {
                let at = [a, b, c].map(|(x, y)| (x + left, y));
                draw_triangle(buffer, object, setup, (index, triangle), at);
            }
            let picture = setup.picture(index, number);
            if pictures && picture != -1 {
                let picture = &scene.textures[picture as usize];
                let place = |at: i32, camera: i32, half: i32, middle: i32| {
                    let at = at.wrapping_sub(camera << 8).wrapping_sub(half << 8);
                    project(at, picture.depth, middle)
                };
                let px = place(picture.position.0, camera_x, half, half);
                let py = place(picture.position.1, camera_y, HALF_HEIGHT, HALF_HEIGHT);
                if px > -picture.width && py > -picture.height && px < width && py < VIEW_HEIGHT {
                    draw_picture(buffer, &scene.pixels, (left + px, py), picture);
                }
            }
        }
    }
}

/// Triangle `triangle` of object `index` at `at` on the screen, by its kind: shaded by its
/// corners' places (0x80) or light (0x8A), black (0x81 to 0x83), or in its colour.
fn draw_triangle(
    buffer: &mut Buffer,
    object: &SceneObject,
    setup: &Setup,
    (index, triangle): (usize, &SceneTriangle),
    at: [(i32, i32); 3],
) {
    let corners = triangle.corners;
    let shaded = |colour: [i32; 3]| [0, 1, 2].map(|k| (at[k].0, at[k].1, colour[k]));
    match triangle.colour.wrapping_sub(0x80) as u32 {
        SHADED => {
            // 0x4118DF: from the point's place, every shift arithmetic and taken mod 32.
            let shade = |corner: usize| {
                let [x, y, _] = object.points[corner];
                let shift = 54i32
                    .wrapping_mul((100i32.wrapping_sub(x)) >> 8)
                    .wrapping_add(8)
                    & 31;
                ((y.wrapping_mul(75) >> shift).wrapping_abs() & 7) + 109
            };
            raster::shaded_triangle(buffer, shaded(corners.map(shade)));
        }
        kind if BLACK.contains(&kind) => raster::light_triangle(buffer, at, &ZEROS),
        LIT => {
            let light = |corner: usize| setup.lights[index][corner] + 108;
            raster::shaded_triangle(buffer, shaded(corners.map(light)));
        }
        _ => raster::flat_triangle(buffer, at, triangle.colour as u8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deadrally_gamedata::race::SceneTriangle;

    /// Objects are drawn farthest first, and objects at the same distance overlap in the order
    /// the original's quicksort leaves them, not in the order they are listed.
    #[test]
    fn objects_at_equal_distance_overlap_in_the_original_s_order() {
        let mut slots: Vec<(i32, usize)> = [5, 3, 5, 3, 5].into_iter().zip(0..).collect();
        sort(&mut slots, 0, 4);
        let drawn: Vec<usize> = slots.iter().rev().map(|&(_, index)| index).collect();
        assert_eq!(drawn, [2, 0, 4, 3, 1]);
    }

    fn object(triangles: usize) -> SceneObject {
        SceneObject {
            points: vec![[0, 0, 256]; 3],
            triangles: vec![
                SceneTriangle {
                    corners: [0, 1, 2],
                    colour: 90
                };
                triangles
            ],
            bounds: [0; 4],
            position: (0, 0),
        }
    }

    fn picture(object: i32, triangle: i32) -> SceneTexture {
        SceneTexture {
            width: 1,
            height: 1,
            offset: 0,
            position: (0, 0),
            depth: 256,
            object,
            triangle,
        }
    }

    /// A triangle shows the picture made for it; a second picture made for the same triangle
    /// shows on the triangle before instead, where the original writes it.
    #[test]
    fn a_second_picture_for_a_triangle_lands_on_the_one_before() {
        let scene = Scene {
            objects: vec![object(3)],
            textures: vec![picture(0, 2), picture(0, 2)],
            pixels: vec![],
        };
        let setup = Setup::new(&scene);
        assert_eq!(setup.picture(0, 2), 0);
        assert_eq!(setup.picture(0, 1), 1);
        assert_eq!(setup.picture(0, 0), -1);
    }

    /// A point on the track (depth 256) lands where its place says, rounded to the nearest
    /// pixel; nearer the eye (less depth) it moves out from the view's middle.
    #[test]
    fn points_project_from_the_view_s_middle() {
        assert_eq!(project(10 << 8, 256, 128), 138);
        assert_eq!(project((10 << 8) + 128, 256, 128), 139);
        assert_eq!(project(10 << 8, 128, 128), 148);
        assert_eq!(project(-(10 << 8), 128, 100), 80);
    }

    /// A triangle lying flat on the track faces away from the light: its corners get light 0,
    /// the darkest of the lit kind's colours.
    #[test]
    fn a_triangle_flat_on_the_track_is_darkest() {
        let flat = SceneObject {
            points: vec![[0, 0, 256], [2560, 0, 256], [0, 2560, 256]],
            triangles: vec![SceneTriangle {
                corners: [0, 1, 2],
                colour: 0x8A,
            }],
            bounds: [0; 4],
            position: (0, 0),
        };
        assert_eq!(lights(&flat), [0, 0, 0]);
    }

    /// A wall facing left, towards the light, is lit 5 of 8: the light comes from the left
    /// (x -10), above (y -10 against the track's down) and the eye's side (z 15).
    #[test]
    fn a_wall_facing_the_light_is_lit() {
        let wall = SceneObject {
            points: vec![[0, 0, 256], [0, 0, 266], [0, 2560, 256]],
            triangles: vec![SceneTriangle {
                corners: [0, 1, 2],
                colour: 0x8A,
            }],
            bounds: [0; 4],
            position: (0, 0),
        };
        assert_eq!(lights(&wall), [5, 5, 5]);
    }
}
