//! Keystore migration module (basic - no migration functionality).

/// Basic migration manager.
pub struct KeystoreMigration;

impl KeystoreMigration {
    /// Create a new basic instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeystoreMigration {
    fn default() -> Self {
        Self::new()
    }
}
