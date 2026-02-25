use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run() -> Result<()> {
    let mut state = StackState::load()?;
    let original_branch = git::current_branch()?;

    let order = state.topo_order();
    let mut restacked = 0;

    for branch_name in &order {
        let meta = state.get_branch(branch_name)?;
        let parent = meta.parent.clone();
        let stored_parent_head = meta.parent_head.clone();

        let current_parent_tip = git::rev_parse(&parent)?;

        if current_parent_tip == stored_parent_head {
            continue;
        }

        // Branch is stale — rebase onto the new parent tip.
        let sp = ui::spinner(&format!("Restacking `{branch_name}` onto `{parent}`..."));
        let ok = git::rebase_onto(&current_parent_tip, &stored_parent_head, branch_name)?;
        sp.finish_and_clear();

        if ok {
            let meta = state.get_branch_mut(branch_name)?;
            meta.parent_head = current_parent_tip;
            restacked += 1;
            ui::success(&format!("Restacked `{branch_name}` onto `{parent}`"));
        } else {
            git::checkout(&original_branch)?;
            state.save()?;
            ui::hint("Resolve the conflicts manually, then run `ez restack` again.");
            bail!(EzError::RebaseConflict(branch_name.clone()));
        }
    }

    // Return to the original branch.
    git::checkout(&original_branch)?;

    state.save()?;

    if restacked == 0 {
        ui::info("All branches are up to date — nothing to restack");
    } else {
        ui::success(&format!("Restacked {restacked} branch(es)"));
    }

    Ok(())
}
