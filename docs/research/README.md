# Research Documentation

This directory contains research documents, algorithm analysis, and technical deep-dives for the arsync project.

---

## Core Research Documents

### [research.md](research.md)
**Original io_uring research document**
- io_uring capabilities and operations inventory
- Rust library analysis (compio, rio, tokio-uring)
- Performance optimization strategies
- Semaphore design and queueing architecture
- **Status**: Foundation research (completed)

### [REMOTE_SYNC_RESEARCH.md](REMOTE_SYNC_RESEARCH.md) ⭐
**Main research initiative for remote synchronization**
- rsync protocol analysis and wire compatibility
- Merkle tree applications to file synchronization
- Parallel hashing strategies
- Bandwidth-delay product adaptation
- Directory-level merkle trees
- Modern transport layers (SSH, QUIC, HTTP/3, VPN mesh networks)
- **Status**: Active research (October 2025)
- **Phases**: 7 research phases outlined

---

## Protocol Design Documents

### [SSH_QUIC_HYBRID_PROTOCOL.md](SSH_QUIC_HYBRID_PROTOCOL.md)
**Control plane + data plane separation design**
- SSH for authentication and control channel
- QUIC for high-performance parallel data transfer
- Pre-shared key exchange via SSH
- Protocol specification with message formats
- Security model and threat analysis
- **Status**: Design proposal ready for implementation

---

## Algorithm Surveys

### [HASH_ALGORITHM_SURVEY.md](HASH_ALGORITHM_SURVEY.md)
**Comprehensive hash algorithm analysis**
- Non-cryptographic hashes (xxHash, CityHash, HighwayHash)
- Cryptographic hashes (BLAKE3, SHA-256, SHA-3)
- Performance benchmarks and comparisons
- Post-quantum considerations
- Use case recommendations
- **Date**: October 9, 2025
- **Status**: Survey complete

---

## Technical Deep-Dives

### [NVME_ARCHITECTURE.md](NVME_ARCHITECTURE.md)
**Why io_uring is necessary for modern storage**
- NVMe command queue architecture (64K queues × 64K commands)
- Why traditional blocking I/O underutilizes NVMe
- io_uring's queue-pair model matching NVMe design
- Performance implications and measurements

### [FADVISE_VS_O_DIRECT.md](FADVISE_VS_O_DIRECT.md)
**Why fadvise is superior to O_DIRECT**
- Linus Torvalds' "deranged monkey" critique of O_DIRECT
- Alignment requirements and synchronous overhead
- fadvise benefits and kernel optimizations
- Performance comparison

### [RSYNC_COMPARISON.md](RSYNC_COMPARISON.md)
**Detailed comparison: rsync vs arsync**
- Feature-by-feature comparison
- Performance characteristics
- Security advantages (TOCTOU-free operations)
- Six key innovations in arsync

### [INDUSTRY_STANDARDS.md](INDUSTRY_STANDARDS.md)
**Industry standards and best practices**
- File synchronization standards
- Enterprise backup requirements
- Compliance considerations

### [LINUX_KERNEL_CONTRIBUTIONS.md](LINUX_KERNEL_CONTRIBUTIONS.md)
**Linux kernel development insights**
- io_uring development process
- Kernel contribution guidelines
- Upstream collaboration strategies

### [POWER_MEASUREMENT.md](POWER_MEASUREMENT.md)
**Power consumption measurement methodology**
- Energy efficiency benchmarking
- Power measurement tools and techniques
- Green computing considerations

---

## Document Categories

**Active Research** (ongoing):
- REMOTE_SYNC_RESEARCH.md (7 phases, some pending)
- HASH_ALGORITHM_SURVEY.md (will need periodic updates)

**Design Proposals** (implementation-ready):
- SSH_QUIC_HYBRID_PROTOCOL.md

**Technical Background** (reference material):
- NVME_ARCHITECTURE.md
- FADVISE_VS_O_DIRECT.md
- RSYNC_COMPARISON.md
- research.md (original io_uring research)

---

## Reading Order

**New to the project?** Start here:

1. **[research.md](research.md)** - Understand io_uring and why arsync exists
2. **[NVME_ARCHITECTURE.md](NVME_ARCHITECTURE.md)** - Why modern storage needs async I/O
3. **[RSYNC_COMPARISON.md](RSYNC_COMPARISON.md)** - How arsync improves on rsync
4. **[REMOTE_SYNC_RESEARCH.md](REMOTE_SYNC_RESEARCH.md)** - Future direction (remote sync)

**Implementing remote sync?** Read these:

1. **[REMOTE_SYNC_RESEARCH.md](REMOTE_SYNC_RESEARCH.md)** - Overall research plan
2. **[SSH_QUIC_HYBRID_PROTOCOL.md](SSH_QUIC_HYBRID_PROTOCOL.md)** - Protocol design
3. **[HASH_ALGORITHM_SURVEY.md](HASH_ALGORITHM_SURVEY.md)** - Algorithm selection

**Deep technical background:**

- **[FADVISE_VS_O_DIRECT.md](FADVISE_VS_O_DIRECT.md)** - I/O optimization
- **[NVME_ARCHITECTURE.md](NVME_ARCHITECTURE.md)** - Storage architecture

---

## Research Status Summary

| Area | Status | Next Steps |
|------|--------|------------|
| **Local file sync** | ✅ Complete | In production use |
| **io_uring optimization** | ✅ Complete | Continuous improvement |
| **Metadata preservation** | ✅ Complete | Well-tested |
| **Remote sync protocol** | 🔬 Research | Phase 1: rsync wire protocol analysis |
| **Merkle tree sync** | 📝 Design | Prototype needed |
| **QUIC transport** | 📝 Design | Implementation pending |
| **Hash algorithm** | ✅ Survey complete | Implementation: BLAKE3 + xxHash |

**Legend**: ✅ Complete | 📝 Design ready | 🔬 Active research | ⏳ Future work

---

## Contributing to Research

Research documents are living documents. To contribute:

1. **Add findings**: Update with new benchmark data, algorithm analysis, or protocol details
2. **Challenge assumptions**: If research conclusions seem incorrect, document why
3. **Add references**: Link to papers, RFCs, blog posts, or implementations
4. **Update status**: Mark completed research phases, update benchmarks for new hardware

Research documents should be:
- **Well-sourced**: Include links to papers, RFCs, implementations
- **Dated**: Note when research was conducted (hardware/software context matters)
- **Versioned**: Track major changes to research conclusions
- **Honest**: Document both successes and dead-ends

---

## Related Documentation

**User/Developer docs** (in `docs/`):
- `DEVELOPER.md` - Developer setup and workflow
- `IMPLEMENTATION_PLAN.md` - Project phases and deliverables
- `TESTING_STRATEGY.md` - Test approach and coverage
- `BENCHMARKING_PLAN.md` - Performance measurement strategy

**See parent directory** for operational documentation.

---

**Last Updated**: October 9, 2025

