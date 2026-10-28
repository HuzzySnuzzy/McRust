//! Minecraft: Java Edition network protocol primitives.
//!
//! Reference: <https://minecraft.wiki/w/Java_Edition_protocol>

pub mod codec;
pub mod status;

/// Protocol version sent in the handshake. Status pings work with any value;
/// login/play must match the server's version exactly.
pub const DEFAULT_PROTOCOL_VERSION: i32 = 772; // 1.21.8

/// Connection state requested by the handshake packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum NextState {
    Status = 1,
    Login = 2,
    Transfer = 3,
}
