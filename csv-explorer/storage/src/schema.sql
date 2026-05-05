-- CSV Explorer database schema
-- Apply with: sqlite3 explorer.db < schema.sql

-- Rights table
CREATE TABLE IF NOT EXISTS rights (
    id TEXT PRIMARY KEY,
    chain TEXT NOT NULL,
    seal_ref TEXT NOT NULL,
    commitment TEXT NOT NULL,
    owner TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    created_tx TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT,
    transfer_count INTEGER DEFAULT 0,
    last_transfer_at TIMESTAMP,
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Transfers table
CREATE TABLE IF NOT EXISTS transfers (
    id TEXT PRIMARY KEY,
    right_id TEXT NOT NULL REFERENCES rights(id),
    from_chain TEXT NOT NULL,
    to_chain TEXT NOT NULL,
    from_owner TEXT NOT NULL,
    to_owner TEXT NOT NULL,
    lock_tx TEXT NOT NULL,
    mint_tx TEXT,
    proof_ref TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    duration_ms INTEGER,
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Seals table
CREATE TABLE IF NOT EXISTS seals (
    id TEXT PRIMARY KEY,
    chain TEXT NOT NULL,
    seal_type TEXT NOT NULL,
    seal_ref TEXT NOT NULL,
    right_id TEXT REFERENCES rights(id),
    status TEXT NOT NULL DEFAULT 'available',
    consumed_at TIMESTAMP,
    consumed_tx TEXT,
    block_height BIGINT NOT NULL,
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Contracts table
CREATE TABLE IF NOT EXISTS contracts (
    id TEXT PRIMARY KEY,
    chain TEXT NOT NULL,
    contract_type TEXT NOT NULL,
    address TEXT NOT NULL,
    deployed_tx TEXT NOT NULL,
    deployed_at TIMESTAMP NOT NULL,
    version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Sync progress table
CREATE TABLE IF NOT EXISTS sync_progress (
    chain TEXT PRIMARY KEY,
    latest_block BIGINT NOT NULL DEFAULT 0,
    latest_slot BIGINT,
    last_synced_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_rights_chain ON rights(chain);
CREATE INDEX IF NOT EXISTS idx_rights_owner ON rights(owner);
CREATE INDEX IF NOT EXISTS idx_rights_status ON rights(status);
CREATE INDEX IF NOT EXISTS idx_transfers_right_id ON transfers(right_id);
CREATE INDEX IF NOT EXISTS idx_transfers_status ON transfers(status);
CREATE INDEX IF NOT EXISTS idx_seals_chain ON seals(chain);
CREATE INDEX IF NOT EXISTS idx_seals_status ON seals(status);
CREATE INDEX IF NOT EXISTS idx_seals_right_id ON seals(right_id);
CREATE INDEX IF NOT EXISTS idx_seals_seal_ref ON seals(seal_ref);
CREATE INDEX IF NOT EXISTS idx_rights_seal_ref ON rights(seal_ref);
CREATE INDEX IF NOT EXISTS idx_contracts_chain ON contracts(chain);
CREATE INDEX IF NOT EXISTS idx_contracts_status ON contracts(status);
