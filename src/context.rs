use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    parts: Vec<String>,
}

impl Context {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            bail!("context cannot be empty");
        }
        let parts: Vec<String> = trimmed
            .split(':')
            .map(|p| p.trim().to_string())
            .collect();
        for part in &parts {
            if part.is_empty() {
                bail!("context segment cannot be empty (got '{}')", input);
            }
            if part.contains('/') || part.contains('\\') {
                bail!("context segment '{}' contains a path separator", part);
            }
            if part.starts_with('.') {
                bail!("context segment '{}' cannot start with '.'", part);
            }
        }
        Ok(Self { parts })
    }

    pub fn as_str(&self) -> String {
        self.parts.join(":")
    }

    pub fn to_relative_path(&self) -> PathBuf {
        self.parts.iter().collect()
    }

    #[allow(dead_code)]
    pub fn from_relative_path(rel: &Path) -> Result<Self> {
        let parts: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        if parts.is_empty() {
            bail!("cannot derive context from empty path");
        }
        Ok(Self { parts })
    }

    pub fn matches_prefix(&self, prefix: &Context) -> bool {
        if prefix.parts.len() > self.parts.len() {
            return false;
        }
        prefix
            .parts
            .iter()
            .zip(self.parts.iter())
            .all(|(a, b)| a == b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_render() {
        let c = Context::parse("work:staff").unwrap();
        assert_eq!(c.as_str(), "work:staff");
        assert_eq!(c.to_relative_path(), PathBuf::from("work/staff"));
    }

    #[test]
    fn from_path_round_trip() {
        let c = Context::from_relative_path(Path::new("private/garden")).unwrap();
        assert_eq!(c.as_str(), "private:garden");
    }

    #[test]
    fn rejects_empty_segment() {
        assert!(Context::parse("work::staff").is_err());
        assert!(Context::parse("").is_err());
        assert!(Context::parse(":work").is_err());
    }

    #[test]
    fn rejects_path_separators() {
        assert!(Context::parse("work/staff").is_err());
    }

    #[test]
    fn prefix_matching() {
        let parent = Context::parse("work").unwrap();
        let child = Context::parse("work:staff").unwrap();
        let other = Context::parse("private:kids").unwrap();
        assert!(child.matches_prefix(&parent));
        assert!(parent.matches_prefix(&parent));
        assert!(!parent.matches_prefix(&child));
        assert!(!other.matches_prefix(&parent));
    }
}
