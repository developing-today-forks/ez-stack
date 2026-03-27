use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::github;
use crate::stack::StackState;
use crate::ui;

/// Resolve the `.worktrees/<name>` path relative to the main worktree root.
/// Uses the first entry from `git worktree list` which is always the main worktree.
fn main_worktree_root() -> Result<String> {
    let worktrees = git::worktree_list()?;
    worktrees
        .first()
        .map(|wt| wt.path.clone())
        .ok_or_else(|| anyhow::anyhow!("could not determine main worktree root"))
}

fn worktree_path(name: &str) -> Result<String> {
    let root = main_worktree_root()?;
    Ok(format!("{root}/.worktrees/{name}"))
}

pub fn create(name: &str, from: Option<&str>) -> Result<()> {
    let mut state = StackState::load()?;
    let current = git::current_branch()?;

    // --- Phase 1: Validate (no mutations) ---

    let parent = if let Some(base) = from {
        if !state.is_trunk(base) && !state.is_managed(base) {
            bail!(EzError::UserMessage(format!(
                "branch `{base}` is not tracked by ez — use trunk or a managed branch with --from"
            )));
        }
        base.to_string()
    } else {
        if !state.is_trunk(&current) && !state.is_managed(&current) {
            bail!(EzError::UserMessage(format!(
                "current branch `{current}` is not tracked by ez — switch to a managed branch or trunk first"
            )));
        }
        current.clone()
    };

    if git::branch_exists(name) {
        ui::hint(&format!("Use `ez checkout {name}` to switch to it"));
        bail!(EzError::BranchAlreadyExists(name.to_string()));
    }

    let wt_path = worktree_path(name)?;
    let parent_head = git::rev_parse(&parent)?;

    // --- Phase 2: Mutate (all-or-nothing) ---

    git::create_branch_at(name, &parent_head)?;
    state.add_branch(name, &parent, &parent_head);

    if let Err(e) = git::worktree_add(&wt_path, name) {
        // Rollback: remove the branch we just created.
        let _ = git::delete_branch(name, true);
        state.remove_branch(name);
        return Err(e);
    }

    state.save()?;

    // --- Phase 3: Output ---

    ui::success(&format!(
        "Created branch `{name}` on top of `{parent}` in worktree `{wt_path}`"
    ));
    ui::hint(&format!("cd {wt_path}"));

    ui::receipt(&serde_json::json!({
        "cmd": "worktree_create",
        "branch": name,
        "parent": parent,
        "worktree_path": wt_path,
    }));

    println!("{wt_path}");

    Ok(())
}

pub fn delete(name: &str, force: bool, yes: bool) -> Result<()> {
    let mut state = StackState::load()?;

    // --- Phase 1: Gather all info and validate (no mutations) ---

    let repo_root = main_worktree_root()?;
    let wt_path = worktree_path(name)?;
    let current_dir = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_default();

    let inside_worktree = current_dir == wt_path || current_dir.starts_with(&format!("{wt_path}/"));

    if inside_worktree && !yes {
        ui::warn(&format!(
            "You are inside the worktree `{name}` that you are about to delete"
        ));
        if !ui::confirm("Delete this worktree and switch to the repo root?") {
            ui::info("Cancelled");
            return Ok(());
        }
    }

    // Snapshot branch info from worktree list before any mutations.
    let branch = git::worktree_list()?
        .into_iter()
        .find(|wt| wt.path == wt_path)
        .and_then(|wt| wt.branch);

    // Pre-compute all stack changes so we can apply them atomically.
    struct StackCleanup {
        branch_name: String,
        parent: String,
        pr_number: Option<u64>,
        parent_head_for_children: String,
        children: Vec<String>,
        child_prs: Vec<(String, Option<u64>)>,
    }

    let cleanup = if let Some(ref branch_name) = branch {
        if state.is_managed(branch_name) {
            let meta = state.get_branch(branch_name)?;
            let parent = meta.parent.clone();
            let pr_number = meta.pr_number;
            let parent_head_for_children =
                git::rev_parse(branch_name).unwrap_or_else(|_| meta.parent_head.clone());
            let children = state.children_of(branch_name);
            let child_prs: Vec<(String, Option<u64>)> = children
                .iter()
                .filter_map(|c| state.get_branch(c).ok().map(|m| (c.clone(), m.pr_number)))
                .collect();
            Some(StackCleanup {
                branch_name: branch_name.clone(),
                parent,
                pr_number,
                parent_head_for_children,
                children,
                child_prs,
            })
        } else {
            None
        }
    } else {
        None
    };

    // --- Phase 2: Mutate filesystem (the one step that can fail) ---

    // Move out of the worktree before removing it.
    if inside_worktree {
        std::env::set_current_dir(&repo_root)?;
    }

    // Prune stale entries from previous failed deletes.
    let _ = git::worktree_prune();

    let wt_dir = std::path::Path::new(&wt_path);
    if wt_dir.exists() && wt_dir.join(".git").exists() {
        let result = if force {
            git::worktree_remove_force(&wt_path)
        } else {
            git::worktree_remove(&wt_path)
        };
        if let Err(e) = result {
            // Worktree removal failed — nothing else was changed. Bail cleanly.
            bail!(
                "Could not remove worktree at `{wt_path}`: {e}\n\
                 Use `ez worktree delete {name} --force` to discard uncommitted changes"
            );
        }
        ui::success(&format!("Removed worktree at `{wt_path}`"));
    } else if wt_dir.exists() {
        let _ = std::fs::remove_dir_all(&wt_path);
        ui::success(&format!("Cleaned up stale directory at `{wt_path}`"));
    } else {
        ui::info(&format!("Worktree directory already removed: `{wt_path}`"));
    }

    // --- Phase 3: Mutate stack state (atomic — only runs after worktree is gone) ---

    if let Some(c) = cleanup {
        // Reparent children in state.
        for child_name in &c.children {
            if let Ok(child) = state.get_branch_mut(child_name) {
                child.parent = c.parent.clone();
                child.parent_head = c.parent_head_for_children.clone();
            }
            ui::info(&format!("Reparented `{child_name}` onto `{}`", c.parent));
        }

        // Update PR bases on GitHub (best-effort, don't fail the command).
        if c.pr_number.is_some() {
            for (child_name, child_pr) in &c.child_prs {
                if let Some(pr) = child_pr {
                    if let Err(e) = github::update_pr_base(*pr, &c.parent) {
                        ui::warn(&format!("Failed to update PR base for `{child_name}`: {e}"));
                    }
                }
            }
        }

        state.remove_branch(&c.branch_name);
        let _ = git::delete_branch(&c.branch_name, true);
        state.save()?;

        ui::success(&format!("Deleted branch `{}`", c.branch_name));

        if !c.children.is_empty() {
            ui::hint(&format!(
                "Run `ez restack` to rebase reparented branches onto `{}`",
                c.parent
            ));
        }

        ui::receipt(&serde_json::json!({
            "cmd": "worktree_delete",
            "branch": c.branch_name,
            "worktree_path": wt_path,
        }));
    } else if let Some(branch_name) = &branch {
        // Branch exists but isn't ez-managed — just delete it.
        let _ = git::delete_branch(branch_name, force);
        state.save()?;

        ui::receipt(&serde_json::json!({
            "cmd": "worktree_delete",
            "branch": branch_name,
            "worktree_path": wt_path,
        }));
    } else {
        state.save()?;

        ui::receipt(&serde_json::json!({
            "cmd": "worktree_delete",
            "branch": serde_json::Value::Null,
            "worktree_path": wt_path,
        }));
    }

    // --- Phase 4: Output ---

    if inside_worktree {
        ui::hint(&format!("cd {repo_root}"));
        println!("{repo_root}");
    }

    Ok(())
}

pub fn list() -> Result<()> {
    let worktrees = git::worktree_list()?;
    if worktrees.is_empty() {
        ui::info("No worktrees found");
        return Ok(());
    }
    for wt in worktrees {
        let name = std::path::Path::new(&wt.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&wt.path);
        let branch = wt.branch.as_deref().unwrap_or("(detached HEAD)");
        println!("{:<30} {}  {}", name, branch, &wt.path);
    }
    Ok(())
}
