pub mod sha2;

/// Hasher trait, which provides a pluggable way to swap hashing algorithm used
pub trait Hasher {
    /// Returns the hashed bytes
    /// # Arguments
    /// * `bytes` - byte slice to hash
    fn hash_bytes(bytes: &[u8]) -> Vec<u8>;
}
