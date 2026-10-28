//! Simple noise-based terrain so the renderer has something to draw before
//! the client can receive chunks from a real server.

use crate::{Block, Chunk, ChunkPos, CHUNK_HEIGHT, CHUNK_SIZE};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

pub const SEA_LEVEL: usize = 62;

pub struct TerrainGenerator {
    height: Fbm<Perlin>,
    trees: Perlin,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            height: Fbm::<Perlin>::new(seed).set_octaves(5).set_frequency(0.004),
            trees: Perlin::new(seed.wrapping_add(1)),
        }
    }

    pub fn surface_height(&self, wx: i32, wz: i32) -> usize {
        let n = self.height.get([wx as f64, wz as f64]); // roughly -1..1
        let h = SEA_LEVEL as f64 + n * 40.0 + 4.0;
        h.clamp(1.0, (CHUNK_HEIGHT - 16) as f64) as usize
    }

    pub fn generate(&self, pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::default();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let wx = pos.x * CHUNK_SIZE as i32 + x as i32;
                let wz = pos.z * CHUNK_SIZE as i32 + z as i32;
                let h = self.surface_height(wx, wz);
                let beach = h <= SEA_LEVEL + 1;
                for y in 0..=h {
                    let block = if y == 0 {
                        Block::Bedrock
                    } else if y + 4 < h {
                        Block::Stone
                    } else if beach {
                        Block::Sand
                    } else if y == h {
                        if h > SEA_LEVEL + 45 {
                            Block::Snow
                        } else {
                            Block::Grass
                        }
                    } else {
                        Block::Dirt
                    };
                    chunk.set(x, y, z, block);
                }
                for y in h + 1..=SEA_LEVEL {
                    chunk.set(x, y, z, Block::Water);
                }
            }
        }
        self.place_trees(&mut chunk, pos);
        chunk
    }

    fn place_trees(&self, chunk: &mut Chunk, pos: ChunkPos) {
        // Keep trees 2 blocks from chunk edges so leaves never cross chunks.
        for z in 2..CHUNK_SIZE - 2 {
            for x in 2..CHUNK_SIZE - 2 {
                let wx = pos.x * CHUNK_SIZE as i32 + x as i32;
                let wz = pos.z * CHUNK_SIZE as i32 + z as i32;
                let n = self.trees.get([wx as f64 * 0.9, wz as f64 * 0.9]);
                if n < 0.7 {
                    continue;
                }
                let h = self.surface_height(wx, wz);
                if chunk.get(x, h, z) != Block::Grass || h + 7 >= CHUNK_HEIGHT {
                    continue;
                }
                let top = h + 5;
                for y in h + 1..=top {
                    chunk.set(x, y, z, Block::Log);
                }
                for y in top - 2..=top + 1 {
                    let r: i32 = if y > top { 1 } else { 2 };
                    for dz in -r..=r {
                        for dx in -r..=r {
                            let (lx, lz) = ((x as i32 + dx) as usize, (z as i32 + dz) as usize);
                            if chunk.get(lx, y, lz).is_air() {
                                chunk.set(lx, y, lz, Block::Leaves);
                            }
                        }
                    }
                }
            }
        }
    }
}
