//! FNV-1a 64: the state fingerprint stored in a replay's result.
//!
//! Not cryptographic. It detects edited inputs and diverging simulations, which is
//! what a replay check needs; it does not stop someone forging a matching file.

/// Incremental FNV-1a 64-bit hasher.
pub(crate) struct Fnv(u64);

impl Fnv {
    /// Start a fresh hash.
    pub(crate) const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    /// Mix in raw bytes.
    pub(crate) fn bytes(&mut self, data: &[u8]) {
        for &b in data {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    /// Mix in a `u64` (little endian).
    pub(crate) fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }

    /// Mix in a `u32` (little endian).
    pub(crate) fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }

    /// Mix in an `i32` (little endian).
    pub(crate) fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }

    /// Mix in the exact bit pattern of an `f32`.
    pub(crate) fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }

    /// Mix in a `usize` as 64 bits.
    pub(crate) fn usize(&mut self, v: usize) {
        self.u64(u64::try_from(v).unwrap_or(u64::MAX));
    }

    /// Mix in a flag.
    pub(crate) fn flag(&mut self, v: bool) {
        self.bytes(&[u8::from(v)]);
    }

    /// Sixteen lowercase hex digits.
    pub(crate) fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}
