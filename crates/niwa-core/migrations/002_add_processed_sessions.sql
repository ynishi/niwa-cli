-- Add processed_sessions table for Gardener (auto-crawler)

CREATE TABLE IF NOT EXISTS processed_sessions (
    file_path TEXT PRIMARY KEY,
    file_hash TEXT NOT NULL,
    expertise_id TEXT NOT NULL,
    processed_at INTEGER NOT NULL,
    FOREIGN KEY (expertise_id) REFERENCES expertises(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_processed_sessions_expertise ON processed_sessions(expertise_id);
CREATE INDEX IF NOT EXISTS idx_processed_sessions_processed_at ON processed_sessions(processed_at DESC);
