/// A small built-in block set. Once the client joins real servers these will
/// be driven by the server's block-state registry instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Block {
    #[default]
    Air,
    Stone,
    Dirt,
    Grass,
    Sand,
    Water,
    Log,
    Leaves,
    Bedrock,
    Snow,
}

impl Block {
    pub fn is_opaque(self) -> bool {
        !matches!(self, Block::Air | Block::Water | Block::Leaves)
    }

    pub fn is_air(self) -> bool {
        self == Block::Air
    }

    /// Placeholder flat colours until a texture atlas is loaded: `(top, side)`.
    pub fn colors(self) -> ([f32; 3], [f32; 3]) {
        match self {
            Block::Air => ([0.0; 3], [0.0; 3]),
            Block::Stone => ([0.50, 0.50, 0.50], [0.47, 0.47, 0.47]),
            Block::Dirt => ([0.53, 0.38, 0.26], [0.53, 0.38, 0.26]),
            Block::Grass => ([0.36, 0.62, 0.25], [0.53, 0.38, 0.26]),
            Block::Sand => ([0.86, 0.81, 0.60], [0.84, 0.79, 0.58]),
            Block::Water => ([0.20, 0.35, 0.80], [0.20, 0.35, 0.80]),
            Block::Log => ([0.66, 0.53, 0.33], [0.40, 0.30, 0.18]),
            Block::Leaves => ([0.22, 0.48, 0.16], [0.22, 0.48, 0.16]),
            Block::Bedrock => ([0.20, 0.20, 0.20], [0.20, 0.20, 0.20]),
            Block::Snow => ([0.95, 0.97, 1.00], [0.80, 0.80, 0.82]),
        }
    }
}
