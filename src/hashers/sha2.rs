use super::Hasher;
use sha2::Digest;

/// SHA256 hasher wrapper
pub struct Sha256Hasher;

impl Hasher for Sha256Hasher {
    fn hash_bytes(bytes: &[u8]) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.input(bytes);
        hasher.result().as_slice().to_owned()
    }
}

/// SHA512 hasher wrapper
pub struct Sha512Hasher;

impl Hasher for Sha512Hasher {
    fn hash_bytes(bytes: &[u8]) -> Vec<u8> {
        let mut hasher = sha2::Sha512::new();
        hasher.input(bytes);
        hasher.result().as_slice().to_owned()
    }
}
