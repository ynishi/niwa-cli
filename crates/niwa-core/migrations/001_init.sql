-- Initial schema for NIWA Expertise Graph

-- Expertises table
CREATE TABLE IF NOT EXISTS expertises (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    scope TEXT NOT NULL CHECK(scope IN ('personal', 'company', 'project')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    data_json TEXT NOT NULL,
    description TEXT,
    UNIQUE(id, scope)
);

CREATE INDEX IF NOT EXISTS idx_expertises_scope ON expertises(scope);
CREATE INDEX IF NOT EXISTS idx_expertises_updated ON expertises(updated_at DESC);

-- Tags table
CREATE TABLE IF NOT EXISTS tags (
    expertise_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    FOREIGN KEY (expertise_id) REFERENCES expertises(id) ON DELETE CASCADE,
    PRIMARY KEY (expertise_id, tag)
);

CREATE INDEX IF NOT EXISTS idx_tags_tag ON tags(tag);

-- Relations table (dependency graph)
CREATE TABLE IF NOT EXISTS relations (
    from_id TEXT NOT NULL,
    to_id TEXT NOT NULL,
    relation_type TEXT NOT NULL CHECK(relation_type IN ('uses', 'extends', 'conflicts', 'requires')),
    metadata TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (from_id) REFERENCES expertises(id) ON DELETE CASCADE,
    FOREIGN KEY (to_id) REFERENCES expertises(id) ON DELETE CASCADE,
    PRIMARY KEY (from_id, to_id, relation_type)
);

CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(from_id);
CREATE INDEX IF NOT EXISTS idx_relations_to ON relations(to_id);
CREATE INDEX IF NOT EXISTS idx_relations_type ON relations(relation_type);

-- Versions table (version history)
CREATE TABLE IF NOT EXISTS versions (
    expertise_id TEXT NOT NULL,
    version TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    data_json TEXT NOT NULL,
    FOREIGN KEY (expertise_id) REFERENCES expertises(id) ON DELETE CASCADE,
    PRIMARY KEY (expertise_id, version)
);

CREATE INDEX IF NOT EXISTS idx_versions_expertise ON versions(expertise_id);
CREATE INDEX IF NOT EXISTS idx_versions_created ON versions(created_at DESC);

-- FTS5 for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS expertises_fts USING fts5(
    id UNINDEXED,
    description,
    tags
);

-- FTS5 triggers
CREATE TRIGGER IF NOT EXISTS expertises_ai AFTER INSERT ON expertises BEGIN
    INSERT INTO expertises_fts(id, description, tags)
    VALUES (
        new.id,
        new.description,
        (SELECT group_concat(tag, ' ') FROM tags WHERE expertise_id = new.id)
    );
END;

CREATE TRIGGER IF NOT EXISTS expertises_ad AFTER DELETE ON expertises BEGIN
    DELETE FROM expertises_fts WHERE id = old.id;
END;

CREATE TRIGGER IF NOT EXISTS expertises_au AFTER UPDATE ON expertises BEGIN
    UPDATE expertises_fts
    SET description = new.description,
        tags = (SELECT group_concat(tag, ' ') FROM tags WHERE expertise_id = new.id)
    WHERE id = new.id;
END;

-- Trigger to update FTS when tags change
CREATE TRIGGER IF NOT EXISTS tags_ai AFTER INSERT ON tags BEGIN
    UPDATE expertises_fts
    SET tags = (SELECT group_concat(tag, ' ') FROM tags WHERE expertise_id = new.expertise_id)
    WHERE id = new.expertise_id;
END;

CREATE TRIGGER IF NOT EXISTS tags_ad AFTER DELETE ON tags BEGIN
    UPDATE expertises_fts
    SET tags = (SELECT group_concat(tag, ' ') FROM tags WHERE expertise_id = old.expertise_id)
    WHERE id = old.expertise_id;
END;
