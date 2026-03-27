use anyhow::Result;

use crate::error::EzError;
use crate::git;
use crate::github;
use crate::stack::StackState;
use crate::ui;

pub fn run(json: bool) -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    if json {
        let is_trunk = state.is_trunk(&current);
        let children = state.children_of(&current);

        if is_trunk {
            println!(
                "{}",
                serde_json::json!({
                    "branch": current,
                    "parent": serde_json::Value::Null,
                    "pr_number": serde_json::Value::Null,
                    "pr_url": serde_json::Value::Null,
                    "pr_state": serde_json::Value::Null,
                    "is_draft": false,
                    "depth": 0_usize,
                    "commits": 0_usize,
                    "children": children,
                    "needs_restack": false,
                })
            );
            return Ok(());
        }

        if !state.is_managed(&current) {
            anyhow::bail!(EzError::BranchNotInStack(current.clone()));
        }

        let meta = state.get_branch(&current)?;

        let depth = state.path_to_trunk(&current).len().saturating_sub(1);

        let range = format!("{}..{}", meta.parent, current);
        let commits = git::log_oneline(&range, 50)?;
        let commit_count = commits.len();

        let needs_restack = git::rev_parse(&meta.parent)
            .map(|tip| tip != meta.parent_head)
            .unwrap_or(false);

        let pr_status = github::get_pr_status(&current).unwrap_or(None);
        let pr_number_val: serde_json::Value = match meta.pr_number {
            Some(n) => serde_json::Value::Number(n.into()),
            None => serde_json::Value::Null,
        };
        let pr_url_val: serde_json::Value = match meta.pr_number {
            Some(n) => match github::repo_name().ok() {
                Some(repo) => {
                    serde_json::Value::String(format!("https://github.com/{repo}/pull/{n}"))
                }
                None => serde_json::Value::Null,
            },
            None => serde_json::Value::Null,
        };
        let pr_state_val: serde_json::Value = match meta.pr_number {
            Some(_) => match &pr_status {
                Some(pr) => serde_json::Value::String(pr.state.clone()),
                None => serde_json::Value::String("OPEN".to_string()),
            },
            None => serde_json::Value::Null,
        };
        let is_draft_val: bool = match &pr_status {
            Some(pr) => pr.is_draft,
            None => false,
        };

        println!(
            "{}",
            serde_json::json!({
                "branch": current,
                "parent": meta.parent,
                "pr_number": pr_number_val,
                "pr_url": pr_url_val,
                "pr_state": pr_state_val,
                "is_draft": is_draft_val,
                "depth": depth,
                "commits": commit_count,
                "children": children,
                "needs_restack": needs_restack,
            })
        );
        return Ok(());
    }

    // If on trunk, show trunk info and direct children
    if state.is_trunk(&current) {
        ui::header(&format!(
            "On trunk branch: {}",
            ui::branch_display(&current, true)
        ));
        let children = state.children_of(&current);
        if children.is_empty() {
            ui::info("No stacked branches yet.");
            ui::hint("Run `ez create <name>` to start a stack.");
        } else {
            ui::info(&format!("{} stacked branch(es):", children.len()));
            for child in &children {
                eprintln!("  {}", ui::branch_display(child, false));
            }
        }
        return Ok(());
    }

    // Branch must be managed
    if !state.is_managed(&current) {
        ui::warn(&format!("Branch `{current}` is not tracked by ez."));
        ui::hint("Run `ez create <name>` from a tracked branch to add it to the stack.");
        return Ok(());
    }

    let meta = state.get_branch(&current)?;

    // Branch name header
    ui::header(&format!("Branch: {}", ui::branch_display(&current, true)));

    // Parent
    ui::info(&format!(
        "Parent: {}",
        ui::branch_display(&meta.parent, false)
    ));

    // Children
    let children = state.children_of(&current);
    if children.is_empty() {
        ui::info("Children: none (top of stack)");
    } else {
        ui::info(&format!(
            "Children: {}",
            children
                .iter()
                .map(|c| ui::branch_display(c, false))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // PR status
    if let Some(pr_number) = meta.pr_number {
        match github::get_pr_status(&current) {
            Ok(Some(pr)) => {
                let badge = ui::pr_badge(pr.number, &pr.state, pr.is_draft);
                let state_label = if pr.is_draft {
                    "draft".to_string()
                } else {
                    pr.state.clone()
                };
                ui::info(&format!("PR: {badge} {state_label} — {}", pr.title));
                ui::hint(&pr.url);
            }
            _ => {
                ui::info(&format!("PR: {}", ui::pr_badge(pr_number, "OPEN", false)));
            }
        }
    } else {
        ui::info("PR: not yet created");
        ui::hint("Run `ez submit` to create a PR.");
    }

    // Stack position
    let path = state.path_to_trunk(&current);
    let depth = path.len() - 1; // subtract trunk
    let path_display: Vec<String> = path
        .iter()
        .rev()
        .map(|b| ui::branch_display(b, b == &current))
        .collect();
    ui::info(&format!(
        "Stack position: {} deep ({})",
        depth,
        path_display.join(" → ")
    ));

    // Commits on this branch
    let range = format!("{}..{}", meta.parent, current);
    let commits = git::log_oneline(&range, 50)?;
    if commits.is_empty() {
        ui::info("Commits: none");
    } else {
        let label = if commits.len() == 1 {
            "commit"
        } else {
            "commits"
        };
        ui::info(&format!("Commits: {} {label}", commits.len()));
        for (sha, msg) in &commits {
            eprintln!("  {} {}", ui::dim(sha), msg);
        }
    }

    // Check if needs restack
    let parent_actual_head = git::rev_parse(&meta.parent)?;
    if meta.parent_head != parent_actual_head {
        ui::warn("Branch may need restacking — parent has moved.");
        ui::hint("Run `ez restack` to update.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_status_json_schema_keys() {
        let val = serde_json::json!({
            "branch": "feat/x",
            "parent": "main",
            "pr_number": 42_u64,
            "pr_url": "https://github.com/a/b/pull/42",
            "pr_state": "OPEN",
            "is_draft": false,
            "depth": 2_usize,
            "commits": 1_usize,
            "children": ["feat/y"],
            "needs_restack": false,
        });
        assert_eq!(val["branch"], "feat/x");
        assert_eq!(val["pr_number"], 42);
        assert!(val["children"].is_array());
        assert_eq!(val["needs_restack"], false);
    }
}
