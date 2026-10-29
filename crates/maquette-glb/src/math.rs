//! The little matrix math the analysis needs. Matrices are column major,
//! as glTF stores them: `m[column][row]`.

pub type Mat4 = [[f32; 4]; 4];
pub type Vec3 = [f32; 3];

pub const IDENTITY: Mat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [[0.0; 4]; 4];
    for (c, column) in out.iter_mut().enumerate() {
        for (r, cell) in column.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    out
}

pub fn point(m: &Mat4, p: Vec3) -> Vec3 {
    [
        m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0],
        m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1],
        m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2],
    ]
}

pub fn translation(m: &Mat4) -> Vec3 {
    [m[3][0], m[3][1], m[3][2]]
}

/// Inverts an affine matrix (the bottom row is taken as 0 0 0 1).
pub fn invert_affine(m: &Mat4) -> Option<Mat4> {
    let (a, b, c) = (m[0][0], m[1][0], m[2][0]);
    let (d, e, f) = (m[0][1], m[1][1], m[2][1]);
    let (g, h, i) = (m[0][2], m[1][2], m[2][2]);
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    // Rows of the inverted 3x3.
    let r = [
        [
            (e * i - f * h) * inv,
            (c * h - b * i) * inv,
            (b * f - c * e) * inv,
        ],
        [
            (f * g - d * i) * inv,
            (a * i - c * g) * inv,
            (c * d - a * f) * inv,
        ],
        [
            (d * h - e * g) * inv,
            (b * g - a * h) * inv,
            (a * e - b * d) * inv,
        ],
    ];
    let t = translation(m);
    let mut out = IDENTITY;
    for row in 0..3 {
        for col in 0..3 {
            out[col][row] = r[row][col];
        }
        out[3][row] = -(r[row][0] * t[0] + r[row][1] * t[1] + r[row][2] * t[2]);
    }
    Some(out)
}

pub fn distance(a: Vec3, b: Vec3) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The distance from `p` to the segment from `a` to `b`.
pub fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if len > 0.0 {
        (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1] + (p[2] - a[2]) * ab[2]) / len)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    distance(p, [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t])
}

/// An axis aligned box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb {
        min: [f32::INFINITY; 3],
        max: [f32::NEG_INFINITY; 3],
    };

    pub fn add(&mut self, p: Vec3) {
        for (k, v) in p.iter().enumerate() {
            self.min[k] = self.min[k].min(*v);
            self.max[k] = self.max[k].max(*v);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.min[0] > self.max[0]
    }

    pub fn size(&self) -> Vec3 {
        if self.is_empty() {
            return [0.0; 3];
        }
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    pub fn diagonal(&self) -> f32 {
        let s = self.size();
        (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
    }

    /// The largest per axis gap between the boxes, 0 when they touch or
    /// overlap.
    pub fn gap(&self, other: &Aabb) -> f32 {
        (0..3)
            .map(|k| {
                (other.min[k] - self.max[k])
                    .max(self.min[k] - other.max[k])
                    .max(0.0)
            })
            .fold(0.0, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        distance(a, b) < 1e-4
    }

    #[test]
    fn inverse_undoes_a_transform() {
        // Scale 2, rotate 90 degrees about Y, translate (1, 2, 3).
        let m: Mat4 = [
            [0.0, 0.0, -2.0, 0.0],
            [0.0, 2.0, 0.0, 0.0],
            [2.0, 0.0, 0.0, 0.0],
            [1.0, 2.0, 3.0, 1.0],
        ];
        let inv = invert_affine(&m).expect("invertible");
        let p = [0.3, -1.2, 4.0];
        assert!(close(point(&inv, point(&m, p)), p));
        assert!(close(point(&mul(&m, &inv), p), p));
        assert!(invert_affine(&[[0.0; 4]; 4]).is_none());
    }

    #[test]
    fn boxes_measure_gaps() {
        let mut a = Aabb::EMPTY;
        assert!(a.is_empty());
        a.add([0.0, 0.0, 0.0]);
        a.add([1.0, 1.0, 1.0]);
        let mut b = Aabb::EMPTY;
        b.add([3.0, 0.0, 0.0]);
        b.add([4.0, 1.0, 1.0]);
        assert!((a.gap(&b) - 2.0).abs() < 1e-6);
        assert!((b.gap(&a) - 2.0).abs() < 1e-6);
        assert_eq!(a.gap(&a), 0.0);
        assert!((a.diagonal() - 3f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn segments_measure_to_their_nearest_point() {
        let (a, b) = ([0.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        assert!((segment_distance([1.0, 3.0, 0.0], a, b) - 3.0).abs() < 1e-6);
        assert!((segment_distance([-3.0, 4.0, 0.0], a, b) - 5.0).abs() < 1e-6);
        assert!((segment_distance([5.0, 0.0, 0.0], a, b) - 3.0).abs() < 1e-6);
        assert!((segment_distance([0.0, 1.0, 0.0], a, a) - 1.0).abs() < 1e-6);
    }
}
