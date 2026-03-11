use anyhow::Result;

use crate::git;
use crate::github;
use crate::stack::StackState;
use crate::ui;

pub fn run(_json: bool) -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    ui::header("Stack");

    // Print trunk as the root
    let trunk_display = ui::branch_display(&state.trunk, current == state.trunk);
    eprintln!("{trunk_display}");

    // Render children of trunk recursively
    let children = state.children_of(&state.trunk);
    let count = children.len();
    for (i, child) in children.iter().enumerate() {
        let is_last = i == count - 1;
        render_tree(&state, child, 1, is_last, &[], &current)?;
    }

    Ok(())
}

fn render_tree(
    state: &StackState,
    branch: &str,
    depth: usize,
    is_last: bool,
    ancestors_last: &[bool],
    current: &str,
) -> Result<()> {
    let is_current = branch == current;
    let meta = state.get_branch(branch)?;

    // Build the display text for this branch
    let name_display = ui::branch_display(branch, is_current);

    // Get PR badge if available
    let pr_text = if let Some(pr_number) = meta.pr_number {
        if let Ok(Some(pr)) = github::get_pr_status(branch) {
            let badge = ui::pr_badge(pr.number, &pr.state, pr.is_draft);
            let state_label = if pr.is_draft {
                "draft".to_string()
            } else {
                pr.state.clone()
            };
            format!(" ({badge} {state_label})")
        } else {
            format!(" ({})", ui::pr_badge(pr_number, "OPEN", false))
        }
    } else {
        String::new()
    };

    // Count commits on this branch
    let range = format!("{}..{}", meta.parent, branch);
    let commits = git::log_oneline(&range, 100).unwrap_or_default();
    let commit_count = commits.len();
    let commit_text = if commit_count == 1 {
        ui::dim(" 1 commit")
    } else {
        ui::dim(&format!(" {commit_count} commits"))
    };

    // Current branch indicator
    let current_marker = if is_current {
        format!("     {}", ui::dim("← current"))
    } else {
        String::new()
    };

    let line_text = format!("{name_display}{pr_text}{commit_text}{current_marker}");
    let line = ui::tree_line(depth, is_last, ancestors_last, &line_text);
    eprintln!("{line}");

    // Recurse into children
    let children = state.children_of(branch);
    let child_count = children.len();
    let mut child_ancestors = ancestors_last.to_vec();
    child_ancestors.push(is_last);
    for (i, child) in children.iter().enumerate() {
        let child_is_last = i == child_count - 1;
        render_tree(
            state,
            child,
            depth + 1,
            child_is_last,
            &child_ancestors,
            current,
        )?;
    }

    Ok(())
}
