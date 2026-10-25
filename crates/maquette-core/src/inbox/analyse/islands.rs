//! The merged objects heuristic: welded vertices joined into islands, and
//! islands grouped into objects.

use std::collections::HashMap;

use super::{GROUP_GAP, GROUP_MIN_SHARE, Islands, MAX_ISLANDS, count};
use crate::inbox::math::{Aabb, Vec3};

struct UnionFind(Vec<u32>);

impl UnionFind {
    fn new(n: usize) -> Self {
        Self(
            (0..n)
                .map(|i| u32::try_from(i).unwrap_or(u32::MAX))
                .collect(),
        )
    }

    fn find(&mut self, mut x: u32) -> u32 {
        while self.0[x as usize] != x {
            let parent = self.0[x as usize];
            self.0[x as usize] = self.0[parent as usize];
            x = parent;
        }
        x
    }

    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[ra as usize] = rb;
        }
    }
}

pub(super) fn primitive_islands(
    prim: &gltf::Primitive<'_>,
    buffers: &[Option<Vec<u8>>],
    name: &str,
) -> Option<Islands> {
    let reader = prim.reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()));
    let positions: Vec<Vec3> = reader.read_positions()?.collect();
    if positions.len() < 3 {
        return None;
    }
    let indices: Vec<u32> = match reader.read_indices() {
        Some(i) => i.into_u32().collect(),
        None => (0..u32::try_from(positions.len()).ok()?).collect(),
    };
    let mut bounds = Aabb::EMPTY;
    for p in &positions {
        bounds.add(*p);
    }
    let diagonal = bounds.diagonal().max(1e-6);
    // Weld vertices that share a position (UV and normal seams split them).
    let cell = diagonal * 1e-5;
    let mut welded: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let ids: Vec<u32> = positions
        .iter()
        .map(|p| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "grid cells of a bounded mesh"
            )]
            let key = (
                (p[0] / cell).round() as i64,
                (p[1] / cell).round() as i64,
                (p[2] / cell).round() as i64,
            );
            let next = u32::try_from(welded.len()).unwrap_or(u32::MAX);
            *welded.entry(key).or_insert(next)
        })
        .collect();
    let mut sets = UnionFind::new(welded.len());
    let triangles: Vec<[u32; 3]> = indices
        .chunks_exact(3)
        .filter_map(|t| {
            let v = [
                *ids.get(t[0] as usize)?,
                *ids.get(t[1] as usize)?,
                *ids.get(t[2] as usize)?,
            ];
            Some(v)
        })
        .collect();
    for t in &triangles {
        sets.union(t[0], t[1]);
        sets.union(t[1], t[2]);
    }
    let mut islands: HashMap<u32, (Aabb, u32)> = HashMap::new();
    for (tri, t) in triangles.iter().zip(indices.chunks_exact(3)) {
        let root = sets.find(tri[0]);
        let entry = islands.entry(root).or_insert((Aabb::EMPTY, 0));
        for &i in t {
            if let Some(p) = positions.get(i as usize) {
                entry.0.add(*p);
            }
        }
        entry.1 += 1;
    }
    let island_count = count(islands.len());
    let mut list: Vec<(Aabb, u32)> = islands.into_values().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1));
    list.truncate(MAX_ISLANDS);
    let mut groups = UnionFind::new(list.len());
    let gap = diagonal * GROUP_GAP;
    for i in 0..list.len() {
        for j in i + 1..list.len() {
            if list[i].0.gap(&list[j].0) <= gap {
                groups.union(count(i), count(j));
            }
        }
    }
    let mut tris_per_group: HashMap<u32, u32> = HashMap::new();
    for (i, island) in list.iter().enumerate() {
        *tris_per_group.entry(groups.find(count(i))).or_default() += island.1;
    }
    let total = count(triangles.len()).max(1) as f32;
    let significant = tris_per_group
        .values()
        .filter(|&&t| t as f32 / total >= GROUP_MIN_SHARE)
        .count();
    Some(Islands {
        mesh: name.to_string(),
        islands: island_count,
        groups: count(significant),
    })
}
