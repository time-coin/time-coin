-- Masternode Grant System Database Schema

-- Grant applications
CREATE TABLE IF NOT EXISTS grant_applications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT UNIQUE NOT NULL,
    verification_token TEXT UNIQUE NOT NULL,
    verified BOOLEAN DEFAULT 0,
    status TEXT DEFAULT 'pending', -- pending, verified, approved, active, forfeited, decommissioned
    grant_amount INTEGER DEFAULT 100000000000, -- 1000 TIME in satoshis
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    verified_at TIMESTAMP,
    activated_at TIMESTAMP,
    expires_at TIMESTAMP,
    masternode_address TEXT,
    public_key TEXT
);

-- Masternode activations
CREATE TABLE IF NOT EXISTS masternodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    grant_id INTEGER NOT NULL,
    address TEXT UNIQUE NOT NULL,
    public_key TEXT NOT NULL,
    locked_amount INTEGER NOT NULL,
    tier TEXT DEFAULT 'entry', -- entry = 1000 TIME
    status TEXT DEFAULT 'active', -- active, decommissioning, decommissioned
    activated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    decommission_requested_at TIMESTAMP,
    unlock_at TIMESTAMP, -- 3 months after decommission request
    ip_address TEXT,
    port INTEGER,
    FOREIGN KEY (grant_id) REFERENCES grant_applications(id)
);

-- Activity log
CREATE TABLE IF NOT EXISTS grant_activity_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    grant_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    details TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (grant_id) REFERENCES grant_applications(id)
);

-- Email verification tracking
CREATE TABLE IF NOT EXISTS email_verifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL,
    token TEXT NOT NULL,
    sent_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    verified_at TIMESTAMP
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_grant_email ON grant_applications(email);
CREATE INDEX IF NOT EXISTS idx_grant_status ON grant_applications(status);
CREATE INDEX IF NOT EXISTS idx_masternode_address ON masternodes(address);
CREATE INDEX IF NOT EXISTS idx_masternode_status ON masternodes(status);
