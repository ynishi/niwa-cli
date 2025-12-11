//! Interactive tutorial for NIWA CLI

use crate::state::AppState;
use clap::Parser;
use sen::{Args, CliResult, State};

/// Show interactive tutorial for NIWA CLI
#[derive(Parser, Debug)]
pub struct TutorialArgs {}

#[sen::handler]
pub async fn tutorial(
    _state: State<AppState>,
    Args(_args): Args<TutorialArgs>,
) -> CliResult<String> {
    let tutorial_text = r#"
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  NIWA Tutorial: Expertise Graph Management System
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Welcome to NIWA! This tutorial shows you how to use NIWA as a
Skill/Knowledge management system.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  📝 Use Case 1: Add Knowledge Manually
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Add a quick tip:
  $ niwa gen --id rust-error-handling \
      --text "Use Result<T,E> for recoverable errors"

Extract from a file:
  $ niwa gen --id project-arch --file ARCHITECTURE.md

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  🔍 Use Case 2: Search & Browse Knowledge
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Search by keyword:
  $ niwa search "error handling"

List all knowledge:
  $ niwa list

Show details:
  $ niwa show rust-error-handling

Browse by tags:
  $ niwa tags

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  🔗 Use Case 3: Build Knowledge Graph
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Create relations:
  $ niwa link rust-error-handling \
      --to rust-best-practices \
      --relation-type extends

View dependencies:
  $ niwa deps rust-error-handling

Visualize graph:
  $ niwa graph

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  🌱 Use Case 4: Auto-learn from Session Logs
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Initialize garden monitoring (one-time):
  $ niwa garden init claude-code

Process recent sessions:
  $ niwa garden --recent-days 5 --limit 10

Dry run to see what will be processed:
  $ niwa garden --recent-days 5 --limit 10 --dry-run

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  💼 Real-World Example: PR Review Workflow
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Scenario: "Review this PR for NIWA Core"

1. Find relevant policy:
   $ niwa search "migration policy"

2. Check the policy details:
   $ niwa show niwa-migration-policy

3. View related knowledge:
   $ niwa deps niwa-migration-policy

4. Review checklist (from stored expertise):
   ✅ Migration uses ALTER TABLE ADD COLUMN only?
   ❌ No DROP COLUMN or DROP TABLE?
   ✅ Uses runtime Migrator::new() instead of migrate!() macro?

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  🎯 Why NIWA Instead of Export?
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Traditional approach:
  Knowledge → Export → Load in tool → Limited search

NIWA approach:
  Knowledge → SQLite + FTS5 → Direct CLI → Full-text search
                                         → Graph navigation
                                         → Version history

Benefits:
  ✅ No export step needed
  ✅ Full-text search with FTS5
  ✅ Relationship graph navigation
  ✅ Version history tracking
  ✅ Direct CLI integration

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  🚀 Quick Start
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Try these commands now:

1. Add your first knowledge:
   $ niwa gen --id my-first-tip --text "Your expertise here"

2. List all knowledge:
   $ niwa list

3. Setup auto-learning:
   $ niwa garden init claude-code
   $ niwa garden --recent-days 1 --limit 3 --dry-run

For more details, see: README.md and ARCHITECTURE.md

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"#;

    Ok(tutorial_text.to_string())
}
