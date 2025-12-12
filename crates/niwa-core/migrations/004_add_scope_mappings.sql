-- Scope mappings for automatic scope detection from project paths
-- Pattern uses glob-like syntax (e.g., "projects/company-*", "work/*")

CREATE TABLE IF NOT EXISTS scope_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern TEXT NOT NULL UNIQUE,
    scope TEXT NOT NULL CHECK (scope IN ('personal', 'company', 'project')),
    priority INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Index for efficient pattern matching
CREATE INDEX IF NOT EXISTS idx_scope_mappings_priority ON scope_mappings(priority DESC);

-- Default mappings (can be customized by user)
-- Higher priority = checked first
INSERT OR IGNORE INTO scope_mappings (pattern, scope, priority) VALUES
    ('*', 'personal', 0);  -- Default fallback
