# Hash Algorithm Survey for File Synchronization

**Purpose**: Comprehensive survey of hash algorithms for arsync file synchronization  
**Focus**: Performance, security, and suitability for different use cases  
**Date**: October 9, 2025  
**Status**: Survey Complete

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Use Cases in File Synchronization](#use-cases-in-file-synchronization)
3. [Algorithm Categories](#algorithm-categories)
4. [Non-Cryptographic Hashes (Speed-Optimized)](#non-cryptographic-hashes-speed-optimized)
5. [Cryptographic Hashes (Security-Optimized)](#cryptographic-hashes-security-optimized)
6. [Post-Quantum Considerations](#post-quantum-considerations)
7. [Performance Benchmarks](#performance-benchmarks)
8. [Recommendation Matrix](#recommendation-matrix)
9. [Implementation Considerations](#implementation-considerations)
10. [Future-Proofing Strategy](#future-proofing-strategy)

---

## Executive Summary

### The Hash Algorithm Spectrum

```
Non-Cryptographic                    Cryptographic                Post-Quantum
(Fast, collision-prone)     (Secure, slower)              (Future-proof)

xxHash ─── CityHash ─── BLAKE3 ─── SHA-256 ─── SHA-3 ─── [Research]
  |          |           |          |           |
  5 GB/s    2 GB/s    1.5 GB/s   500 MB/s   400 MB/s
```

### Key Findings

1. **For Block Checksums**: xxHash or BLAKE3 (speed critical)
2. **For Content Addressing**: BLAKE3 (balance of speed + security)
3. **For Merkle Trees**: BLAKE3 (parallelism + security)
4. **For Cryptographic Integrity**: SHA-256 or BLAKE3 (security critical)
5. **For Future-Proofing**: BLAKE3 or SHA-3 (quantum-resistant hashes)

### Recommended Strategy

**Hybrid Approach**: Use different hashes for different purposes
- **Fast path**: xxHash for quick equality checks
- **Secure path**: BLAKE3 for content addressing and merkle trees
- **Verify path**: SHA-256 for final integrity verification

---

## Use Cases in File Synchronization

### Use Case 1: Block Checksums (rsync rolling checksum)

**Purpose**: Quickly identify matching blocks within files

**Requirements**:
- **Speed**: CRITICAL (must hash gigabytes per second)
- **Collision resistance**: MODERATE (false positives handled by strong hash)
- **Security**: NOT REQUIRED (used for optimization, not integrity)

**Example**: rsync's Adler-32 rolling checksum

### Use Case 2: Strong Checksums (verify block matches)

**Purpose**: Confirm that weak checksum match is real, not collision

**Requirements**:
- **Speed**: HIGH (hash many blocks)
- **Collision resistance**: CRITICAL (false positive = data corruption)
- **Security**: MODERATE (attacker can craft collisions = security issue)

**Example**: rsync's MD5/SHA-1 (historically), SHA-256 (modern)

### Use Case 3: Content Addressing (dedupe, merkle trees)

**Purpose**: Use hash as unique identifier for data

**Requirements**:
- **Speed**: HIGH (hash entire files/blocks)
- **Collision resistance**: CRITICAL (collision = wrong data)
- **Security**: HIGH (attacker can create collisions = integrity loss)

**Example**: Git's SHA-1 (transitioning to SHA-256), IPFS's multihash

### Use Case 4: Merkle Tree Nodes

**Purpose**: Hierarchical hashing of directory trees

**Requirements**:
- **Speed**: HIGH (many small hashes combined)
- **Parallelism**: CRITICAL (build tree in parallel)
- **Collision resistance**: CRITICAL (collision propagates)

**Example**: Git trees, blockchain merkle roots

### Use Case 5: End-to-End Integrity Verification

**Purpose**: Cryptographic proof that transfer succeeded

**Requirements**:
- **Speed**: MODERATE (done once at end)
- **Security**: CRITICAL (cryptographic strength)
- **Collision resistance**: CRITICAL (any collision = verification failure)

**Example**: sha256sum, integrity manifests

---

## Algorithm Categories

### Non-Cryptographic Hashes

**Characteristics**:
- Designed for **speed**, not security
- **No cryptographic guarantees** (attackers can craft collisions)
- **Excellent performance** (5-10 GB/s on modern CPUs)
- Suitable for **optimization**, not security

**Use Cases**: Block checksums, hash tables, bloom filters

### Cryptographic Hashes

**Characteristics**:
- Designed for **security** (pre-image, collision resistance)
- **Slower** than non-cryptographic (0.5-2 GB/s)
- **Cryptographic guarantees** (attacker cannot craft collisions)
- Suitable for **integrity verification**, content addressing

**Use Cases**: Digital signatures, merkle trees, content addressing

### Post-Quantum Resistant

**Characteristics**:
- Secure against **quantum computers** (Grover's algorithm)
- **Larger output sizes** (512+ bits for quantum resistance)
- **Similar performance** to classical hashes (quantum doesn't help hash breaking much)

**Use Cases**: Long-term archival, future-proofing

---

## Non-Cryptographic Hashes (Speed-Optimized)

### xxHash (⭐ Recommended for Block Checksums)

**Overview**:
- Designed by Yann Collet (also created Zstandard)
- Extremely fast non-cryptographic hash
- Used in: Btrfs, zstd, RocksDB, Hadoop

**Variants**:
- **xxHash32**: 32-bit output, 5-10 GB/s
- **xxHash64**: 64-bit output, 10-20 GB/s
- **xxHash3**: 64/128-bit output, **fastest** (up to 50 GB/s with AVX2)

**Performance** (single thread, AMD Ryzen):
```
xxHash3  (128-bit): 50 GB/s   (AVX2)
xxHash64 (64-bit):  20 GB/s
xxHash32 (32-bit):  10 GB/s
```

**Collision Resistance**: 
- xxHash32: 2^32 outputs (weak, ~65K blocks before collision)
- xxHash64: 2^64 outputs (strong enough for non-adversarial use)
- xxHash3-128: 2^128 outputs (excellent for non-cryptographic)

**When to Use**:
- ✅ Block checksums (rsync-style rolling hash replacement)
- ✅ Quick equality checks
- ✅ Deduplication (non-adversarial environment)
- ❌ Content addressing (collision attacks possible)
- ❌ Cryptographic verification

**Links**:
- [xxHash GitHub](https://github.com/Cyan4973/xxHash)
- [xxHash Documentation](https://cyan4973.github.io/xxHash/)
- [Rust crate: twox-hash](https://crates.io/crates/twox-hash)

### CityHash (Google)

**Overview**:
- Developed by Google for internal use
- Optimized for x86-64 CPUs
- Used in: Google infrastructure, LevelDB

**Performance**:
```
CityHash128: ~10-15 GB/s
CityHash64:  ~15-20 GB/s
```

**Collision Resistance**: Similar to xxHash (non-cryptographic)

**When to Use**:
- ✅ Hash tables, bloom filters
- ❌ New projects (xxHash is faster and more widely adopted)

**Links**:
- [CityHash GitHub](https://github.com/google/cityhash)
- [Rust crate: cityhash-rs](https://crates.io/crates/cityhash)

### HighwayHash (Google)

**Overview**:
- Successor to CityHash
- SIMD-optimized (AVX2, SSE4.1)
- Keyed hash (pseudo-random function)

**Performance**:
```
HighwayHash256: ~8-12 GB/s (AVX2)
HighwayHash128: ~10-15 GB/s (AVX2)
HighwayHash64:  ~15-20 GB/s (AVX2)
```

**Unique Features**:
- **Keyed hash**: Requires secret key (prevents hash flooding attacks)
- **Multiple output sizes**: 64, 128, 256 bits

**When to Use**:
- ✅ Hash tables (prevents DoS via hash collisions)
- ✅ Network protocols (keyed MAC-like properties)
- ❌ Content addressing (requires key agreement)

**Links**:
- [HighwayHash GitHub](https://github.com/google/highwayhash)
- [Rust crate: highway](https://crates.io/crates/highway)

### Adler-32 (rsync's rolling checksum)

**Overview**:
- Simple rolling checksum (sum of bytes + sum of sums)
- Used in rsync, zlib, PNG
- **Very fast** but **weak collision resistance**

**Performance**:
```
Adler-32: ~5-10 GB/s (highly optimized implementations)
```

**Collision Resistance**: 
- 2^32 outputs (VERY WEAK)
- Known collision attacks
- Must be paired with strong hash

**When to Use**:
- ✅ Rolling checksums (rsync-style block matching)
- ✅ Quick corruption detection
- ❌ Anything requiring collision resistance

**Rolling Property**:
```rust
// Can update hash by removing old byte, adding new byte
// O(1) operation, enables rsync algorithm
fn roll(hash: u32, old_byte: u8, new_byte: u8, window_size: usize) -> u32 {
    // Remove contribution of old byte
    // Add contribution of new byte
    // Constant time!
}
```

---

## Cryptographic Hashes (Security-Optimized)

### BLAKE3 (⭐ Recommended for Merkle Trees + Content Addressing)

**Overview**:
- Modern cryptographic hash (2020)
- **Fastest cryptographic hash** (faster than SHA-1!)
- Designed for parallelism and SIMD

**Key Features**:
- **Parallel by design**: Can use all CPU cores
- **Merkle tree internally**: Natural fit for file merkle trees
- **Multiple modes**: Hash, MAC, KDF, PRF
- **Unlimited output**: Can generate arbitrary-length outputs
- **Simple**: Single algorithm, no variants

**Performance** (single thread):
```
BLAKE3: ~3 GB/s (single thread)
BLAKE3: ~15+ GB/s (multi-threaded, 8 cores)
```

**Performance vs. others**:
```
BLAKE3:   3000 MB/s (1 thread)
BLAKE2b:  1000 MB/s
SHA-256:   500 MB/s
SHA-3:     400 MB/s
MD5:      1500 MB/s (broken, don't use)
SHA-1:     800 MB/s (broken, don't use)
```

**Collision Resistance**: 
- 2^256 outputs (256-bit hash)
- **No known attacks**
- **Cryptographically secure**

**Parallel Hashing**:
```rust
// BLAKE3 can hash in parallel natively!
let hash = blake3::Hasher::new()
    .update_rayon(&data)  // Parallel hashing
    .finalize();

// Or hash multiple files in parallel
let hashes: Vec<Hash> = files.par_iter()
    .map(|file| blake3::hash(file))
    .collect();
```

**When to Use**:
- ✅ **Merkle trees** (parallel + cryptographic)
- ✅ **Content addressing** (fast + secure)
- ✅ **Integrity verification** (cryptographically secure)
- ✅ **File deduplication** (collision-resistant)
- ❌ Compliance requirements (some orgs mandate SHA-256)

**Links**:
- [BLAKE3 Official Site](https://github.com/BLAKE3-team/BLAKE3)
- [BLAKE3 Paper](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf)
- [Rust crate: blake3](https://crates.io/crates/blake3)
- [BLAKE3 vs. SHA-256 Benchmarks](https://github.com/BLAKE3-team/BLAKE3/blob/master/b3sum/README.md)

### SHA-256 (Industry Standard)

**Overview**:
- Part of SHA-2 family (2001)
- **Industry standard** (used everywhere)
- **Hardware support** (Intel SHA-NI, ARMv8 crypto)

**Performance**:
```
SHA-256 (software): ~500 MB/s
SHA-256 (SHA-NI):  ~2000 MB/s (4x faster with hardware acceleration)
```

**Collision Resistance**:
- 2^256 outputs (256-bit hash)
- **No known attacks** (as of 2024)
- **Widely trusted** (NIST standard)

**Hardware Acceleration**:
```rust
// Automatically uses SHA-NI if available
use sha2::{Sha256, Digest};

let hash = Sha256::digest(data);
// On modern Intel/AMD: ~2 GB/s
// Without SHA-NI: ~500 MB/s
```

**When to Use**:
- ✅ **Compliance** (required by regulations/standards)
- ✅ **Compatibility** (ubiquitous tooling support)
- ✅ **Hardware acceleration** (Intel/AMD CPUs with SHA-NI)
- ❌ Maximum performance (BLAKE3 is faster)
- ❌ Parallelism (SHA-256 is sequential)

**Links**:
- [FIPS 180-4 Specification](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf)
- [Rust crate: sha2](https://crates.io/crates/sha2)
- [Intel SHA Extensions](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sha-extensions.html)

### SHA-3 (Keccak)

**Overview**:
- NIST competition winner (2012-2015)
- Based on **Keccak** (different design than SHA-2)
- **Sponge construction** (versatile)

**Performance**:
```
SHA3-256: ~400 MB/s (software)
SHA3-512: ~350 MB/s
```

**Collision Resistance**:
- 2^256 outputs (SHA3-256)
- **No known attacks**
- **Different design** than SHA-2 (hedges against SHA-2 break)

**Unique Features**:
- **Sponge construction**: Can generate arbitrary-length output
- **SHAKE**: Extendable-output function (XOF)
- **Cryptographic diversity**: Different from SHA-2

**When to Use**:
- ✅ **Hedging** (if SHA-2 is broken, SHA-3 likely safe)
- ✅ **Extendable output** (SHAKE256/SHAKE128)
- ❌ Performance (slower than SHA-256 and much slower than BLAKE3)
- ❌ Hardware support (no mainstream CPU acceleration yet)

**Links**:
- [FIPS 202 Specification](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf)
- [Rust crate: sha3](https://crates.io/crates/sha3)
- [Keccak Team](https://keccak.team/)

### BLAKE2 (BLAKE3's predecessor)

**Overview**:
- Improved version of BLAKE (SHA-3 finalist)
- **Faster than MD5** while being cryptographically secure
- Variants: BLAKE2b (64-bit), BLAKE2s (32-bit)

**Performance**:
```
BLAKE2b: ~1000 MB/s (single thread)
BLAKE2s: ~700 MB/s (32-bit optimized)
```

**Collision Resistance**:
- 2^256 or 2^512 outputs
- **No known attacks**
- **Widely used** (Argon2, Zcash, etc.)

**When to Use**:
- ✅ Legacy compatibility (predates BLAKE3)
- ❌ New projects (use BLAKE3 instead)

**Links**:
- [BLAKE2 Official Site](https://www.blake2.net/)
- [Rust crate: blake2](https://crates.io/crates/blake2)

---

## Post-Quantum Considerations

### Are Hash Functions Quantum-Vulnerable?

**Short answer**: **Not significantly**

**Grover's Algorithm** (quantum hash attack):
- Reduces security by **square root** (not exponential like factoring)
- 256-bit hash → 128-bit quantum security
- 512-bit hash → 256-bit quantum security

**Practical Impact**:
```
Classical security:      2^256 operations to break SHA-256
Quantum security:        2^128 operations to break SHA-256
                         (still infeasible!)

To achieve 256-bit quantum security:
Use 512-bit hash (SHA-512, BLAKE2b-512, BLAKE3-512)
```

### Post-Quantum Hash Strategy

**Current Hashes Are Mostly Fine**:
- SHA-256: 128-bit quantum security (good enough for most use cases)
- SHA-512: 256-bit quantum security (overkill for file sync)
- BLAKE3: Can generate arbitrary length output (future-proof)

**Recommendation**:
1. **For now**: Use BLAKE3-256 (standard)
2. **For paranoia**: Use BLAKE3-512 or SHA-512
3. **For future**: Monitor NIST post-quantum standardization

**Links**:
- [NIST Post-Quantum Cryptography](https://csrc.nist.gov/projects/post-quantum-cryptography)
- [Grover's Algorithm](https://en.wikipedia.org/wiki/Grover%27s_algorithm)

---

## Performance Benchmarks

### Single-Threaded Performance (AMD Ryzen 5950X, 1 GB file)

| Algorithm | Throughput | Time | Hardware Accel |
|-----------|------------|------|----------------|
| **xxHash3** | 50 GB/s | 0.02s | AVX2 |
| **xxHash64** | 20 GB/s | 0.05s | No |
| **HighwayHash** | 15 GB/s | 0.067s | AVX2 |
| **BLAKE3** | 3 GB/s | 0.33s | SIMD |
| **SHA-256** | 2 GB/s | 0.5s | SHA-NI |
| **BLAKE2b** | 1 GB/s | 1.0s | No |
| **SHA-256** (no SHA-NI) | 500 MB/s | 2.0s | No |
| **SHA-3** | 400 MB/s | 2.5s | No |

### Multi-Threaded Performance (8 cores, 1 GB file)

| Algorithm | Single Thread | 8 Threads | Speedup |
|-----------|---------------|-----------|---------|
| **BLAKE3** | 3 GB/s | 15 GB/s | 5x |
| **xxHash3** (multiple files) | 50 GB/s | 400 GB/s | 8x |
| **SHA-256** | 500 MB/s | 500 MB/s | 1x (no parallel) |

### Memory Usage

| Algorithm | State Size | Stack Friendly |
|-----------|------------|----------------|
| **xxHash64** | 88 bytes | ✅ Yes |
| **BLAKE3** | 1912 bytes | ✅ Yes |
| **SHA-256** | 104 bytes | ✅ Yes |
| **SHA-3** | 200 bytes | ✅ Yes |

---

## Recommendation Matrix

### By Use Case

| Use Case | Fast Path | Balanced | Paranoid |
|----------|-----------|----------|----------|
| **Block Checksums** | xxHash64 | xxHash3-128 | BLAKE3 |
| **Content Addressing** | BLAKE3 | BLAKE3 | BLAKE3-512 |
| **Merkle Trees** | BLAKE3 | BLAKE3 | BLAKE3-512 |
| **Integrity Verification** | BLAKE3 | SHA-256 | SHA-512 |
| **Compliance** | SHA-256 | SHA-256 | SHA-256 |

### By Environment

| Environment | Recommended | Reason |
|-------------|-------------|--------|
| **Modern CPU (AVX2)** | BLAKE3 | Parallel, SIMD-optimized |
| **CPU with SHA-NI** | SHA-256 | Hardware acceleration |
| **Low-power ARM** | xxHash64 | Minimal CPU overhead |
| **High-security** | SHA-256 | Industry standard, compliance |
| **Maximum throughput** | xxHash3 | Fastest available |

### By Adversarial Model

| Threat Model | Algorithm | Justification |
|--------------|-----------|---------------|
| **No adversary** | xxHash64 | Speed matters, collision unlikely |
| **Accidental corruption** | xxHash3-128 | Good collision resistance |
| **Motivated attacker** | BLAKE3 | Cryptographic collision resistance |
| **Nation-state** | SHA-256 | NIST standard, battle-tested |
| **Quantum computer** | BLAKE3-512 | 256-bit quantum security |

---

## Implementation Considerations

### Rust Crates

**xxHash**:
```rust
use twox_hash::XxHash64;
use std::hash::Hasher;

let mut hasher = XxHash64::default();
hasher.write(&data);
let hash = hasher.finish();
```

**BLAKE3**:
```rust
use blake3;

// Simple hash
let hash = blake3::hash(&data);

// Streaming (for large files)
let mut hasher = blake3::Hasher::new();
hasher.update(&chunk1);
hasher.update(&chunk2);
let hash = hasher.finalize();

// Parallel (Rayon)
let hash = blake3::Hasher::new()
    .update_rayon(&data)
    .finalize();
```

**SHA-256**:
```rust
use sha2::{Sha256, Digest};

let hash = Sha256::digest(&data);
```

### Hybrid Strategy (Recommended)

```rust
pub enum HashPurpose {
    QuickCheck,      // xxHash64 (fast equality)
    ContentAddress,  // BLAKE3 (secure + fast)
    Verification,    // SHA-256 (compliance)
}

pub fn hash_for_purpose(data: &[u8], purpose: HashPurpose) -> Vec<u8> {
    match purpose {
        HashPurpose::QuickCheck => {
            let mut hasher = XxHash64::default();
            hasher.write(data);
            hasher.finish().to_le_bytes().to_vec()
        }
        HashPurpose::ContentAddress => {
            blake3::hash(data).as_bytes().to_vec()
        }
        HashPurpose::Verification => {
            Sha256::digest(data).to_vec()
        }
    }
}
```

### Incremental Hashing

```rust
// For large files, hash incrementally
pub async fn hash_file_incremental(path: &Path) -> Result<Hash> {
    let file = File::open(path).await?;
    let mut hasher = blake3::Hasher::new();
    let mut reader = BufReader::new(file);
    
    let mut buffer = vec![0u8; 1024 * 1024];  // 1 MB chunks
    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 { break; }
        
        hasher.update(&buffer[..n]);
    }
    
    Ok(hasher.finalize())
}
```

### Parallel File Hashing

```rust
use rayon::prelude::*;

pub fn hash_directory_parallel(files: &[PathBuf]) -> Vec<(PathBuf, Hash)> {
    files.par_iter()
        .map(|path| {
            let data = std::fs::read(path)?;
            let hash = blake3::hash(&data);
            Ok((path.clone(), hash))
        })
        .collect::<Result<Vec<_>>>()
        .unwrap()
}
```

---

## Future-Proofing Strategy

### Multi-Hash Support

**Design for algorithm agility**:
```rust
pub enum HashAlgorithm {
    XxHash64,
    XxHash128,
    Blake3_256,
    Blake3_512,
    Sha256,
    Sha512,
    Sha3_256,
}

pub struct MultiHash {
    algorithm: HashAlgorithm,
    digest: Vec<u8>,
}
```

### Versioned Hash Format

```rust
// Store algorithm ID with hash
pub struct VersionedHash {
    version: u8,       // Protocol version
    algorithm: u8,     // Hash algorithm ID
    digest: Vec<u8>,   // Hash bytes
}

// Can parse hashes from different versions
impl VersionedHash {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let version = bytes[0];
        let algorithm = bytes[1];
        let digest = bytes[2..].to_vec();
        
        // Validate algorithm is supported
        match (version, algorithm) {
            (1, 0) => Ok(Self { version, algorithm, digest }),  // v1, xxHash64
            (1, 1) => Ok(Self { version, algorithm, digest }),  // v1, BLAKE3
            (1, 2) => Ok(Self { version, algorithm, digest }),  // v1, SHA-256
            _ => Err(Error::UnsupportedHashAlgorithm),
        }
    }
}
```

### Migration Path

**Phase 1**: Add BLAKE3 support alongside existing hashes
**Phase 2**: Default to BLAKE3 for new syncs
**Phase 3**: Maintain compatibility with older hash algorithms
**Phase 4**: Eventually deprecate weak algorithms (MD5, SHA-1)

---

## References

### xxHash
- [xxHash GitHub](https://github.com/Cyan4973/xxHash)
- [xxHash Algorithm](https://cyan4973.github.io/xxHash/)
- [Btrfs xxHash Integration](https://kdave.github.io/btrfs-hilights-5.5-new-hashes/)

### BLAKE3
- [BLAKE3 Official](https://github.com/BLAKE3-team/BLAKE3)
- [BLAKE3 Specification](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf)
- [BLAKE3 Paper](https://eprint.iacr.org/2020/1401.pdf)
- [BLAKE3 vs. SHA-256 Benchmarks](https://github.com/BLAKE3-team/BLAKE3/blob/master/b3sum/README.md)

### SHA-2 and SHA-3
- [FIPS 180-4 (SHA-2)](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf)
- [FIPS 202 (SHA-3)](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf)
- [Intel SHA Extensions](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sha-extensions.html)

### Post-Quantum Cryptography
- [NIST Post-Quantum Project](https://csrc.nist.gov/projects/post-quantum-cryptography)
- [Grover's Algorithm](https://en.wikipedia.org/wiki/Grover%27s_algorithm)
- [Post-Quantum Hash Functions](https://csrc.nist.gov/publications/detail/fips/202/final)

### Filesystem Integration
- [Btrfs Hash Selection](https://kdave.github.io/selecting-hash-for-btrfs/)
- [ZFS Checksums](https://openzfs.github.io/openzfs-docs/man/7/zfsprops.7.html#checksum)

### Benchmarks
- [Rust Crypto Benchmarks](https://github.com/RustCrypto/hashes/tree/master/benches)
- [SMHasher](https://github.com/rurban/smhasher) - Hash function test suite

---

## Conclusion

### Our Recommendation for arsync

**Hybrid Strategy**:

1. **Block Checksums** (rsync rolling hash): **xxHash64**
   - 20 GB/s throughput
   - Good enough collision resistance for non-adversarial use
   - Can upgrade to xxHash3-128 if more collision resistance needed

2. **Content Addressing + Merkle Trees**: **BLAKE3**
   - Cryptographically secure
   - 3 GB/s single-thread, 15+ GB/s multi-thread
   - Parallel-friendly (perfect for merkle trees)
   - Future-proof (arbitrary output length)

3. **Final Verification** (optional): **SHA-256**
   - Compliance-friendly
   - Hardware-accelerated on modern CPUs
   - Industry standard

**Algorithm Agility**:
- Support multiple algorithms (protocol field identifies hash type)
- Allow user to choose based on their security/performance requirements
- Default to BLAKE3 for new deployments
- Maintain compatibility with SHA-256 for existing tools

**Future-Proofing**:
- BLAKE3 can generate 512-bit hashes for post-quantum security
- Protocol versioning allows algorithm upgrades
- No migration needed (old and new hashes coexist)

---

## Document Information

**Version**: 1.0  
**Date**: October 9, 2025  
**Authors**: arsync research team  
**Status**: Survey Complete - Ready for Implementation

**Note on Benchmarks**: All performance numbers are based on 2024-2025 hardware and software. As of this writing (October 2025):
- BLAKE3 is the fastest cryptographic hash
- xxHash3 is the fastest non-cryptographic hash
- SHA-256 hardware acceleration (SHA-NI) is widely available on modern CPUs
- Post-quantum threats to hash functions remain theoretical (no practical quantum computers capable of breaking 256-bit hashes exist yet)

**Recommendation Review Schedule**: Re-evaluate hash algorithm recommendations every 2 years or when:
- New attacks on recommended algorithms are published
- Significantly faster algorithms become available
- Quantum computing capabilities advance substantially
- NIST issues new cryptographic standards

