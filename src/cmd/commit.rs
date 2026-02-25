use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(message: &str, all: bool) -> Result<()> {
    let mut state = StackState::load()?;
    let current = git::current_branch()?;

    if state.is_trunk(&current) {
        bail!(EzError::OnTrunk);
    }

    if !state.is_managed(&current) {
        bail!(EzError::BranchNotInStack(current));
    }

    if all {
        git::add_all()?;
    }

    if !git::has_staged_changes()? {
        bail!(EzError::NothingToCommit);
    }

    git::commit(message)?;
    ui::success(&format!("Committed on `{current}`: {message}"));

    // Auto-restack children so they stay on top of the new HEAD.
    let new_head = git::rev_parse("HEAD")?;
    let children = state.children_of(&current);

    for child in &children {
        let meta = state.get_branch(child)?;
        let old_base = meta.parent_head.clone();

        ui::info(&format!("Restacking `{child}` onto `{current}`..."));
        let ok = git::rebase_onto(&new_head, &old_base, child)?;
        if !ok {
            // Checkout back to the branch we were on before reporting the conflict.
            git::checkout(&current)?;
            bail!(EzError::RebaseConflict(child.clone()));
        }

        // Update the child's parent_head to reflect the new base.
        let meta = state.get_branch_mut(child)?;
        meta.parent_head = new_head.clone();
    }

    // After restacking we may be on a child branch; return to the original.
    if !children.is_empty() {
        git::checkout(&current)?;
    }

    state.save()?;

    if !children.is_empty() {
        ui::success(&format!("Restacked {} child branch(es)", children.len()));
    }

    Ok(())
}
