// Utility functions and helpers
use std::time::{SystemTime, UNIX_EPOCH};

/// Get current timestamp in microseconds
pub fn get_timestamp_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

/// Convert byte array to different integer types
pub trait ByteArrayExt {
    fn to_u16_le(&self) -> Option<u16>;
    fn to_u32_le(&self) -> Option<u32>;
    fn to_u64_le(&self) -> Option<u64>;
}

impl ByteArrayExt for [u8] {
    fn to_u16_le(&self) -> Option<u16> {
        if self.len() >= 2 {
            Some(u16::from_le_bytes([self[0], self[1]]))
        } else {
            None
        }
    }

    fn to_u32_le(&self) -> Option<u32> {
        if self.len() >= 4 {
            Some(u32::from_le_bytes([self[0], self[1], self[2], self[3]]))
        } else {
            None
        }
    }

    fn to_u64_le(&self) -> Option<u64> {
        if self.len() >= 8 {
            Some(u64::from_le_bytes([
                self[0], self[1], self[2], self[3], self[4], self[5], self[6], self[7],
            ]))
        } else {
            None
        }
    }
}
