# NIWA: Expertise Graph Management System

**Version:** 0.1.0
**Status:** Design Phase
**Based on:** sen-rs v0.5.0, llm-toolkit v0.58.0

---

## 🎯 Core Concept: The Intelligent Kernel

NIWA は「知能資産の永続化と成長」を実現する Expertise Graph 管理システムです。

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

## 🏗️ Architecture Layers

### Layer 1: CLI Interface (sen-rs based)

**Framework:** sen-rs v0.5.0

sen-rs の Router API を使い、Axum スタイルのハンドラー定義：

```rust
use sen::{Router, State, CliResult};

#[tokio::main]
async fn main() {
    let state = AppState::new().await;

    let router = Router::new()
        // Generation commands (LLM-powered)
        .route("gen", handlers::gen::generate)
        .route("improve", handlers::gen::improve)
        .route("merge", handlers::gen::merge)

        // Query commands
        .route("list", handlers::query::list)
        .route("show", handlers::query::show)
        .route("search", handlers::query::search)
        .route("tags", handlers::query::tags)
        .route("graph", handlers::query::graph)

        // Relations
        .route("link", handlers::relations::link)
        .route("deps", handlers::relations::deps)

        // MCP Server
        .route("mcp", handlers::mcp::serve)

        // Export
        .route("export", handlers::export::export)

        .with_state(state)
        .with_agent_mode(); // JSON output for LLM integration

    let response = router.execute().await;

    if response.agent_mode {
        println!("{}", response.to_agent_json());
    } else {
        if !response.output.is_empty() {
            println!("{}", response.output);
        }
    }

    std::process::exit(response.exit_code);
}
```

### Layer 2: Core Business Logic

#### 2.1 Expertise Types (llm-toolkit)

llm-toolkit v0.58.0 から Expertise 型を使用（llm-toolkit-expertise は deprecated）：

```rust
use llm_toolkit::agent::expertise::{Expertise, WeightedFragment, KnowledgeFragment};
use schemars::JsonSchema;

// Expertise はそのまま SchemaBasedResponse で生成可能
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Expertise {
    pub id: String,
    pub version: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub content: Vec<WeightedFragment>,
}
```

#### 2.2 SQLite Schema

**Database:** `~/.niwa/graph.db`

```sql
-- expertises テーブル
CREATE TABLE expertises (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    scope TEXT NOT NULL CHECK(scope IN ('personal', 'company', 'project')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    data_json TEXT NOT NULL,  -- Full Expertise object as JSON
    description TEXT,          -- Cached for search
    UNIQUE(id, scope)
);

CREATE INDEX idx_expertises_scope ON expertises(scope);
CREATE INDEX idx_expertises_updated ON expertises(updated_at DESC);

-- tags テーブル
CREATE TABLE tags (
    expertise_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    FOREIGN KEY (expertise_id) REFERENCES expertises(id) ON DELETE CASCADE,
    PRIMARY KEY (expertise_id, tag)
);

CREATE INDEX idx_tags_tag ON tags(tag);

-- relations テーブル（依存関係グラフ）
CREATE TABLE relations (
    from_id TEXT NOT NULL,
    to_id TEXT NOT NULL,
    relation_type TEXT NOT NULL CHECK(relation_type IN ('uses', 'extends', 'conflicts', 'requires')),
    metadata TEXT,  -- Optional JSON metadata
    created_at INTEGER NOT NULL,
    FOREIGN KEY (from_id) REFERENCES expertises(id) ON DELETE CASCADE,
    FOREIGN KEY (to_id) REFERENCES expertises(id) ON DELETE CASCADE,
    PRIMARY KEY (from_id, to_id, relation_type)
);

-- versions テーブル（バージョン履歴）
CREATE TABLE versions (
    expertise_id TEXT NOT NULL,
    version TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    data_json TEXT NOT NULL,
    FOREIGN KEY (expertise_id) REFERENCES expertises(id) ON DELETE CASCADE,
    PRIMARY KEY (expertise_id, version)
);

-- FTS5 for full-text search
CREATE VIRTUAL TABLE expertises_fts USING fts5(
    id UNINDEXED,
    description,
    tags,
    content=expertises,
    content_rowid=rowid
);

-- FTS5 triggers
CREATE TRIGGER expertises_ai AFTER INSERT ON expertises BEGIN
    INSERT INTO expertises_fts(rowid, id, description, tags)
    VALUES (new.rowid, new.id, new.description,
            (SELECT group_concat(tag, ' ') FROM tags WHERE expertise_id = new.id));
END;

CREATE TRIGGER expertises_ad AFTER DELETE ON expertises BEGIN
    DELETE FROM expertises_fts WHERE rowid = old.rowid;
END;

CREATE TRIGGER expertises_au AFTER UPDATE ON expertises BEGIN
    UPDATE expertises_fts SET description = new.description,
                               tags = (SELECT group_concat(tag, ' ') FROM tags WHERE expertise_id = new.id)
    WHERE rowid = new.rowid;
END;
```

#### 2.2.1 Migration Policy

**原則: 破壊的変更を避ける**

NIWA は CLI/Desktop App であり、ユーザーのローカルデータを保護することが最優先です。

**許可される変更:**
- ✅ テーブル追加
- ✅ カラム追加（`ALTER TABLE ADD COLUMN`）
- ✅ インデックス追加
- ✅ トリガー追加

**禁止される変更:**
- ❌ カラム削除（代わりに deprecated として残す）
- ❌ テーブル削除（代わりに使用を停止）
- ❌ データ型の変更（互換性がない場合）
- ❌ データ損失を伴う変更

**Migration の実装:**
- 実行時ロード: `sqlx::migrate::Migrator::new()` を使用（コンパイル時 `migrate!()` マクロは使わない）
- 理由: CLI では migration ファイルの追加がバイナリリビルド後に行われることがあるため
- 場所: `crates/niwa-core/migrations/*.sql`

#### 2.3 Application State

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub generator: Arc<ExpertiseGenerator>,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let db_path = Self::db_path()?;
        let db = SqlitePool::connect(&db_path).await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&db).await?;

        let generator = Arc::new(ExpertiseGenerator::new()?);

        Ok(Self { db: Arc::new(db), generator })
    }

    fn db_path() -> Result<String> {
        let home = std::env::var("HOME")?;
        let niwa_dir = PathBuf::from(home).join(".niwa");
        std::fs::create_dir_all(&niwa_dir)?;
        Ok(niwa_dir.join("graph.db").display().to_string())
    }
}
```

### Layer 3: LLM-Powered Generation

#### 3.1 ExpertiseGenerator

llm-toolkit の Agent を使用：

```rust
use llm_toolkit::agent::{Agent, AgentConfig};

pub struct ExpertiseGenerator {
    agent: Agent,
}

impl ExpertiseGenerator {
    pub fn new() -> Result<Self> {
        let config = AgentConfig::builder()
            .model("claude-sonnet-4-5")
            .build()?;

        let agent = Agent::new(config)?;
        Ok(Self { agent })
    }

    /// Generate from conversation log
    pub async fn generate_from_log(
        &self,
        log_content: &str,
        id: &str,
        scope: Scope,
    ) -> Result<Expertise> {
        let prompt = format!(
            r#"Analyze the following conversation log and extract reusable knowledge as an Expertise profile.

# Conversation Log
```
{}
```

# Task
Create an Expertise profile with:
- ID: {}
- Version: "1.0.0"
- Appropriate tags
- Knowledge fragments (Logic, Guideline, QualityStandard, Text)
- Priorities (Critical > High > Normal > Low)

Generate the complete Expertise object.
"#,
            log_content, id
        );

        // SchemaBasedResponse で構造化データ取得
        let mut expertise: Expertise = self.agent
            .generate_schema_based(&prompt)
            .await?;

        expertise.scope = scope;
        Ok(expertise)
    }
}
```

### Layer 4: MCP Server Integration

sen-rs v0.6 の MCP フィーチャーを活用（現在は準備中）：

```rust
// handlers/mcp.rs
use sen::{State, CliResult};

pub async fn serve(state: State<AppState>) -> CliResult<()> {
    // MCP Server を起動
    // - GetPrompt: Expertise を System Prompt として提供
    // - ListResources: Expertise の詳細をリソースとして提供
    // - ListTools: (Phase 2) Capabilities をツールとして提供

    todo!("MCP Server implementation using sen-rs mcp feature")
}
```

---

## 📦 Module Structure

```
niwa-cli/
├── Cargo.toml              # Workspace root
├── ARCHITECTURE.md         # This file
├── README.md
└── crates/
    ├── niwa-core/          # Core domain logic
    │   ├── Cargo.toml
    │   ├── migrations/     # SQLx migrations
    │   │   └── 001_init.sql
    │   └── src/
    │       ├── lib.rs
    │       ├── storage.rs  # SQLite CRUD
    │       ├── query.rs    # Search & filter
    │       ├── graph.rs    # Relations
    │       └── types.rs    # Re-export from llm-toolkit
    │
    ├── niwa-generator/     # LLM-powered generation
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── generator.rs
    │       └── prompts.rs
    │
    ├── niwa-mcp/           # MCP Server (Phase 2)
    │   └── Cargo.toml
    │
    ├── niwa-export/        # Exporters
    │   └── src/
    │       ├── gemini.rs
    │       └── cursor.rs
    │
    └── niwa/               # Main CLI binary
        ├── Cargo.toml
        └── src/
            ├── main.rs     # Router setup (< 100 lines)
            ├── state.rs    # AppState
            └── handlers/
                ├── mod.rs
                ├── gen.rs      # gen, improve, merge
                ├── query.rs    # list, show, search, tags, graph
                ├── relations.rs # link, deps
                ├── mcp.rs      # mcp
                └── export.rs   # export
```

---

## 🎨 CLI Commands

### Generation Commands (LLM-powered)

```bash
# Generate from log file
niwa gen --file session1.log --id rust-expert --scope personal

# Generate interactively
niwa gen --interactive

# Improve existing expertise
niwa improve rust-expert --instruction "Add error handling best practices"

# Merge multiple expertises
niwa merge rust-expert error-handling --output rust-complete
```

### Query Commands

```bash
# List all expertises
niwa list
niwa list --scope personal
niwa list --tag rust

# Show details
niwa show rust-expert

# Full-text search
niwa search "error handling"

# List tags
niwa tags

# Show dependency graph (ASCII art)
niwa graph rust-expert
```

### Relations Commands

```bash
# Create relation
niwa link rust-expert --to error-handling --type uses

# Show dependencies
niwa deps rust-expert
```

### MCP Server

```bash
# Start MCP server for Claude Code
niwa mcp

# Add to Claude Desktop config:
# ~/.config/claude/config.json
{
  "mcpServers": {
    "niwa": {
      "command": "niwa",
      "args": ["mcp"]
    }
  }
}
```

### Export

```bash
# Export to Gemini CLI config
niwa export gemini --out ~/.gemini/

# Export to Cursor rules
niwa export cursor --out .cursorrules

# Export single expertise as YAML
niwa export yaml rust-expert > rust-expert.yaml
```

---

## 🔧 Technology Stack

### Core Dependencies

```toml
[dependencies]
# Framework
sen = { version = "0.5", features = ["mcp", "sensors"] }

# LLM
llm-toolkit = "0.58.0"

# Database
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-native-tls", "migrate"] }

# CLI
clap = { version = "4.4", features = ["derive"] }
comfy-table = "7.1"
dialoguer = "0.11"
indicatif = "0.17"

# Async
tokio = { version = "1.35", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
schemars = "0.8"

# Error handling
anyhow = "1.0"
thiserror = "1.0"
```

---

## 🚀 Implementation Phases

### Phase 0: Project Setup ✅
- [x] Workspace structure
- [x] Dependencies

### Phase 1: Core + SQLite
- [ ] SQLite schema & migrations
- [ ] CRUD operations (storage.rs)
- [ ] Query engine (query.rs, graph.rs)
- [ ] Basic CLI (list, show)

### Phase 2: LLM Generation
- [ ] ExpertiseGenerator implementation
- [ ] `niwa gen` command
- [ ] `niwa improve` command
- [ ] Interactive generation

### Phase 3: MCP Server
- [ ] MCP protocol implementation (using sen-rs mcp feature)
- [ ] Claude Code integration
- [ ] Prompts / Resources / Tools

### Phase 4: Exporters
- [ ] Gemini CLI config generator
- [ ] Cursor rules generator

### Phase 5: Gardener (Auto-learning)
- [ ] Session log parser
- [ ] `.claude` directory crawler
- [ ] Pattern extraction
- [ ] Auto-growth logic

---

## 🎯 Success Criteria

1. **Zero Configuration**: `niwa mcp` で即座に Claude Code と連携
2. **LLM First**: 手作業での YAML 編集が不要
3. **Fast Query**: SQLite FTS5 で高速な全文検索
4. **Type Safe**: sen-rs による型安全なハンドラー
5. **Vendor Free**: Graph は SQLite に保存され、任意のツールに投影可能

---

## 📚 References

- [sen-rs](https://github.com/ynishi/sen-rs) - CLI framework
- [llm-toolkit](https://github.com/ynishi/llm-toolkit) - LLM agent library
- [kanri-agent](~/projects/kanri) - Original prototype (deprecated)
