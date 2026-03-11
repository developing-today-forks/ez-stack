use anyhow::Result;
use dialoguer::Select;

use crate::git;
use crate::github;
use crate::stack::StackState;
use crate::ui;

pub fn run(_name: Option<&str>) -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    // Collect all managed branches, sorted
    let mut branches: Vec<String> = state.branches.keys().cloned().collect();
    branches.sort();

    // Add trunk at the beginning
    branches.insert(0, state.trunk.clone());

    // Build display items with PR badges
    let display_items: Vec<String> = branches
        .iter()
        .map(|name| {
            let is_current = name == &current;
            let branch_text = ui::branch_display(name, is_current);

            if let Some(meta) = state.branches.get(name)
                && let Some(pr_number) = meta.pr_number
            {
                if let Ok(Some(pr)) = github::get_pr_status(name) {
                    return format!(
                        "{} {}",
                        branch_text,
                        ui::pr_badge(pr.number, &pr.state, pr.is_draft)
                    );
                }
                return format!("{} {}", branch_text, ui::pr_badge(pr_number, "OPEN", false));
            }

            branch_text
        })
        .collect();

    // Find the index of the current branch for default selection
    let default_idx = branches.iter().position(|b| b == &current).unwrap_or(0);

    let selection = Select::new()
        .with_prompt("Select branch")
        .items(&display_items)
        .default(default_idx)
        .interact()?;

    let selected = &branches[selection];

    if selected == &current {
        ui::info(&format!("Already on `{selected}`"));
        return Ok(());
    }

    git::checkout(selected)?;
    ui::success(&format!("Switched to `{selected}`"));

    Ok(())
}
