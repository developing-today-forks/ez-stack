use anyhow::{Result, bail};

use crate::error::EzError;
use crate::git;
use crate::stack::StackState;
use crate::ui;

pub fn run(trunk: Option<String>) -> Result<()> {
    if !git::is_repo() {
        bail!(EzError::NotARepo);
    }

    if StackState::is_initialized()? {
        bail!(EzError::AlreadyInitialized);
    }

    let trunk = match trunk {
        Some(t) => t,
        None => git::default_branch()?,
    };

    let state = StackState::new(trunk.clone());
    state.save()?;

    ui::success(&format!("Initialized ez with trunk branch `{trunk}`"));
    Ok(())
}
