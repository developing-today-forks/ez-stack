use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::github;
use crate::stack::StackState;
use crate::ui;

pub fn run(title: Option<&str>, body: Option<&str>, body_file: Option<&str>) -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    if state.is_trunk(&current) {
        bail!(EzError::OnTrunk);
    }

    if !state.is_managed(&current) {
        bail!(EzError::BranchNotInStack(current.clone()));
    }

    let meta = state.get_branch(&current)?;
    let pr_number = meta.pr_number.ok_or_else(|| {
        anyhow::anyhow!(
            "No PR found for branch `{current}` — run `ez push` to create one first"
        )
    })?;

    let resolved_body: Option<String> = if let Some(path) = body_file {
        Some(github::body_from_file(path)?)
    } else {
        body.map(|s| s.to_string())
    };

    github::edit_pr(pr_number, title, resolved_body.as_deref())?;

    if let Ok(Some(pr)) = github::get_pr_status(&current) {
        ui::success(&format!("Updated PR #{}: {}", pr.number, pr.url));
    } else {
        ui::success(&format!("Updated PR #{pr_number}"));
    }

    Ok(())
}
