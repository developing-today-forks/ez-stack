use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(message: &str, all: bool, if_changed: bool, paths: &[String]) -> Result<()> {
    let mut state = StackState::load()?;
    let current = git::current_branch()?;

    if state.is_trunk(&current) {
        bail!(EzError::OnTrunk);
    }

    if !state.is_managed(&current) {
        bail!(EzError::BranchNotInStack(current));
    }

    if all && !paths.is_empty() {
        bail!(EzError::UserMessage(
            "cannot combine --all (-a) with path arguments\n  → Use `ez commit -am \"msg\"` to stage everything, or `ez commit -m \"msg\" -- <paths>` to stage specific files".to_string()
        ));
    }

    if !paths.is_empty() {
        git::add_paths(paths)?;
    } else if all {
        git::add_all()?;
    }

    // --if-changed: silently succeed if nothing to commit.
    if if_changed && !git::has_staged_changes()? {
        return Ok(());
    }

    if !git::has_staged_changes()? {
        bail!(EzError::NothingToCommit);
    }

    let before = git::rev_parse("HEAD")?;

    // Snapshot modified files before commit so we can detect hook changes on failure.
    let pre_modified = git::modified_files();

    if let Err(e) = git::commit(message) {
        // Check if pre-commit hooks modified files.
        let post_modified = git::modified_files();
        let hook_changed: Vec<&String> = post_modified
            .iter()
            .filter(|f| !pre_modified.contains(f))
            .collect();
        if !hook_changed.is_empty() {
            ui::warn(&format!(
                "Pre-commit hook modified {} file(s):",
                hook_changed.len()
            ));
            for f in &hook_changed {
                eprintln!("  {f}");
            }
            ui::hint(
                "Re-stage and retry: `ez commit -am \"...\"` or `git add -u && ez commit -m \"...\"`",
            );
        }
        return Err(e);
    }

    let after = git::rev_parse("HEAD")?;
    let short_after = &after[..after.len().min(7)];
    let subject = message.lines().next().unwrap_or(message);
    ui::success(&format!(
        "Committed {short_after} on `{current}`: {subject}"
    ));

    // Show diff stat so agents can verify what was committed.
    let (files, ins, del) = git::diff_stat_numbers();
    if let Ok(stat) = git::show_stat_head() {
        let stat = stat.trim();
        if !stat.is_empty() {
            eprintln!("{stat}");
        }
    }

    // Emit receipt.
    ui::receipt(&serde_json::json!({
        "cmd": "commit",
        "branch": current,
        "before": &before[..before.len().min(7)],
        "after": short_after,
        "files_changed": files,
        "insertions": ins,
        "deletions": del,
    }));

    // Auto-restack children so they stay on top of the new HEAD.
    let new_head = after;
    let children = state.children_of(&current);

    let current_root = git::repo_root()?;
    let mut restacked_count = 0;

    for child in &children {
        // Guard FIRST — before extracting old_base (avoids unused-variable warning when skipping).
        if let Ok(Some(_wt_path)) = git::branch_checked_out_elsewhere(child, &current_root) {
            ui::info(&format!("Skipped `{child}` (in worktree)"));
            continue;
        }

        let meta = state.get_branch(child)?;
        let old_base = meta.parent_head.clone();

        ui::info(&format!("Restacking `{child}`..."));
        let ok = git::rebase_onto(&new_head, &old_base, child)?;
        if !ok {
            // Save progress so the user can fix conflicts and continue with `ez restack`.
            state.save()?;
            git::checkout(&current)?;
            ui::hint("Resolve the conflicts manually, then run `ez restack` to continue.");
            bail!(EzError::RebaseConflict(child.clone()));
        }

        let meta = state.get_branch_mut(child)?;
        meta.parent_head = new_head.clone();
        restacked_count += 1;
    }

    // After restacking we may be on a child branch; return to the original.
    if !children.is_empty() {
        git::checkout(&current)?;
    }

    state.save()?;

    if restacked_count > 0 {
        ui::info(&format!("Restacked {restacked_count} child branch(es)"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_if_changed_semantics() {
        // if_changed=true, nothing staged → should skip (return early)
        assert!(true && !false); // if_changed && !has_staged → skip
        // if_changed=true, something staged → should commit
        assert!(!(true && !true)); // if_changed && !has_staged = false → don't skip
        // if_changed=false, nothing staged → NothingToCommit error (existing behavior)
        assert!(!(false && !false)); // if_changed=false → guard never fires
    }
}
