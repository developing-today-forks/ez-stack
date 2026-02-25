use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn up() -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    let children = state.children_of(&current);
    if children.is_empty() {
        bail!(EzError::AlreadyAtTop);
    }

    let target = &children[0];
    git::checkout(target)?;
    ui::success(&format!(
        "Moved up: {} → {}",
        ui::branch_display(&current, false),
        ui::branch_display(target, true),
    ));

    Ok(())
}

pub fn down() -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    if state.is_trunk(&current) {
        bail!(EzError::AlreadyAtBottom);
    }

    if !state.is_managed(&current) {
        bail!(EzError::BranchNotInStack(current.clone()));
    }

    let parent = state.get_branch(&current)?.parent.clone();
    git::checkout(&parent)?;
    ui::success(&format!(
        "Moved down: {} → {}",
        ui::branch_display(&current, false),
        ui::branch_display(&parent, true),
    ));

    Ok(())
}

pub fn top() -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    let target = state.stack_top(&current);
    if target == current {
        bail!(EzError::AlreadyAtTop);
    }

    git::checkout(&target)?;
    ui::success(&format!(
        "Jumped to top: {} → {}",
        ui::branch_display(&current, false),
        ui::branch_display(&target, true),
    ));

    Ok(())
}

pub fn bottom() -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    let target = if state.is_trunk(&current) {
        // On trunk: go to the first child (bottom of the stack).
        let children = state.children_of(&current);
        if children.is_empty() {
            bail!(EzError::AlreadyAtBottom);
        }
        children[0].clone()
    } else {
        let bottom = state.stack_bottom(&current);
        if bottom == current {
            bail!(EzError::AlreadyAtBottom);
        }
        bottom
    };

    git::checkout(&target)?;
    ui::success(&format!(
        "Jumped to bottom: {} → {}",
        ui::branch_display(&current, false),
        ui::branch_display(&target, true),
    ));

    Ok(())
}
