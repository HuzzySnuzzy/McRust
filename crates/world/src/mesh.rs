//! Converts chunks into triangle meshes, emitting only faces that are visible.

use crate::{Block, ChunkPos, World, CHUNK_HEIGHT, CHUNK_SIZE};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

struct Face {
    normal: [i32; 3],
    /// Corners of the unit cube face, counter-clockwise when viewed from outside.
    corners: [[f32; 3]; 4],
    shade: f32,
}

const FACES: [Face; 6] = [
    // +X
    Face {
        normal: [1, 0, 0],
        corners: [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]],
        shade: 0.8,
    },
    // -X
    Face {
        normal: [-1, 0, 0],
        corners: [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]],
        shade: 0.8,
    },
    // +Y
    Face {
        normal: [0, 1, 0],
        corners: [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
        shade: 1.0,
    },
    // -Y
    Face {
        normal: [0, -1, 0],
        corners: [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
        shade: 0.5,
    },
    // +Z
    Face {
        normal: [0, 0, 1],
        corners: [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
        shade: 0.65,
    },
    // -Z
    Face {
        normal: [0, 0, -1],
        corners: [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
        shade: 0.65,
    },
];

fn face_visible(block: Block, neighbor: Block) -> bool {
    if neighbor.is_opaque() {
        return false;
    }
    // Don't draw internal faces between two blocks of the same transparent type.
    !(block == neighbor && block == Block::Water)
}

/// Builds a mesh for one chunk, in world coordinates. Neighbouring chunks are
/// consulted through `world` so faces on chunk borders are culled correctly.
pub fn mesh_chunk(world: &World, pos: ChunkPos) -> Mesh {
    let mut mesh = Mesh::default();
    let Some(chunk) = world.chunk(pos) else {
        return mesh;
    };
    let ox = pos.x * CHUNK_SIZE as i32;
    let oz = pos.z * CHUNK_SIZE as i32;

    for y in 0..CHUNK_HEIGHT {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let block = chunk.get(x, y, z);
                if block.is_air() {
                    continue;
                }
                let (wx, wy, wz) = (ox + x as i32, y as i32, oz + z as i32);
                let (top, side) = block.colors();
                for face in &FACES {
                    let [nx, ny, nz] = face.normal;
                    let inside = x as i32 + nx >= 0
                        && x as i32 + nx < CHUNK_SIZE as i32
                        && z as i32 + nz >= 0
                        && z as i32 + nz < CHUNK_SIZE as i32
                        && wy + ny >= 0
                        && wy + ny < CHUNK_HEIGHT as i32;
                    let neighbor = if inside {
                        chunk.get(
                            (x as i32 + nx) as usize,
                            (wy + ny) as usize,
                            (z as i32 + nz) as usize,
                        )
                    } else {
                        world.block(wx + nx, wy + ny, wz + nz)
                    };
                    if !face_visible(block, neighbor) {
                        continue;
                    }
                    let base = if ny == 1 { top } else { side };
                    let color = base.map(|c| c * face.shade);
                    let start = mesh.vertices.len() as u32;
                    for c in face.corners {
                        let mut position = [wx as f32 + c[0], wy as f32 + c[1], wz as f32 + c[2]];
                        if block == Block::Water && ny == 1 {
                            position[1] -= 0.1;
                        }
                        mesh.vertices.push(Vertex { position, color });
                    }
                    mesh.indices.extend_from_slice(&[
                        start,
                        start + 1,
                        start + 2,
                        start,
                        start + 2,
                        start + 3,
                    ]);
                }
            }
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Chunk;

    #[test]
    fn single_block_has_six_faces() {
        let mut world = World::default();
        let mut chunk = Chunk::default();
        chunk.set(5, 5, 5, Block::Stone);
        world.insert(ChunkPos::new(0, 0), chunk);
        let mesh = mesh_chunk(&world, ChunkPos::new(0, 0));
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn adjacent_blocks_cull_shared_face() {
        let mut world = World::default();
        let mut chunk = Chunk::default();
        chunk.set(5, 5, 5, Block::Stone);
        chunk.set(6, 5, 5, Block::Stone);
        world.insert(ChunkPos::new(0, 0), chunk);
        let mesh = mesh_chunk(&world, ChunkPos::new(0, 0));
        assert_eq!(mesh.indices.len() / 6, 10);
    }

    #[test]
    fn faces_across_chunk_border_are_culled() {
        let mut world = World::default();
        let mut a = Chunk::default();
        a.set(15, 5, 0, Block::Stone);
        let mut b = Chunk::default();
        b.set(0, 5, 0, Block::Stone);
        world.insert(ChunkPos::new(0, 0), a);
        world.insert(ChunkPos::new(1, 0), b);
        assert_eq!(mesh_chunk(&world, ChunkPos::new(0, 0)).indices.len() / 6, 5);
    }

    #[test]
    fn faces_wind_outward() {
        // Cross product of the first two edges must point along the normal.
        for f in &FACES {
            let [a, b, c, _] = f.corners;
            let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let expected = f.normal.map(|v| v as f32);
            assert_eq!(n, expected, "face {:?}", f.normal);
        }
    }
}
