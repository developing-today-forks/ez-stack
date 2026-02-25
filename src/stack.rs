use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::error::EzError;
use crate::git;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchMeta {
    pub name: String,
    pub parent: String,
    pub parent_head: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_number: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackState {
    pub trunk: String,
    pub remote: String,
    pub branches: HashMap<String, BranchMeta>,
}

impl StackState {
    pub fn new(trunk: String) -> Self {
        Self {
            trunk,
            remote: "origin".to_string(),
            branches: HashMap::new(),
        }
    }

    pub fn meta_dir() -> Result<PathBuf> {
        let root = git::repo_root()?;
        Ok(PathBuf::from(root).join(".git").join("ez"))
    }

    pub fn state_path() -> Result<PathBuf> {
        Ok(Self::meta_dir()?.join("stack.json"))
    }

    pub fn is_initialized() -> Result<bool> {
        Ok(Self::state_path()?.exists())
    }

    pub fn load() -> Result<Self> {
        let path = Self::state_path()?;
        if !path.exists() {
            bail!(EzError::NotInitialized);
        }
        let data = fs::read_to_string(&path)?;
        let state: StackState = serde_json::from_str(&data)?;
        Ok(state)
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::meta_dir()?;
        fs::create_dir_all(&dir)?;
        let data = serde_json::to_string_pretty(self)?;
        fs::write(Self::state_path()?, data)?;
        Ok(())
    }

    pub fn add_branch(&mut self, name: &str, parent: &str, parent_head: &str) {
        self.branches.insert(
            name.to_string(),
            BranchMeta {
                name: name.to_string(),
                parent: parent.to_string(),
                parent_head: parent_head.to_string(),
                pr_number: None,
            },
        );
    }

    pub fn remove_branch(&mut self, name: &str) {
        self.branches.remove(name);
    }

    pub fn get_branch(&self, name: &str) -> Result<&BranchMeta> {
        self.branches
            .get(name)
            .ok_or_else(|| EzError::BranchNotInStack(name.to_string()).into())
    }

    pub fn get_branch_mut(&mut self, name: &str) -> Result<&mut BranchMeta> {
        self.branches
            .get_mut(name)
            .ok_or_else(|| EzError::BranchNotInStack(name.to_string()).into())
    }

    pub fn children_of(&self, parent: &str) -> Vec<String> {
        let mut children: Vec<String> = self
            .branches
            .values()
            .filter(|b| b.parent == parent)
            .map(|b| b.name.clone())
            .collect();
        children.sort();
        children
    }

    pub fn is_trunk(&self, branch: &str) -> bool {
        branch == self.trunk
    }

    pub fn is_managed(&self, branch: &str) -> bool {
        self.branches.contains_key(branch)
    }

    /// Returns branches in topological order (parents before children).
    pub fn topo_order(&self) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();

        fn visit(
            name: &str,
            state: &StackState,
            visited: &mut std::collections::HashSet<String>,
            result: &mut Vec<String>,
        ) {
            if visited.contains(name) || state.is_trunk(name) {
                return;
            }
            visited.insert(name.to_string());
            if let Some(meta) = state.branches.get(name) {
                visit(&meta.parent, state, visited, result);
            }
            result.push(name.to_string());
        }

        for name in self.branches.keys() {
            visit(name, self, &mut visited, &mut result);
        }
        result
    }

    /// Walk up from a branch to trunk, returning the path (branch first, trunk last).
    pub fn path_to_trunk(&self, branch: &str) -> Vec<String> {
        let mut path = vec![branch.to_string()];
        let mut current = branch.to_string();
        let mut visited = std::collections::HashSet::new();
        visited.insert(branch.to_string());
        loop {
            if self.is_trunk(&current) {
                break;
            }
            match self.branches.get(&current) {
                Some(meta) => {
                    if !visited.insert(meta.parent.clone()) {
                        break; // cycle detected
                    }
                    path.push(meta.parent.clone());
                    current = meta.parent.clone();
                }
                None => break,
            }
        }
        path
    }

    /// Find the bottom branch (closest to trunk) in the stack containing `branch`.
    pub fn stack_bottom(&self, branch: &str) -> String {
        let path = self.path_to_trunk(branch);
        // path is [branch, ..., trunk], second to last is bottom
        if path.len() >= 2 {
            path[path.len() - 2].clone()
        } else {
            branch.to_string()
        }
    }

    /// Find the top branch (furthest from trunk) by following the first child repeatedly.
    pub fn stack_top(&self, branch: &str) -> String {
        let mut current = branch.to_string();
        let mut visited = std::collections::HashSet::new();
        visited.insert(branch.to_string());
        loop {
            let children = self.children_of(&current);
            if children.is_empty() {
                return current;
            }
            let next = children[0].clone();
            if !visited.insert(next.clone()) {
                return current; // cycle detected
            }
            current = next;
        }
    }
}
