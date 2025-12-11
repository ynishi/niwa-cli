# NIWA: Expertise Graph Management System

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**NIWA** (庭 - Garden) は、LLM エージェントの知識を永続化・成長させる Expertise Graph 管理システムです。

---

## 🎯 Core Concept

### "The Intelligent Kernel"

NIWA は **「知能資産の永続化と成長」** を実現します。

```
┌─────────────────────────────────────────────────────────────┐
│  User Interface (CLI powered by sen-rs)                     │
│  ┌──────────────────┐         ┌──────────────────────────┐ │
│  │  Generation      │         │  Query & Management      │ │
│  │  (LLM Agent)     │         │  (Read-only CLI)         │ │
│  │                  │         │                          │ │
│  │  gen             │         │  list / show / search    │ │
│  │  improve         │         │  tags / graph / filter   │ │
│  │  merge           │         └──────────┬───────────────┘ │
│  └────────┬─────────┘                    │                 │
└───────────┼──────────────────────────────┼─────────────────┘
            │                              │
            ▼                              ▼
┌─────────────────────────────────────────────────────────────┐
│  niwa-core (SQLite Graph + Expertise CRUD)                  │
│  - llm-toolkit Expertise types                              │
│  - SQLite storage with FTS5                                 │
│  - Relations & versioning                                   │
└────────────────────────┬────────────────────────────────────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
    ┌────────┐     ┌─────────┐    ┌──────────┐
    │  MCP   │     │ Gemini  │    │  Cursor  │
    │ Server │     │ Export  │    │  Export  │
    └────────┘     └─────────┘    └──────────┘
```

### 設計原則

1. **ベンダーロックイン回避**: Expertise Graph は SQLite に保存され、任意のツール（Claude, Gemini, Custom Agent）に投影可能
2. **永続的な成長**: セッションログから学習し、Expertise を自動生成・改善
3. **型安全**: sen-rs フレームワークによるコンパイル時の型チェック
4. **LLM First**: llm-toolkit による SchemaBasedResponse で構造化データ取得

---

## ✨ Features

### Core Features

- ✅ **SQLite-based storage** with FTS5 full-text search
- ✅ **Expertise CRUD** operations with versioning
- ✅ **Dependency graph** (Relations: uses, extends, conflicts, requires)
- ✅ **Scope-based organization** (personal, company, project)
- ✅ **Tag-based filtering** and search
- ✅ **Type-safe API** with comprehensive error handling

### LLM-Powered Features

- ✅ **Auto-generation** from files or text input (`niwa gen`)
- ✅ **Expertise improvement** with LLM assistance (`niwa improve`)
- ✅ **Session log learning** - Auto-extract from Claude Code sessions (`niwa garden`)
- ✅ **Smart deduplication** via SHA256 file hashing

### CLI Experience

- ✅ **Interactive tutorial** (`niwa tutorial`)
- ✅ **Rich formatting** with tables and colored output
- ✅ **Graph visualization** (`niwa graph`)
- ✅ **Flexible filters** (--limit, --recent-days for garden)

### Future Plans

- 📋 **MCP Server** for direct Claude integration
- 📋 **Exporters** (Gemini CLI, Cursor rules)
- 💡 **Currently:** Use NIWA CLI directly - no export needed!

---

## 🚀 Quick Start

### Prerequisites

NIWA uses Claude Code CLI internally for LLM-powered features. Make sure you have:

- **Rust 1.70+** and **SQLite 3.35+**
- **Claude Code CLI** installed ([Installation Guide](https://docs.anthropic.com/en/docs/claude-code))
- **ANTHROPIC_API_KEY** environment variable set

```bash
# Set your API key (required for gen, improve, garden commands)
export ANTHROPIC_API_KEY=sk-ant-...
```

**Note:** NIWA internally uses `llm-toolkit`, which calls the `claude` command. Your environment variables (including `ANTHROPIC_API_KEY`) are passed through to Claude Code.

### Installation

#### From crates.io (Recommended)

```bash
cargo install niwa
```

#### From source

```bash
git clone https://github.com/ynishi/niwa-cli.git
cd niwa-cli
make install

# Or manually:
cargo build --release
cargo install --path crates/niwa
```

### Library Usage (niwa-core)

```rust
use niwa_core::{Database, Expertise, Scope, StorageOperations};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize database
    let db = Database::open_default().await?;

    // Create expertise
    let mut expertise = Expertise::new("rust-expert", "1.0.0");
    expertise.inner.description = Some("Expert in Rust programming".to_string());
    expertise.inner.tags = vec!["rust".to_string(), "programming".to_string()];
    expertise.metadata.scope = Scope::Personal;

    // Store
    db.storage().create(expertise).await?;

    // Query
    let results = db.query().search("rust", Default::default()).await?;
    println!("Found {} expertises", results.len());

    // List all
    let all = db.storage().list(Scope::Personal).await?;
    for exp in all {
        println!("- {} (v{})", exp.id(), exp.version());
    }

    Ok(())
}
```

---

## 📦 Architecture

### Module Structure

```
niwa-cli/
├── crates/
│   ├── niwa-core/          # Core library (✅ Complete)
│   │   ├── src/
│   │   │   ├── db.rs       # Database connection & migrations
│   │   │   ├── storage.rs  # CRUD operations
│   │   │   ├── query.rs    # Search & filtering
│   │   │   ├── graph.rs    # Relations & dependency graph
│   │   │   ├── types.rs    # Expertise types & Scope
│   │   │   └── error.rs    # Error types
│   │   └── migrations/
│   │       └── 001_init.sql
│   │
│   ├── niwa-generator/     # LLM-powered generation (🚧 In Progress)
│   ├── niwa-export/        # Exporters (Planned)
│   └── niwa/               # CLI binary (Planned)
│
├── ARCHITECTURE.md         # Detailed architecture
└── README.md               # This file
```

### Database Schema

```sql
-- Expertises with FTS5 full-text search
CREATE TABLE expertises (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    scope TEXT CHECK(scope IN ('personal', 'company', 'project')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    data_json TEXT NOT NULL,
    description TEXT
);

-- Tags
CREATE TABLE tags (
    expertise_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (expertise_id, tag)
);

-- Relations (dependency graph)
CREATE TABLE relations (
    from_id TEXT NOT NULL,
    to_id TEXT NOT NULL,
    relation_type TEXT CHECK(relation_type IN ('uses', 'extends', 'conflicts', 'requires')),
    metadata TEXT,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (from_id, to_id, relation_type)
);

-- Version history
CREATE TABLE versions (
    expertise_id TEXT NOT NULL,
    version TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    data_json TEXT NOT NULL,
    PRIMARY KEY (expertise_id, version)
);

-- FTS5 for full-text search
CREATE VIRTUAL TABLE expertises_fts USING fts5(
    id, description, tags
);
```

---

## 🔧 Development

### Prerequisites

- Rust 1.70+
- SQLite 3.35+ (for FTS5 support)

### Build

```bash
cargo build
```

### Test

```bash
cargo test -p niwa-core
```

### Check

```bash
cargo check --workspace
```

---

## 💡 Use Cases

### As a Skill Management System

NIWA can replace traditional "Skill" or "Expertise" management systems. Instead of exporting to external formats, **use NIWA CLI directly** to browse and query your knowledge.

#### Scenario 1: "What was the NIWA architecture again?"

```bash
# Search for architecture-related knowledge
$ niwa search "architecture"

# Show detailed information
$ niwa show niwa-architecture-detail

# Browse all knowledge
$ niwa list
```

#### Scenario 2: "Review this PR for NIWA Core"

```bash
# Find migration policy
$ niwa search "migration"
$ niwa show niwa-migration-policy

# Check related knowledge
$ niwa deps niwa-migration-policy

# Visualize knowledge graph
$ niwa graph
```

**Review checklist** (based on stored expertise):
- ✅ Migration uses `ALTER TABLE ADD COLUMN` only?
- ❌ No `DROP COLUMN` or `DROP TABLE`?
- ✅ Uses runtime `Migrator::new()` instead of `migrate!()` macro?

#### Scenario 3: Auto-learning from session logs

```bash
# Initialize garden monitoring (one-time setup)
$ niwa garden init claude-code

# Process recent sessions (last 5 days, max 10 files)
$ niwa garden --recent-days 5 --limit 10

# Check what was learned
$ niwa list
$ niwa tags
```

### As a Personal Knowledge Base

```bash
# Add quick tips
$ niwa gen --id rust-error-handling --text "Use Result<T,E> for recoverable errors, panic! for bugs"

# Extract from documentation
$ niwa gen --id project-architecture --file ARCHITECTURE.md

# Search when you need it
$ niwa search "error handling"

# Build knowledge graph
$ niwa link rust-error-handling --to rust-best-practices --relation-type extends
```

---

## 📚 Examples

### Storage Operations

```rust
use niwa_core::{Database, Expertise, Scope, StorageOperations};

let db = Database::open("~/.niwa/graph.db").await?;

// Create
let expertise = Expertise::new("rust-expert", "1.0.0");
db.storage().create(expertise).await?;

// Get
let exp = db.storage().get("rust-expert", Scope::Personal).await?;

// Update
if let Some(mut exp) = exp {
    exp.inner.version = "2.0.0".to_string();
    db.storage().update(exp).await?;
}

// Delete
db.storage().delete("rust-expert", Scope::Personal).await?;
```

### Search & Query

```rust
use niwa_core::{Database, SearchOptions, Scope};

let db = Database::open_default().await?;

// Full-text search
let options = SearchOptions::new().limit(10);
let results = db.query().search("error handling", options).await?;

// Filter by tags
let options = SearchOptions::new().scope(Scope::Personal);
let results = db.query()
    .filter_by_tags(vec!["rust".to_string()], options)
    .await?;

// List all tags
let tags = db.query().list_tags(None).await?;
for (tag, count) in tags {
    println!("{}: {}", tag, count);
}
```

### Graph Operations

```rust
use niwa_core::{Database, RelationType};

let db = Database::open_default().await?;

// Create relation
db.graph().create_relation(
    "rust-expert",
    "error-handling",
    RelationType::Uses,
    None
).await?;

// Get dependencies
let deps = db.graph().get_dependencies("rust-expert").await?;

// Get dependents
let dependents = db.graph().get_dependents("error-handling").await?;

// Build full graph
let graph = db.graph().build_graph().await?;
```

---

## 🗺️ Roadmap

### ✅ Phase 1: Core + SQLite (Complete)
- [x] Database schema & migrations
- [x] CRUD operations
- [x] Query engine (search, filter)
- [x] Graph operations (relations)
- [x] Comprehensive tests

### ✅ Phase 2: LLM Generation (Complete)
- [x] ExpertiseGenerator implementation
- [x] `niwa gen` command (--file, --text)
- [x] `niwa improve` command
- [x] LLM-powered extraction

### ✅ Phase 5: Gardener (Complete)
- [x] Session log parser
- [x] `niwa garden` command
- [x] claude-code / cursor presets
- [x] Auto-learning from session logs
- [x] Deduplication via file hashing

### 📋 Phase 3: MCP Server (Future)
- [ ] MCP protocol implementation
- [ ] Prompts / Resources / Tools
- [ ] Direct MCP integration

### 📋 Phase 4: Exporters (Future)
- [ ] Gemini CLI config generator
- [ ] Cursor rules generator
- [ ] **Note:** Currently use NIWA CLI directly instead of export

---

## 📖 Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) - Detailed architecture and design decisions
- [API Documentation](https://docs.rs/niwa-core) (Coming soon)

---

## 🤝 Contributing

Contributions are welcome! Please read [ARCHITECTURE.md](ARCHITECTURE.md) to understand the design first.

---

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

---

## 🙏 Acknowledgments

NIWA is built on top of:

- [sen-rs](https://github.com/ynishi/sen-rs) - CLI framework
- [llm-toolkit](https://github.com/ynishi/llm-toolkit) - LLM agent library
- [sqlx](https://github.com/launchbadge/sqlx) - Async SQL toolkit

Inspired by:
- [kanri-agent](https://github.com/ynishi/kanri) - Original prototype

---

**NIWA** (庭): Your personal garden of AI expertise, growing with every conversation.
