use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(message: Option<&str>, all: bool) -> Result<()> {
    let mut state = StackState::load()?;
    let current = git::current_branch()?;

    if state.is_trunk(&current) {
        bail!(EzError::OnTrunk);
    }

    if !state.is_managed(&current) {
        bail!(EzError::BranchNotInStack(current.clone()));
    }

    if all {
        git::add_all()?;
    }

    if !all && !git::has_staged_changes()? {
        bail!(EzError::NothingToCommit);
    }

    git::commit_amend(message)?;
    ui::success("Amended commit");

    // Auto-restack children of the current branch.
    let current_head = git::rev_parse("HEAD")?;
    let children = state.children_of(&current);

    for child_name in &children {
        let old_parent_head = state.get_branch(child_name)?.parent_head.clone();

        let sp = ui::spinner(&format!("Restacking `{child_name}`..."));
        let ok = git::rebase_onto(&current_head, &old_parent_head, child_name)?;
        sp.finish_and_clear();

        if ok {
            let child = state.get_branch_mut(child_name)?;
            child.parent_head = current_head.clone();
            ui::info(&format!("Restacked `{child_name}`"));
        } else {
            git::checkout(&current)?;
            state.save()?;
            ui::hint("Resolve conflicts, then run `ez restack`");
            bail!(EzError::RebaseConflict(child_name.clone()));
        }
    }

    // Return to the original branch after restacking.
    git::checkout(&current)?;

    state.save()?;
    ui::success(&format!(
        "Amended `{current}` and restacked {} child branch(es)",
        children.len()
    ));
    Ok(())
}
