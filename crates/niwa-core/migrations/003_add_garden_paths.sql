-- Garden monitoring paths
CREATE TABLE IF NOT EXISTS garden_paths (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    preset_name TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    added_at INTEGER NOT NULL
);

-- Index for efficient querying
CREATE INDEX IF NOT EXISTS idx_garden_paths_enabled ON garden_paths(enabled);
