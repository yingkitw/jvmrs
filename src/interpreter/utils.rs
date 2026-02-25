//! Bytecode reading utilities.

/// Read u16 (big-endian) from byte slice at offset.
pub fn read_u16(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16) << 8) | (data[offset + 1] as u16)
}

/// Read i16 (big-endian) from byte slice at offset.
pub fn read_i16(data: &[u8], offset: usize) -> i16 {
    ((data[offset] as i16) << 8) | (data[offset + 1] as i16)
}

/// Read u32 (big-endian) from byte slice at offset.
pub fn read_u32(data: &[u8], offset: usize) -> u32 {
    ((data[offset] as u32) << 24)
        | ((data[offset + 1] as u32) << 16)
        | ((data[offset + 2] as u32) << 8)
        | (data[offset + 3] as u32)
}
