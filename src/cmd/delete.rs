use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::github;
use crate::stack::StackState;
use crate::ui;

pub fn run(branch: Option<&str>, force: bool) -> Result<()> {
    let mut state = StackState::load()?;
    let current = git::current_branch()?;

    let target = branch.unwrap_or(&current).to_string();

    if state.is_trunk(&target) {
        bail!(EzError::OnTrunk);
    }

    if !state.is_managed(&target) {
        bail!(EzError::BranchNotInStack(target.clone()));
    }

    // Worktree guard: if the target branch is checked out in another worktree, bail.
    let current_root = git::repo_root()?;
    if let Ok(Some(wt_path)) = git::branch_checked_out_elsewhere(&target, &current_root) {
        bail!(EzError::UserMessage(format!(
            "branch `{target}` is checked out in worktree `{wt_path}`\n  → Run `ez worktree delete {target}` to remove it first"
        )));
    }

    let meta = state.get_branch(&target)?;
    let parent = meta.parent.clone();
    let pr_number = meta.pr_number;

    // The children are based on the deleted branch's commits, so their parent_head
    // should be the deleted branch's current tip (used as old_base during restack).
    let parent_head_for_children = git::rev_parse(&target)?;

    // Reparent children of the target branch to the target's parent.
    let children = state.children_of(&target);
    for child_name in &children {
        let child = state.get_branch_mut(child_name)?;
        child.parent = parent.clone();
        child.parent_head = parent_head_for_children.clone();
        ui::info(&format!("Reparented `{child_name}` onto `{parent}`"));
    }

    // If the target had a PR, update the base branch of each child's PR on GitHub.
    if pr_number.is_some() {
        let new_base = parent.clone();
        for child_name in &children {
            let child = state.get_branch(child_name)?;
            if let Some(child_pr) = child.pr_number
                && let Err(e) = github::update_pr_base(child_pr, &new_base)
            {
                ui::warn(&format!("Failed to update PR base for `{child_name}`: {e}"));
            }
        }
    }

    // If currently on the target branch, checkout parent first.
    if current == target {
        git::checkout(&parent)?;
    }

    // Delete local branch (BEFORE removing from state, so state isn't corrupted on failure).
    if git::branch_exists(&target)
        && let Err(e) = git::delete_branch(&target, force)
    {
        if force {
            ui::warn(&format!("Failed to delete local branch `{target}`: {e}"));
        } else {
            ui::warn(&format!(
                "Branch `{target}` has unmerged changes — use --force to delete anyway"
            ));
            state.save()?;
            return Err(e);
        }
    }

    // Try to delete remote branch (ignore errors).
    let _ = git::delete_remote_branch(&state.remote, &target);

    // Remove from state only after git operations succeeded.
    state.remove_branch(&target);
    state.save()?;

    ui::success(&format!("Deleted branch `{target}`"));
    if !children.is_empty() {
        ui::hint(&format!(
            "Run `ez restack` to rebase reparented branches onto `{parent}`"
        ));
    }

    ui::receipt(&serde_json::json!({
        "cmd": "delete",
        "branch": target,
        "parent": parent,
        "reparented_children": children.len(),
    }));

    Ok(())
}
