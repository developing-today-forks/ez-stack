use std::path::Path;
use std::process::Command;

use crate::git;
use crate::ui;

/// Run a hook script if it exists. Hook scripts live in `.ez/hooks/` in the main
/// worktree root. They are plain shell scripts (must be executable).
///
/// Hooks run in `working_dir` with these environment variables set:
/// - EZ_BRANCH: the branch name
/// - EZ_WORKTREE: the worktree path (if applicable)
/// - EZ_PARENT: the parent branch name
///
/// Hook failures warn but don't abort the command.
pub fn run_hook(hook_name: &str, working_dir: &str, branch: &str, parent: &str, worktree: &str) {
    let root = match git::main_worktree_root() {
        Ok(r) => r,
        Err(_) => return,
    };

    let hook_path = Path::new(&root).join(".ez/hooks").join(hook_name);

    if !hook_path.exists() {
        return;
    }

    ui::info(&format!("Running hook: {hook_name}"));

    let result = Command::new("bash")
        .arg(hook_path.to_str().unwrap_or(""))
        .current_dir(working_dir)
        .env("EZ_BRANCH", branch)
        .env("EZ_WORKTREE", worktree)
        .env("EZ_PARENT", parent)
        .status();

    match result {
        Ok(status) if status.success() => {
            ui::success(&format!("Hook `{hook_name}` completed"));
        }
        Ok(status) => {
            ui::warn(&format!(
                "Hook `{hook_name}` exited with code {}",
                status.code().unwrap_or(-1)
            ));
        }
        Err(e) => {
            ui::warn(&format!("Hook `{hook_name}` failed to run: {e}"));
        }
    }
}
