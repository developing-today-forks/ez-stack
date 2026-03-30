use anyhow::Result;
use std::path::PathBuf;

use crate::ui;

const SKILL_CONTENT: &str = include_str!("../../SKILL.md");

fn skill_dir() -> Result<PathBuf> {
    let root = crate::git::repo_root()?;
    Ok(PathBuf::from(root).join(".claude/skills/ez-workflow"))
}

pub fn install() -> Result<()> {
    let dir = skill_dir()?;
    let skill_path = dir.join("SKILL.md");

    if skill_path.exists() {
        // Update existing skill.
        let existing = std::fs::read_to_string(&skill_path)?;
        if existing == SKILL_CONTENT {
            ui::info("ez-workflow skill is already up to date");
            return Ok(());
        }
        std::fs::write(&skill_path, SKILL_CONTENT)?;
        ui::success("Updated ez-workflow skill");
    } else {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&skill_path, SKILL_CONTENT)?;
        ui::success("Installed ez-workflow skill");
    }

    // Machine output: path to stdout.
    println!("{}", skill_path.display());

    Ok(())
}

pub fn uninstall() -> Result<()> {
    let dir = skill_dir()?;

    if !dir.exists() {
        ui::info("ez-workflow skill is not installed in this repo");
        return Ok(());
    }

    std::fs::remove_dir_all(&dir)?;
    ui::success("Uninstalled ez-workflow skill");
    ui::hint("Remove .claude/skills/ez-workflow/ from version control if committed");

    Ok(())
}
