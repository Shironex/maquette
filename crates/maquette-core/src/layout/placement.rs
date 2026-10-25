//! Placement geometry: transforms, bounds, the side by side fallback row
//! and which models a camera sees.

use std::collections::BTreeMap;

use super::*;

/// Column major, like glTF: `m[column][row]`.
pub type Mat4 = [[f32; 4]; 4];

/// The matrix of a placement: translate, then rotate X, Y, Z, then scale,
/// as three.js composes an object with the default euler order.
pub fn matrix(p: &Placement) -> Mat4 {
    let [x, y, z] = p.rotation.map(f32::to_radians);
    let (sx, cx) = x.sin_cos();
    let (sy, cy) = y.sin_cos();
    let (sz, cz) = z.sin_cos();
    // Rx * Ry * Rz, row by row.
    let r = [
        [cy * cz, -cy * sz, sy],
        [cx * sz + sx * sy * cz, cx * cz - sx * sy * sz, -sx * cy],
        [sx * sz - cx * sy * cz, sx * cz + cx * sy * sz, cx * cy],
    ];
    let s = p.scale;
    let mut m = [[0.0; 4]; 4];
    for (col, column) in m.iter_mut().take(3).enumerate() {
        for (row, cell) in column.iter_mut().take(3).enumerate() {
            *cell = r[row][col] * s;
        }
    }
    m[3] = [p.position[0], p.position[1], p.position[2], 1.0];
    m
}

pub(super) fn transform(m: &Mat4, v: [f32; 3]) -> [f32; 3] {
    crate::inbox::math::point(m, v)
}

/// Bounds in a model's own frame, min and max.
pub type Bounds = ([f32; 3], [f32; 3]);

pub(super) fn corners((min, max): Bounds) -> [[f32; 3]; 8] {
    std::array::from_fn(|i| {
        [
            if i & 1 == 0 { min[0] } else { max[0] },
            if i & 2 == 0 { min[1] } else { max[1] },
            if i & 4 == 0 { min[2] } else { max[2] },
        ]
    })
}

/// The world bounds of `bounds` moved by `p`.
pub fn placed_bounds(bounds: Bounds, p: &Placement) -> Bounds {
    let m = matrix(p);
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for corner in corners(bounds) {
        let q = transform(&m, corner);
        for k in 0..3 {
            min[k] = min[k].min(q[k]);
            max[k] = max[k].max(q[k]);
        }
    }
    (min, max)
}

pub(super) fn overlaps(a: Bounds, b: Bounds) -> bool {
    (0..3).all(|k| a.0[k] < b.1[k] && b.0[k] < a.1[k])
}

/// Where the Build scene starter puts models without a placement, as the
/// shift from their own origin: when two of them overlap, all of them go
/// into a row along X, centred on the origin, with a quarter of the largest
/// diagonal between them. Otherwise they stay where they are. `models` is
/// in the order the starter imports them (by file name).
pub fn fallback_row(models: &[(String, Bounds)]) -> BTreeMap<String, [f32; 3]> {
    let mut out: BTreeMap<String, [f32; 3]> = models
        .iter()
        .map(|(id, _)| (id.clone(), [0.0; 3]))
        .collect();
    let crowded = models
        .iter()
        .enumerate()
        .any(|(i, a)| models[i + 1..].iter().any(|b| overlaps(a.1, b.1)));
    if models.len() < 2 || !crowded {
        return out;
    }
    let diagonal = |(min, max): Bounds| {
        (0..3)
            .map(|k| (max[k] - min[k]).powi(2))
            .sum::<f32>()
            .sqrt()
    };
    let gap = 0.25 * models.iter().map(|(_, b)| diagonal(*b)).fold(0.0, f32::max);
    let widths: Vec<f32> = models
        .iter()
        .map(|(_, (min, max))| max[0] - min[0])
        .collect();
    #[expect(clippy::cast_precision_loss, reason = "a handful of models")]
    let mut x = -(widths.iter().sum::<f32>() + gap * (models.len() - 1) as f32) / 2.0;
    for ((id, (min, _)), width) in models.iter().zip(&widths) {
        out.insert(id.clone(), [x - min[0], 0.0, 0.0]);
        x += width + gap;
    }
    out
}

pub(super) fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(super) fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(super) fn normalized(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = dot(v, v).sqrt();
    (len > 1e-6).then(|| [v[0] / len, v[1] / len, v[2] / len])
}

/// True when any part of the world box `bounds` may be in the stop's view,
/// a 16:9 frame ([`ASPECT`]). A box is only left out when all its corners
/// are behind the camera or outside one side of the view.
pub fn in_view(stop: &LayoutStop, bounds: Bounds) -> bool {
    let Some(forward) = normalized(sub(stop.target, stop.position)) else {
        return false;
    };
    let up = if forward[1].abs() > 0.999 {
        [0.0, 0.0, -1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let Some(right) = normalized(cross(forward, up)) else {
        return false;
    };
    let up = cross(right, forward);
    let tan_y = (stop.fov.to_radians() / 2.0).tan();
    let tan_x = tan_y * ASPECT;
    let near = 0.01;
    // Each side as a test that is positive inside the view.
    let sides: [&dyn Fn(f32, f32, f32) -> f32; 5] = [
        &|_, _, z| z - near,
        &|x, _, z| x + z * tan_x,
        &|x, _, z| z * tan_x - x,
        &|_, y, z| y + z * tan_y,
        &|_, y, z| z * tan_y - y,
    ];
    let local: Vec<[f32; 3]> = corners(bounds)
        .iter()
        .map(|c| {
            let rel = sub(*c, stop.position);
            [dot(rel, right), dot(rel, up), dot(rel, forward)]
        })
        .collect();
    sides
        .iter()
        .all(|side| local.iter().any(|[x, y, z]| side(*x, *y, *z) >= 0.0))
}
