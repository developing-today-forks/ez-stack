# Stack Links in PR Bodies — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When `ez push` creates a new PR (or updates one with `--body`), append a numbered list of upstream ancestor PR links to the body; also add `ez push --stack` as a shorthand for `ez submit`.

**Architecture:** Extract all body-generation logic into a new pure module `src/stack_body.rs` (no I/O, fully unit-testable). `push_or_update_pr` in `push.rs` calls it to build the final body and passes `body_explicitly_set` to decide whether to update existing PR bodies. `--stack` flag in `push::run` delegates to `submit::run`.

**Tech Stack:** Rust 1.85, clap 4 (derive), anyhow, existing `git`/`github`/`stack` modules

---

## Chunk 1: Pure stack-body module with unit tests

### Task 1: Create `src/stack_body.rs` with failing tests

**Files:**
- Create: `src/stack_body.rs`

The module exposes two public functions. Write tests first — they will fail because the functions don't exist yet.

- [ ] **Step 1: Create `src/stack_body.rs` with the struct, function stubs, and tests**

```rust
/// One upstream ancestor branch and its PR info (if any).
pub struct AncestorPr {
    pub branch: String,
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>, // pre-resolved, e.g. "https://github.com/org/repo/pull/42"
}

/// Returns the markdown stack section, or None if no ancestors have a PR number.
/// Ancestors are expected in trunk-closest-first order.
/// Only ancestors with a pr_number are listed; ancestors without one are skipped.
pub fn build_stack_section(ancestors: &[AncestorPr]) -> Option<String> {
    todo!()
}

/// Returns the full PR body: user_body, then (if any ancestors have PRs) a
/// separator and the stack section appended.
pub fn build_stack_body(ancestors: &[AncestorPr], user_body: &str) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anc(branch: &str, number: Option<u64>, url: Option<&str>) -> AncestorPr {
        AncestorPr {
            branch: branch.to_string(),
            pr_number: number,
            pr_url: url.map(|s| s.to_string()),
        }
    }

    // --- build_stack_section ---

    #[test]
    fn section_empty_ancestors_returns_none() {
        assert!(build_stack_section(&[]).is_none());
    }

    #[test]
    fn section_ancestor_without_pr_returns_none() {
        let ancestors = vec![anc("feat/a", None, None)];
        assert!(build_stack_section(&ancestors).is_none());
    }

    #[test]
    fn section_one_ancestor_with_url() {
        let ancestors = vec![anc("feat/a", Some(101), Some("https://github.com/org/repo/pull/101"))];
        let section = build_stack_section(&ancestors).unwrap();
        assert_eq!(
            section,
            "**Stack:**\n1. [feat/a #101](https://github.com/org/repo/pull/101)"
        );
    }

    #[test]
    fn section_one_ancestor_without_url() {
        let ancestors = vec![anc("feat/a", Some(101), None)];
        let section = build_stack_section(&ancestors).unwrap();
        assert_eq!(section, "**Stack:**\n1. feat/a #101");
    }

    #[test]
    fn section_skips_ancestors_without_pr_number() {
        let ancestors = vec![
            anc("feat/a", Some(101), Some("https://github.com/org/repo/pull/101")),
            anc("feat/b", None, None), // no PR yet — skipped
            anc("feat/c", Some(103), Some("https://github.com/org/repo/pull/103")),
        ];
        let section = build_stack_section(&ancestors).unwrap();
        assert_eq!(
            section,
            "**Stack:**\n1. [feat/a #101](https://github.com/org/repo/pull/101)\n2. [feat/c #103](https://github.com/org/repo/pull/103)"
        );
    }

    #[test]
    fn section_numbers_are_sequential_for_linked_only() {
        let ancestors = vec![
            anc("feat/a", None, None),
            anc("feat/b", Some(102), Some("https://github.com/org/repo/pull/102")),
            anc("feat/c", Some(103), Some("https://github.com/org/repo/pull/103")),
        ];
        let section = build_stack_section(&ancestors).unwrap();
        // Numbering starts at 1 for linked ancestors only
        assert!(section.contains("1. [feat/b"));
        assert!(section.contains("2. [feat/c"));
    }

    // --- build_stack_body ---

    #[test]
    fn body_no_ancestors_returns_user_body_unchanged() {
        let result = build_stack_body(&[], "My PR description.");
        assert_eq!(result, "My PR description.");
    }

    #[test]
    fn body_ancestors_without_prs_returns_user_body_unchanged() {
        let ancestors = vec![anc("feat/a", None, None)];
        let result = build_stack_body(&ancestors, "My PR description.");
        assert_eq!(result, "My PR description.");
    }

    #[test]
    fn body_appends_section_after_separator() {
        let ancestors = vec![anc("feat/a", Some(101), Some("https://github.com/org/repo/pull/101"))];
        let result = build_stack_body(&ancestors, "My PR description.");
        assert_eq!(
            result,
            "My PR description.\n\n---\n\n**Stack:**\n1. [feat/a #101](https://github.com/org/repo/pull/101)"
        );
    }

    #[test]
    fn body_preserves_user_body_above_section() {
        let ancestors = vec![anc("feat/a", Some(101), None)];
        let body = "This PR adds X.\n\nMore details here.";
        let result = build_stack_body(&ancestors, body);
        assert!(result.starts_with("This PR adds X.\n\nMore details here."));
        assert!(result.contains("\n\n---\n\n**Stack:**"));
    }
}
```

- [ ] **Step 2: Register the module in `src/cmd/mod.rs`**

Add this line to `src/cmd/mod.rs`:
```rust
pub mod stack_body;
```

Wait — `stack_body` is not a cmd, it's a top-level src module. Register it in `src/main.rs` instead by adding:
```rust
mod stack_body;
```

near the other `mod` declarations at the top of `src/main.rs`.

- [ ] **Step 3: Verify tests fail**

```bash
cd /Users/rohoswagger/Documents/code/ez-stack
cargo test stack_body 2>&1
```

Expected: compilation error or `not yet implemented` panics — confirming the stubs need implementing.

---

### Task 2: Implement `build_stack_section` and `build_stack_body`

**Files:**
- Modify: `src/stack_body.rs`

- [ ] **Step 1: Implement `build_stack_section`**

Replace the `todo!()` with:

```rust
pub fn build_stack_section(ancestors: &[AncestorPr]) -> Option<String> {
    let linked: Vec<String> = ancestors
        .iter()
        .filter(|a| a.pr_number.is_some())
        .enumerate()
        .map(|(i, a)| {
            let num = a.pr_number.unwrap();
            match &a.pr_url {
                Some(url) => format!("{}. [{} #{}]({})", i + 1, a.branch, num, url),
                None => format!("{}. {} #{}", i + 1, a.branch, num),
            }
        })
        .collect();

    if linked.is_empty() {
        None
    } else {
        Some(format!("**Stack:**\n{}", linked.join("\n")))
    }
}
```

- [ ] **Step 2: Implement `build_stack_body`**

Replace the `todo!()` with:

```rust
pub fn build_stack_body(ancestors: &[AncestorPr], user_body: &str) -> String {
    match build_stack_section(ancestors) {
        Some(section) => format!("{}\n\n---\n\n{}", user_body, section),
        None => user_body.to_string(),
    }
}
```

- [ ] **Step 3: Run tests — expect all pass**

```bash
cargo test stack_body 2>&1
```

Expected: all tests pass. Output ends with `test result: ok. N passed; 0 failed`.

- [ ] **Step 4: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1
```

Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add src/stack_body.rs src/main.rs
git commit -m "feat: add stack_body module with unit tests"
```

---

## Chunk 2: Wire stack body into push_or_update_pr

### Task 3: Update `push_or_update_pr` to generate stack body

**Files:**
- Modify: `src/cmd/push.rs`

**Context:** `push_or_update_pr` currently receives `title_override: Option<&str>` and `body_override: Option<&str>`. We need to know whether the body was explicitly provided by the user (to decide whether to update existing PRs), and we need to build the ancestor list for the stack section.

- [ ] **Step 1: Read the current file**

Read `src/cmd/push.rs` in full before editing.

- [ ] **Step 2: Add `body_explicitly_set` parameter and ancestor-collection logic**

Change the signature of `push_or_update_pr` to:

```rust
pub fn push_or_update_pr(
    state: &mut StackState,
    branch: &str,
    parent: &str,
    draft: bool,
    title_override: Option<&str>,
    body_override: Option<&str>,
    body_explicitly_set: bool,
) -> Result<String> {
```

At the top of `push_or_update_pr`, before the `get_pr_status` call, add the ancestor-collection block:

```rust
// Collect upstream ancestor PRs for the stack section.
// path_to_trunk returns [branch, ..., trunk]; reverse and drop both ends.
let ancestors: Vec<crate::stack_body::AncestorPr> = {
    let path = state.path_to_trunk(branch);
    // path[0] = branch, path[last] = trunk
    // We want ancestors only: skip first (current) and last (trunk)
    let repo = github::repo_name().unwrap_or_default();
    path[1..path.len().saturating_sub(1)]
        .iter()
        .rev() // trunk-closest first
        .map(|b| {
            let pr_number = state.branches.get(b).and_then(|m| m.pr_number);
            let pr_url = pr_number.map(|n| {
                if repo.is_empty() {
                    String::new()
                } else {
                    format!("https://github.com/{}/pull/{}", repo, n)
                }
            });
            crate::stack_body::AncestorPr {
                branch: b.clone(),
                pr_number,
                pr_url,
            }
        })
        .collect()
};
```

**Note on path ordering:** `path_to_trunk` returns `[branch, parent, grandparent, ..., trunk]`. Slicing `[1..len-1]` gives `[parent, grandparent, ...]`. Reversing gives `[..., grandparent, parent]` which is trunk-closest-first. Correct.

- [ ] **Step 3: Use `build_stack_body` when creating a new PR**

In the `None` (new PR) arm, replace the body resolution:

```rust
None => {
    let range = format!("{parent}..{branch}");
    let commits = git::log_oneline(&range, 1)?;
    let derived_title = commits
        .first()
        .map(|(_, msg)| msg.clone())
        .unwrap_or_else(|| branch.to_string());

    let title = title_override.unwrap_or(&derived_title);
    let default_body = "Part of a stack managed by `ez`.";
    let raw_body = body_override.unwrap_or(default_body);

    // Always append stack section to new PRs.
    let body = crate::stack_body::build_stack_body(&ancestors, raw_body);

    let pr = github::create_pr(title, &body, parent, branch, draft)?;
    state.get_branch_mut(branch)?.pr_number = Some(pr.number);
    ui::info(&format!("Created PR #{}: {}", pr.number, pr.url));
    pr.url
}
```

- [ ] **Step 4: Use `build_stack_body` when updating an existing PR with explicit body**

In the `Some(pr)` arm, update the `edit_pr` block:

```rust
Some(pr) => {
    github::update_pr_base(pr.number, parent)?;
    state.get_branch_mut(branch)?.pr_number = Some(pr.number);
    ui::info(&format!("Updated PR #{} base to `{parent}`", pr.number));

    // Only update body if user explicitly passed --body/--body-file.
    if body_explicitly_set {
        let raw_body = body_override.unwrap_or("Part of a stack managed by `ez`.");
        let body = crate::stack_body::build_stack_body(&ancestors, raw_body);
        github::edit_pr(pr.number, title_override, Some(&body))?;
        if title_override.is_some() {
            ui::info(&format!("Updated PR #{} title", pr.number));
        }
        ui::info(&format!("Updated PR #{} body", pr.number));
    } else if title_override.is_some() {
        // Title only, no body change.
        github::edit_pr(pr.number, title_override, None)?;
        ui::info(&format!("Updated PR #{} title", pr.number));
    }

    pr.url
}
```

- [ ] **Step 5: Update all callers of `push_or_update_pr` to pass `body_explicitly_set`**

There are two callers:

**In `push::run`** — `body_explicitly_set` is `true` when `body_file` or `body` was provided:
```rust
let body_explicitly_set = body.is_some() || body_file.is_some();
// ...
let pr_url = push_or_update_pr(
    &mut state,
    &current,
    &parent,
    draft,
    title,
    resolved_body.as_deref(),
    body_explicitly_set,
)?;
```

**In `submit::run`** — same logic:
```rust
let body_explicitly_set = body.is_some() || body_file.is_some();
// ...
let pr_url = push_or_update_pr(
    &mut state,
    branch,
    &parent,
    draft,
    title,
    resolved_body.as_deref(),
    body_explicitly_set,
)?;
```

- [ ] **Step 6: Build to verify compilation**

```bash
cargo build 2>&1
```

Expected: clean build, no errors.

- [ ] **Step 7: Run all tests**

```bash
cargo test 2>&1
```

Expected: all tests pass (stack_body unit tests still passing).

- [ ] **Step 8: Run fmt and clippy**

```bash
cargo fmt --all
cargo clippy -- -D warnings 2>&1
```

Expected: no warnings.

- [ ] **Step 9: Commit**

```bash
git add src/cmd/push.rs src/cmd/submit.rs
git commit -m "feat: append stack ancestor links to PR body on creation"
```

---

## Chunk 3: Add `ez push --stack` flag

### Task 4: Add `--stack` to `Push` and delegate to `submit`

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/cmd/push.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add `stack` field to `Push` variant in `src/cli.rs`**

In the `Push` variant, add after `body_file`:
```rust
/// Push all branches in the stack (equivalent to ez submit)
#[arg(long)]
stack: bool,
```

- [ ] **Step 2: Pass `stack` through in `src/main.rs`**

Find the `Commands::Push { draft, title, body, body_file, base }` arm and add `stack`:
```rust
Commands::Push {
    draft,
    title,
    body,
    body_file,
    base,
    stack,
} => cmd::push::run(
    draft,
    title.as_deref(),
    body.as_deref(),
    body_file.as_deref(),
    base.as_deref(),
    stack,
),
```

- [ ] **Step 3: Update `push::run` signature and add delegation**

Change signature:
```rust
pub fn run(
    draft: bool,
    title: Option<&str>,
    body: Option<&str>,
    body_file: Option<&str>,
    base_override: Option<&str>,
    stack: bool,
) -> Result<()> {
```

At the very top of `run`, before any other logic, add:
```rust
if stack {
    return crate::cmd::submit::run(draft, title, body, body_file);
}
```

- [ ] **Step 4: Build**

```bash
cargo build 2>&1
```

Expected: clean build.

- [ ] **Step 5: Smoke test**

```bash
./target/debug/ez push --help 2>&1
```

Expected: `--stack` appears in the help output with description "Push all branches in the stack (equivalent to ez submit)".

- [ ] **Step 6: Run all tests, fmt, clippy**

```bash
cargo test 2>&1
cargo fmt --all
cargo clippy -- -D warnings 2>&1
```

Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/cli.rs src/cmd/push.rs src/main.rs
git commit -m "feat: add ez push --stack flag to push entire stack"
```

---

## Chunk 4: Version bump and final verification

### Task 5: Bump version to 0.1.4 and verify

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Bump version**

In `Cargo.toml`, change:
```toml
version = "0.1.3"
```
to:
```toml
version = "0.1.4"
```

- [ ] **Step 2: Full release build + all checks**

```bash
cargo build --release 2>&1
cargo test 2>&1
cargo clippy -- -D warnings 2>&1
```

Expected: all clean.

- [ ] **Step 3: Final smoke tests**

```bash
./target/release/ez --version
./target/release/ez push --help
./target/release/ez submit --help
```

Expected: version shows `0.1.4`, `--stack` visible in push help.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.1.4"
```
