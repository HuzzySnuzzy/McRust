use crate::Block;
use std::collections::HashMap;

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
}

/// A 16 x 256 x 16 column of blocks, indexed `[y][z][x]`.
#[derive(Clone)]
pub struct Chunk {
    blocks: Box<[Block]>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            blocks: vec![Block::Air; CHUNK_SIZE * CHUNK_SIZE * CHUNK_HEIGHT].into_boxed_slice(),
        }
    }
}

impl Chunk {
    #[inline]
    fn index(x: usize, y: usize, z: usize) -> usize {
        (y * CHUNK_SIZE + z) * CHUNK_SIZE + x
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Block {
        self.blocks[Self::index(x, y, z)]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, block: Block) {
        self.blocks[Self::index(x, y, z)] = block;
    }
}

#[derive(Default)]
pub struct World {
    chunks: HashMap<ChunkPos, Chunk>,
}

impl World {
    pub fn insert(&mut self, pos: ChunkPos, chunk: Chunk) {
        self.chunks.insert(pos, chunk);
    }

    pub fn chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    pub fn chunk_positions(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.chunks.keys().copied()
    }

    /// Block at world coordinates. Unloaded chunks and out-of-range y read as air.
    pub fn block(&self, x: i32, y: i32, z: i32) -> Block {
        if y < 0 || y >= CHUNK_HEIGHT as i32 {
            return Block::Air;
        }
        let size = CHUNK_SIZE as i32;
        let pos = ChunkPos::new(x.div_euclid(size), z.div_euclid(size));
        match self.chunks.get(&pos) {
            Some(c) => c.get(
                x.rem_euclid(size) as usize,
                y as usize,
                z.rem_euclid(size) as usize,
            ),
            None => Block::Air,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_coordinates_map_to_correct_chunk() {
        let mut world = World::default();
        let mut chunk = Chunk::default();
        chunk.set(15, 10, 15, Block::Stone);
        world.insert(ChunkPos::new(-1, -1), chunk);
        assert_eq!(world.block(-1, 10, -1), Block::Stone);
        assert_eq!(world.block(0, 10, 0), Block::Air);
        assert_eq!(world.block(-1, -5, -1), Block::Air);
    }
}
