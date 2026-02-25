# ez

**Stacked PRs for GitHub.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/rohoswagger/ez-stack/actions/workflows/ci.yml/badge.svg)](https://github.com/rohoswagger/ez-stack/actions/workflows/ci.yml)

---

`ez` is a fast, lightweight CLI for managing stacked pull requests on GitHub. It shells out to `git` and `gh` so there's nothing magical happening under the hood — just the tools you already know, orchestrated intelligently.

## Why stacked PRs?

Large pull requests are hard to review. Stacked PRs let you break work into a chain of small, focused branches where each branch builds on the one below it:

```
main
 └── feat/auth-types        ← PR #1 (data models)
      └── feat/auth-api     ← PR #2 (API routes, depends on #1)
           └── feat/auth-ui ← PR #3 (frontend, depends on #2)
```

Reviewers see small diffs. You keep working without waiting. When PR #1 merges, `ez` rebases the rest of the stack automatically.

The problem is that `git` doesn't know about stacks. Rebasing, reordering, and keeping GitHub PRs pointed at the right base branch is tedious and error-prone. `ez` handles all of that for you.

## Quick start

```bash
# Install
cargo install ez-stack

# Initialize in any git repo
cd your-repo
ez init

# Start building a stack
ez create feat/parse-config
# ... make changes ...
ez commit -m "add config parser"

ez create feat/use-config
# ... make changes ...
ez commit -m "wire config into app"

# Push and open PRs for the whole stack
ez push
ez submit
```

That's it. Two PRs, correctly chained, with GitHub base branches set automatically.

## Commands

### Stack creation & editing

| Command | Description |
|---------|-------------|
| `ez init` | Initialize `ez` in the current repository |
| `ez create <name>` | Create a new branch on top of the current stack |
| `ez commit [-m <msg>]` | Commit staged changes to the current branch |
| `ez amend` | Amend the last commit on the current branch |
| `ez delete [<name>]` | Delete a branch from the stack and restack |
| `ez move <--up\|--down>` | Reorder the current branch within the stack |

### Syncing & rebasing

| Command | Description |
|---------|-------------|
| `ez sync` | Fetch `main`, rebase the entire stack, clean up merged branches |
| `ez restack` | Rebase each branch in the stack onto its parent |
| `ez push` | Force-push all branches in the stack to the remote |

### Navigation

| Command | Description |
|---------|-------------|
| `ez up` | Check out the branch above the current one |
| `ez down` | Check out the branch below the current one |
| `ez top` | Check out the top of the stack |
| `ez bottom` | Check out the bottom of the stack |
| `ez checkout <name>` | Check out any branch in the stack by name |

### GitHub integration

| Command | Description |
|---------|-------------|
| `ez submit` | Create or update GitHub PRs for all branches in the stack |
| `ez merge` | Merge the bottom PR and restack |

### Inspection

| Command | Description |
|---------|-------------|
| `ez log` | Show the full stack with branch names, commit counts, and PR status |
| `ez status` | Show the current branch and its position in the stack |

## Example workflow

Here's a complete session building a three-branch stack:

```bash
# 1. Start from main
git checkout main && git pull
ez init

# 2. Create the first branch in the stack
ez create feat/auth-types
cat > src/auth/types.rs << 'EOF'
pub struct User { pub id: u64, pub email: String }
pub struct Session { pub token: String, pub user_id: u64 }
EOF
ez commit -m "define User and Session types"

# 3. Stack a second branch on top
ez create feat/auth-api
cat > src/auth/api.rs << 'EOF'
pub fn login(email: &str) -> Session { /* ... */ }
pub fn logout(session: &Session) { /* ... */ }
EOF
ez commit -m "add login/logout API"

# 4. Stack a third branch on top
ez create feat/auth-middleware
cat > src/middleware/auth.rs << 'EOF'
pub fn require_auth(req: &Request) -> Result<User, AuthError> { /* ... */ }
EOF
ez commit -m "add auth middleware"

# 5. See the full stack
ez log
#   main
#   ├── feat/auth-types        (1 commit)
#   │   ├── feat/auth-api      (1 commit)
#   │   │   ├── feat/auth-middleware (1 commit)  ← you are here

# 6. Push everything and open PRs
ez push
ez submit
# Creates 3 PRs:
#   feat/auth-types        → main
#   feat/auth-api          → feat/auth-types
#   feat/auth-middleware    → feat/auth-api

# 7. After feat/auth-types is reviewed and merged on GitHub:
ez sync
# Fetches main (which now includes auth-types),
# rebases auth-api onto main, rebases auth-middleware onto auth-api,
# deletes the merged feat/auth-types branch,
# and updates PR base branches on GitHub.
```

## How it works

`ez` is intentionally simple in its architecture:

- **No custom git internals.** Every git operation is a call to the `git` CLI. Every GitHub operation goes through `gh`. You can always see exactly what happened by reading your git log.
- **Stack metadata** is stored in `.git/ez/stack.json` — a single JSON file tracking branch order, parent relationships, and associated PR numbers. It's local to your repo and ignored by git.
- **Restacking** uses `git rebase --onto` to move each branch in the stack onto its updated parent. This is the same operation you'd do by hand; `ez` just does it for every branch in the right order.
- **PR management** calls `gh pr create` and `gh pr edit` to set base branches so GitHub shows the correct, minimal diff for each PR in the stack.

### Stack metadata format

```json
{
  "version": 1,
  "trunk": "main",
  "branches": [
    { "name": "feat/auth-types", "parent": "main", "pr": 101 },
    { "name": "feat/auth-api", "parent": "feat/auth-types", "pr": 102 },
    { "name": "feat/auth-middleware", "parent": "feat/auth-api", "pr": null }
  ]
}
```

## Prerequisites

- **git** 2.38+
- **gh** (GitHub CLI), authenticated via `gh auth login`
- A GitHub repository with push access

## Installation

### From crates.io

```bash
cargo install ez-stack
```

### From source

```bash
git clone https://github.com/rohoswagger/ez-stack.git
cd ez-stack
cargo install --path .
```

### Install script (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/rohoswagger/ez-stack/main/install.sh | bash
```

To install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/rohoswagger/ez-stack/main/install.sh | bash -s -- v0.1.0
```

### GitHub releases

Pre-built binaries for Linux and macOS are available on the [Releases](https://github.com/rohoswagger/ez-stack/releases) page.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and how to submit changes.

## License

MIT. See [LICENSE](LICENSE) for details.
