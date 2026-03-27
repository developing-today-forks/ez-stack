use anyhow::Result;
use std::collections::HashMap;

use crate::git;
use crate::stack::StackState;

pub fn run() -> Result<()> {
    let state = StackState::load()?;
    let current = git::current_branch()?;

    // Build branch→worktree path map.
    let worktree_map: HashMap<String, String> = git::worktree_list()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|wt| wt.branch.map(|b| (b, wt.path)))
        .collect();

    let order = state.topo_order();

    for branch in &order {
        let meta = state.get_branch(branch)?;

        let marker = if *branch == current { "* " } else { "  " };
        let pr = meta.pr_number.map(|n| format!(" #{n}")).unwrap_or_default();
        let wt = worktree_map
            .get(branch.as_str())
            .map(|p| format!(" {p}"))
            .unwrap_or_default();

        // Machine-readable to stdout: marker, name, PR, worktree path.
        println!("{marker}{branch}{pr}{wt}");
    }

    Ok(())
}
