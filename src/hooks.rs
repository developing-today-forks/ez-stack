use std::path::{Path, PathBuf};

use crate::git;
use crate::ui;

/// Hook files live in `.ez/hooks/<event>/` in the main worktree root.
/// They are markdown files with instructions for agents, NOT executable scripts.
///
/// Directory structure:
///   .ez/hooks/
///     post-create/
///       default.md       ← runs unless --hook overrides
///       setup-node.md    ← ez create --hook setup-node
///       setup-python.md  ← ez create --hook setup-python
///     pre-push/
///       default.md
///
/// ez prints the hook contents to stderr. The agent reads and follows them.
fn hooks_dir() -> Option<PathBuf> {
    let root = git::main_worktree_root().ok()?;
    Some(Path::new(&root).join(".ez/hooks"))
}

/// Get hook content for a specific event and optional hook name.
/// If hook_name is None, looks for "default.md".
/// If hook_name is Some, looks for "<name>.md".
pub fn get_hook(event: &str, hook_name: Option<&str>) -> Option<String> {
    let dir = hooks_dir()?;
    let name = hook_name.unwrap_or("default");
    let hook_path = dir.join(event).join(format!("{name}.md"));

    if !hook_path.exists() {
        return None;
    }

    std::fs::read_to_string(&hook_path).ok()
}

/// Print hook instructions to stderr if the hook file exists.
/// Returns true if a hook was found and printed.
pub fn emit_hook(event: &str, hook_name: Option<&str>) -> bool {
    let name = hook_name.unwrap_or("default");
    if let Some(content) = get_hook(event, hook_name) {
        let content = content.trim();
        if content.is_empty() {
            return false;
        }
        if hook_name.is_some() {
            ui::info(&format!("Hook: {event}/{name}"));
        } else {
            ui::info(&format!("Hook: {event}"));
        }
        eprintln!("{content}");
        true
    } else {
        false
    }
}

/// List available hooks for an event.
#[allow(dead_code)]
pub fn list_hooks(event: &str) -> Vec<String> {
    let dir = match hooks_dir() {
        Some(d) => d.join(event),
        None => return vec![],
    };

    if !dir.exists() {
        return vec![];
    }

    std::fs::read_dir(&dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.strip_suffix(".md").map(|n| n.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}
