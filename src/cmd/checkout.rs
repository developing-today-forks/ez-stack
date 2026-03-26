use anyhow::Result;
use dialoguer::Select;
use std::collections::HashMap;

use crate::git;
use crate::github;
use crate::stack::StackState;
use crate::ui;

/// Build a map of branch name → worktree path for branches in worktrees.
fn worktree_map() -> HashMap<String, String> {
    git::worktree_list()
        .unwrap_or_default()
        .into_iter()
        .filter(|wt| wt.path.contains("/.worktrees/"))
        .filter_map(|wt| wt.branch.map(|b| (b, wt.path)))
        .collect()
}

/// Switch to a branch. If it's in a worktree, print the path to stdout for cd.
fn switch_to(target: &str, wt_map: &HashMap<String, String>) -> Result<()> {
    if let Some(wt_path) = wt_map.get(target) {
        // Branch is in a worktree — print path to stdout for shell wrapper to cd.
        ui::success(&format!("Switching to `{target}` in worktree `{wt_path}`"));
        println!("{wt_path}");
    } else {
        git::checkout(target)?;
        ui::success(&format!("Switched to `{target}`"));
    }
    Ok(())
}

pub fn run(name: Option<&str>) -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;
    let wt_map = worktree_map();

    // Direct checkout by name or PR number.
    if let Some(arg) = name {
        let target = if let Ok(pr_num) = arg.parse::<u64>() {
            state
                .branches
                .values()
                .find(|m| m.pr_number == Some(pr_num))
                .map(|m| m.name.clone())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No branch found with PR #{pr_num}\n  → Run `ez branch` to see all branches"
                    )
                })?
        } else {
            if !state.is_trunk(arg) && !state.is_managed(arg) {
                anyhow::bail!(
                    "Branch `{arg}` is not tracked by ez\n  → Run `ez branch` to see all branches"
                );
            }
            arg.to_string()
        };

        if target == current {
            ui::info(&format!("Already on `{target}`"));
            return Ok(());
        }

        switch_to(&target, &wt_map)?;
        return Ok(());
    }

    // Interactive selector (existing code below, unchanged).

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

    switch_to(selected, &wt_map)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::stack::{BranchMeta, StackState};
    use std::collections::HashMap;

    fn state_with_pr() -> StackState {
        let mut branches = HashMap::new();
        branches.insert(
            "feat/x".to_string(),
            BranchMeta {
                name: "feat/x".to_string(),
                parent: "main".to_string(),
                parent_head: "abc".to_string(),
                pr_number: Some(99),
            },
        );
        StackState {
            trunk: "main".to_string(),
            remote: "origin".to_string(),
            branches,
        }
    }

    #[test]
    fn test_find_branch_by_pr_number() {
        let state = state_with_pr();
        let found = state
            .branches
            .values()
            .find(|m| m.pr_number == Some(99))
            .map(|m| m.name.clone());
        assert_eq!(found, Some("feat/x".to_string()));
    }

    #[test]
    fn test_arg_parses_as_pr_number() {
        assert!("99".parse::<u64>().is_ok());
        assert!("feat/x".parse::<u64>().is_err());
        assert!("0".parse::<u64>().is_ok());
    }
}
