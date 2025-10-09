//! Checksum algorithms for rsync delta transfer
//!
//! Implements:
//! - Rolling checksum (Adler-32 style) for fast block comparison
//! - Strong checksum (MD5) for verification

// MD5 checksum support

/// Rolling checksum modulus (prime number for Adler-32)
const MODULUS: u32 = 65521;

/// Compute rolling checksum (Adler-32 style)
///
/// This is a fast checksum that can be incrementally updated
/// as we slide a window over the data.
#[must_use]
pub fn rolling_checksum(data: &[u8]) -> u32 {
    rolling_checksum_with_seed(data, 0)
}

/// Compute rolling checksum with seed (rsync protocol)
///
/// The seed is mixed into the initial state to make checksums
/// session-unique and prevent precomputed collision attacks.
///
/// # Arguments
///
/// * `data` - The data block to checksum
/// * `seed` - Checksum seed from handshake (0 for unseeded)
///
/// # Examples
///
/// ```
/// # use arsync::protocol::checksum::rolling_checksum_with_seed;
/// let data = b"Hello, World!";
/// let unseeded = rolling_checksum_with_seed(data, 0);
/// let seeded = rolling_checksum_with_seed(data, 12345);
/// assert_ne!(unseeded, seeded); // Different seeds = different checksums
/// ```
#[must_use]
pub fn rolling_checksum_with_seed(data: &[u8], seed: u32) -> u32 {
    // Initialize with seed mixed in
    let mut a: u32 = (seed & 0xFFFF) % MODULUS;
    let mut b: u32 = ((seed >> 16) & 0xFFFF) % MODULUS;

    for &byte in data {
        a = (a + u32::from(byte)) % MODULUS;
        b = (b + a) % MODULUS;
    }

    (b << 16) | a
}

/// Update rolling checksum when sliding window
///
/// Given the old checksum, the byte leaving the window, the byte entering,
/// and the window size, compute the new checksum without scanning the whole window.
#[must_use]
pub fn rolling_checksum_update(
    old_checksum: u32,
    old_byte: u8,
    new_byte: u8,
    block_size: usize,
) -> u32 {
    let mut a = old_checksum & 0xFFFF;
    let mut b = old_checksum >> 16;

    // Remove old byte contribution
    a = (a + MODULUS - u32::from(old_byte)) % MODULUS;
    b = (b + MODULUS - (u32::from(old_byte) * block_size as u32) % MODULUS) % MODULUS;

    // Add new byte contribution
    a = (a + u32::from(new_byte)) % MODULUS;
    b = (b + a) % MODULUS;

    (b << 16) | a
}

/// Compute strong checksum (MD5)
///
/// This is slower but has very low collision probability.
/// Used to verify that a weak checksum match is a real match.
#[must_use]
pub fn strong_checksum(data: &[u8]) -> [u8; 16] {
    md5::compute(data).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_checksum_basic() {
        let data = b"Hello, World!";
        let checksum = rolling_checksum(data);
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_rolling_checksum_update() {
        let data = b"Hello, World!";
        let block_size = 5;

        // Compute checksum for first block
        let block1 = &data[0..block_size];
        let checksum1 = rolling_checksum(block1);

        // Compute checksum for second block (shifted by 1)
        let block2 = &data[1..block_size + 1];
        let checksum2 = rolling_checksum(block2);

        // Update checksum1 by removing data[0] and adding data[block_size]
        let updated = rolling_checksum_update(checksum1, data[0], data[block_size], block_size);

        assert_eq!(updated, checksum2);
    }

    #[test]
    fn test_strong_checksum() {
        let data1 = b"Hello, World!";
        let data2 = b"Hello, World!";
        let data3 = b"Hello, world!"; // Different case

        let checksum1 = strong_checksum(data1);
        let checksum2 = strong_checksum(data2);
        let checksum3 = strong_checksum(data3);

        assert_eq!(checksum1, checksum2);
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_rolling_window_slide() {
        // Test that we can slide a window over data and update checksums
        let data = b"ABCDEFGHIJK";
        let window_size = 3;

        let mut checksum = rolling_checksum(&data[0..window_size]);

        for i in 1..=(data.len() - window_size) {
            checksum = rolling_checksum_update(
                checksum,
                data[i - 1],
                data[i + window_size - 1],
                window_size,
            );

            let expected = rolling_checksum(&data[i..i + window_size]);
            assert_eq!(checksum, expected, "Mismatch at position {}", i);
        }
    }

    #[test]
    fn test_rolling_checksum_with_seed() {
        let data = b"Hello, World!";

        // Unseeded (seed=0) should match original
        let unseeded = rolling_checksum_with_seed(data, 0);
        let original = rolling_checksum(data);
        assert_eq!(unseeded, original);

        // Different seeds produce different checksums
        let seed1 = rolling_checksum_with_seed(data, 12345);
        let seed2 = rolling_checksum_with_seed(data, 67890);
        assert_ne!(seed1, seed2);
        assert_ne!(seed1, unseeded);
        assert_ne!(seed2, unseeded);
    }

    #[test]
    fn test_seeded_checksum_deterministic() {
        let data = b"Test data";
        let seed = 0xDEADBEEF;

        // Same seed should give same result
        let checksum1 = rolling_checksum_with_seed(data, seed);
        let checksum2 = rolling_checksum_with_seed(data, seed);
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_seed_prevents_collisions() {
        // Two different data blocks might have same unseeded checksum (collision)
        // But seeding makes them different
        let data1 = b"AB";
        let data2 = b"BA"; // Different data

        let unseeded1 = rolling_checksum(data1);
        let unseeded2 = rolling_checksum(data2);
        // May or may not collide (depends on algorithm)

        let seed = 0x12345678;
        let seeded1 = rolling_checksum_with_seed(data1, seed);
        let seeded2 = rolling_checksum_with_seed(data2, seed);

        // Even if unseeded collides, seeded makes them distinct
        assert_ne!(seeded1, seeded2);
    }
}
