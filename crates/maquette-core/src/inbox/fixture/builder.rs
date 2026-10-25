//! A small glTF builder that writes boxes, skins and materials into a GLB.

use serde_json::{Value, json};

use crate::inbox::images::write_png;

#[derive(Default)]
pub(super) struct Mesh {
    pub(super) positions: Vec<[f32; 3]>,
    pub(super) normals: Vec<[f32; 3]>,
    pub(super) uvs: Vec<[f32; 2]>,
    pub(super) indices: Vec<u32>,
    pub(super) joints: Vec<[u16; 4]>,
}

impl Mesh {
    /// Appends a box: 24 vertices (flat normals), 12 triangles.
    pub(super) fn cuboid(&mut self, center: [f32; 3], size: [f32; 3], joint: u16) {
        let h = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
        // Normal axis, sign, and the two axes spanning the face.
        let faces = [
            (0, 1.0, 1, 2),
            (0, -1.0, 2, 1),
            (1, 1.0, 2, 0),
            (1, -1.0, 0, 2),
            (2, 1.0, 0, 1),
            (2, -1.0, 1, 0),
        ];
        for (axis, sign, u, v) in faces {
            let base = u32::try_from(self.positions.len()).unwrap_or(u32::MAX);
            for (du, dv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let mut p = center;
                p[axis] += sign * h[axis];
                p[u] += du * h[u];
                p[v] += dv * h[v];
                let mut n = [0.0; 3];
                n[axis] = sign;
                self.positions.push(p);
                self.normals.push(n);
                self.uvs.push([(du + 1.0) / 2.0, (dv + 1.0) / 2.0]);
                self.joints.push([joint, 0, 0, 0]);
            }
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
}

impl Mesh {
    /// Appends a box whose faces are each split into `n` by `n` quads, so
    /// it has `12 * n * n` triangles: a dense model to decimate.
    pub(super) fn dense_cuboid(&mut self, center: [f32; 3], size: [f32; 3], n: u32) {
        let h = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
        let faces = [
            (0, 1.0, 1, 2),
            (0, -1.0, 2, 1),
            (1, 1.0, 2, 0),
            (1, -1.0, 0, 2),
            (2, 1.0, 0, 1),
            (2, -1.0, 1, 0),
        ];
        #[expect(clippy::cast_precision_loss, reason = "a fixture grid is tiny")]
        let step = |i: u32| (i as f32 / n as f32) * 2.0 - 1.0;
        for (axis, sign, u, v) in faces {
            let base = u32::try_from(self.positions.len()).unwrap_or(u32::MAX);
            for j in 0..=n {
                for i in 0..=n {
                    let mut p = center;
                    p[axis] += sign * h[axis];
                    p[u] += step(i) * h[u];
                    p[v] += step(j) * h[v];
                    let mut normal = [0.0; 3];
                    normal[axis] = sign;
                    self.positions.push(p);
                    self.normals.push(normal);
                    self.uvs
                        .push([(step(i) + 1.0) / 2.0, (step(j) + 1.0) / 2.0]);
                    self.joints.push([0; 4]);
                }
            }
            let row = n + 1;
            for j in 0..n {
                for i in 0..n {
                    let a = base + j * row + i;
                    let (b, c, d) = (a + 1, a + row + 1, a + row);
                    self.indices.extend_from_slice(&[a, b, c, a, c, d]);
                }
            }
        }
    }
}

#[derive(Default)]
pub(super) struct Builder {
    pub(super) bin: Vec<u8>,
    pub(super) views: Vec<Value>,
    pub(super) accessors: Vec<Value>,
}

impl Builder {
    pub(super) fn view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let mut view =
            json!({ "buffer": 0, "byteOffset": self.bin.len(), "byteLength": bytes.len() });
        if let Some(target) = target {
            view["target"] = json!(target);
        }
        self.bin.extend_from_slice(bytes);
        self.views.push(view);
        self.views.len() - 1
    }

    pub(super) fn floats(
        &mut self,
        data: &[f32],
        kind: &str,
        width: usize,
        target: Option<u32>,
        bounds: bool,
    ) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        let view = self.view(&bytes, target);
        let mut accessor = json!({
            "bufferView": view,
            "componentType": 5126,
            "count": data.len() / width,
            "type": kind,
        });
        if bounds {
            let mut min = vec![f32::INFINITY; width];
            let mut max = vec![f32::NEG_INFINITY; width];
            for item in data.chunks(width) {
                for (k, v) in item.iter().enumerate() {
                    min[k] = min[k].min(*v);
                    max[k] = max[k].max(*v);
                }
            }
            accessor["min"] = json!(min);
            accessor["max"] = json!(max);
        }
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }

    pub(super) fn indices(&mut self, data: &[u32]) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|i| i.to_le_bytes()).collect();
        let view = self.view(&bytes, Some(34963));
        self.accessors.push(json!({ "bufferView": view, "componentType": 5125, "count": data.len(), "type": "SCALAR" }));
        self.accessors.len() - 1
    }

    pub(super) fn joints(&mut self, data: &[[u16; 4]]) -> usize {
        let bytes: Vec<u8> = data
            .iter()
            .flatten()
            .flat_map(|j| j.to_le_bytes())
            .collect();
        let view = self.view(&bytes, Some(34962));
        self.accessors.push(json!({ "bufferView": view, "componentType": 5123, "count": data.len(), "type": "VEC4" }));
        self.accessors.len() - 1
    }

    /// The primitive JSON for `mesh`, with skin attributes when `skinned`.
    pub(super) fn primitive(&mut self, mesh: &Mesh, material: usize, skinned: bool) -> Value {
        let flat3 = |v: &[[f32; 3]]| v.iter().flatten().copied().collect::<Vec<f32>>();
        let position = self.floats(&flat3(&mesh.positions), "VEC3", 3, Some(34962), true);
        let normal = self.floats(&flat3(&mesh.normals), "VEC3", 3, Some(34962), false);
        let uv: Vec<f32> = mesh.uvs.iter().flatten().copied().collect();
        let uv = self.floats(&uv, "VEC2", 2, Some(34962), false);
        let indices = self.indices(&mesh.indices);
        let mut attributes = json!({ "POSITION": position, "NORMAL": normal, "TEXCOORD_0": uv });
        if skinned {
            let joints = self.joints(&mesh.joints);
            let weights: Vec<f32> = mesh
                .joints
                .iter()
                .flat_map(|_| [1.0, 0.0, 0.0, 0.0])
                .collect();
            let weights = self.floats(&weights, "VEC4", 4, Some(34962), false);
            attributes["JOINTS_0"] = json!(joints);
            attributes["WEIGHTS_0"] = json!(weights);
        }
        json!({ "attributes": attributes, "indices": indices, "material": material })
    }

    pub(super) fn finish(mut self, mut root: Value) -> Vec<u8> {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        root["asset"] = json!({ "version": "2.0", "generator": "Maquette fixtures" });
        root["buffers"] = json!([{ "byteLength": self.bin.len() }]);
        root["bufferViews"] = Value::Array(self.views);
        root["accessors"] = Value::Array(self.accessors);
        let mut text = serde_json::to_vec(&root).unwrap_or_default();
        while !text.len().is_multiple_of(4) {
            text.push(b' ');
        }
        let total = 12 + 8 + text.len() + 8 + self.bin.len();
        let mut out = Vec::with_capacity(total);
        let u = |n: usize| u32::try_from(n).unwrap_or(u32::MAX).to_le_bytes();
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&u(total));
        out.extend_from_slice(&u(text.len()));
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&text);
        out.extend_from_slice(&u(self.bin.len()));
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&self.bin);
        out
    }

    /// Embeds a checker texture and returns the image index count so far.
    pub(super) fn checker(&mut self, size: u32, a: [u8; 3], b: [u8; 3]) -> usize {
        let mut rgb = Vec::with_capacity((size * size * 3) as usize);
        for y in 0..size {
            for x in 0..size {
                let on = ((x / 8) + (y / 8)) % 2 == 0;
                rgb.extend_from_slice(if on { &a } else { &b });
            }
        }
        self.view(&write_png(size, size, &rgb), None)
    }
}
