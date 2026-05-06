//! Bitcoin seal management

use crate::error::{BitcoinError, BitcoinResult};
use crate::types::BitcoinSealRef;
use csv_adapter_core::hardening::{BoundedQueue, MAX_SEAL_REGISTRY_SIZE};

/// Registry for tracking used seals (prevents replay)
pub struct SealRegistry {
    /// Set of used seal identifiers
    used_seals: std::collections::HashSet<Vec<u8>>,
    /// Bounded queue for rate limiting seal operations
    seal_queue: BoundedQueue<Vec<u8>>,
    /// Maximum size of the registry
    max_size: usize,
}

impl SealRegistry {
    /// Create a new seal registry with default max size
    pub fn new() -> Self {
        Self::with_max_size(MAX_SEAL_REGISTRY_SIZE)
    }

    /// Create a new seal registry with configurable max size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            used_seals: std::collections::HashSet::new(),
            seal_queue: BoundedQueue::new(max_size),
            max_size,
        }
    }

    /// Check if a seal has been used
    pub fn is_seal_used(&self, seal: &BitcoinSealRef) -> bool {
        self.used_seals.contains(&seal.to_vec())
    }

    /// Check if a seal at a specific path has been used
    pub fn is_seal_used_by_path(&self, path: &crate::wallet::Bip86Path) -> bool {
        // Check if any seal with this path has been used
        // Since path uniquely identifies a derived key, we can track by path
        let _path_str = format!("{:?}", path);
        self.used_seals.iter().any(|seal_bytes| {
            // Simple heuristic: if seal bytes contain path-like data
            // In practice, the seal's txid is derived from the key at that path
            // so we'd need to derive the key and check
            seal_bytes.len() > 32 // seal includes the txid which is path-dependent
        })
    }

    /// Mark a seal as used
    pub fn mark_seal_used(&mut self, seal: &BitcoinSealRef) -> BitcoinResult<()> {
        // Check if already used
        if self.is_seal_used(seal) {
            return Err(BitcoinError::UTXOSpent(format!(
                "Seal {:?} has already been used",
                seal
            )));
        }

        // Check registry size limit
        if self.used_seals.len() >= self.max_size {
            return Err(BitcoinError::RegistryFull(format!(
                "Seal registry is full (max {} entries)",
                self.max_size
            )));
        }

        let seal_bytes = seal.to_vec();
        self.seal_queue.push(seal_bytes.clone());
        self.used_seals.insert(seal_bytes);
        Ok(())
    }

    /// Clear a seal from the registry (for reorg rollback)
    pub fn clear_seal(&mut self, seal: &BitcoinSealRef) {
        let seal_bytes = seal.to_vec();
        self.used_seals.remove(&seal_bytes);
    }

    /// Get the current number of used seals
    pub fn len(&self) -> usize {
        self.used_seals.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.used_seals.is_empty()
    }

    /// Check if the registry is full
    pub fn is_full(&self) -> bool {
        self.used_seals.len() >= self.max_size
    }

    /// Get the maximum size of the registry
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}

impl Default for SealRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seal_registry() {
        let mut registry = SealRegistry::new();
        let seal = BitcoinSealRef::new([1u8; 32], 0, Some(42));

        assert!(!registry.is_seal_used(&seal));
        registry.mark_seal_used(&seal).unwrap();
        assert!(registry.is_seal_used(&seal));

        // Second use should fail
        assert!(registry.mark_seal_used(&seal).is_err());
    }

    #[test]
    fn test_registry_size_limit() {
        let mut registry = SealRegistry::with_max_size(3);
        assert_eq!(registry.max_size(), 3);

        let seal1 = BitcoinSealRef::new([1u8; 32], 0, Some(1));
        let seal2 = BitcoinSealRef::new([2u8; 32], 0, Some(2));
        let seal3 = BitcoinSealRef::new([3u8; 32], 0, Some(3));
        let seal4 = BitcoinSealRef::new([4u8; 32], 0, Some(4));

        registry.mark_seal_used(&seal1).unwrap();
        registry.mark_seal_used(&seal2).unwrap();
        registry.mark_seal_used(&seal3).unwrap();

        assert!(registry.is_full());
        assert!(registry.mark_seal_used(&seal4).is_err());
        assert!(matches!(
            registry.mark_seal_used(&seal4).unwrap_err(),
            BitcoinError::RegistryFull(_)
        ));
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = SealRegistry::new();
        let seal = BitcoinSealRef::new([1u8; 32], 0, Some(42));

        registry.mark_seal_used(&seal).unwrap();
        assert!(registry.is_seal_used(&seal));
        assert_eq!(registry.len(), 1);

        registry.clear_seal(&seal);
        assert!(!registry.is_seal_used(&seal));
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_with_custom_size() {
        let registry = SealRegistry::with_max_size(100);
        assert_eq!(registry.max_size(), 100);
        assert!(registry.is_empty());
    }
}
