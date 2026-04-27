use anyhow::{anyhow, bail, Context as _, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::context::Context;
use crate::todo::{self, Todo};

const NANOID_ALPHABET: [char; 36] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
    'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't',
    'u', 'v', 'w', 'x', 'y', 'z',
];

const NANOID_LEN: usize = 4;

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct StoredTodo {
    pub todo: Todo,
    pub path: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[allow(dead_code)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_root(&self) -> Result<()> {
        fs::create_dir_all(&self.root)
            .with_context(|| format!("creating store root at {}", self.root.display()))?;
        fs::create_dir_all(self.root.join(".todai"))
            .with_context(|| "creating .todai directory")?;
        Ok(())
    }

    pub fn slug_from_title(title: &str) -> String {
        let s = slug::slugify(title);
        if s.is_empty() {
            nanoid::nanoid!(NANOID_LEN, &NANOID_ALPHABET)
        } else {
            s
        }
    }

    pub fn unique_id(&self, base_slug: &str, ctx: &Context) -> Result<String> {
        let dir = self.root.join(ctx.to_relative_path());
        if !dir.exists() || !dir.join(format!("{base_slug}.md")).exists() {
            return Ok(base_slug.to_string());
        }
        for _ in 0..16 {
            let suffix = nanoid::nanoid!(NANOID_LEN, &NANOID_ALPHABET);
            let candidate = format!("{base_slug}-{suffix}");
            if !dir.join(format!("{candidate}.md")).exists() {
                return Ok(candidate);
            }
        }
        bail!("unable to find unique id for slug '{base_slug}'");
    }

    pub fn path_for(&self, todo: &Todo) -> Result<PathBuf> {
        let ctx = Context::parse(&todo.frontmatter.context)?;
        Ok(self
            .root
            .join(ctx.to_relative_path())
            .join(format!("{}.md", todo.frontmatter.id)))
    }

    pub fn write(&self, todo: &Todo) -> Result<PathBuf> {
        todo.frontmatter.validate()?;
        let path = self.path_for(todo)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating context dir {}", parent.display()))?;
        }
        let rendered = todo::render(todo)?;
        fs::write(&path, rendered)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(path)
    }

    pub fn write_to(&self, todo: &Todo, path: &Path) -> Result<()> {
        todo.frontmatter.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let rendered = todo::render(todo)?;
        fs::write(path, rendered)?;
        Ok(())
    }

    pub fn read(&self, path: &Path) -> Result<StoredTodo> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let todo = todo::parse(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(StoredTodo {
            todo,
            path: path.to_path_buf(),
        })
    }

    pub fn list_all(&self) -> Result<Vec<StoredTodo>> {
        let mut results = Vec::new();
        if !self.root.exists() {
            return Ok(results);
        }
        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !is_skipped_dir(e.file_name().to_string_lossy().as_ref()))
        {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy();
            if !name.ends_with(".md") {
                continue;
            }
            if name.contains(".sync-conflict-") {
                continue;
            }
            match self.read(entry.path()) {
                Ok(stored) => results.push(stored),
                Err(e) => eprintln!("skipping {}: {e:#}", entry.path().display()),
            }
        }
        Ok(results)
    }

    pub fn find(&self, needle: &str) -> Result<StoredTodo> {
        let all = self.list_all()?;
        let exact: Vec<&StoredTodo> = all
            .iter()
            .filter(|t| t.todo.frontmatter.id == needle)
            .collect();
        if exact.len() == 1 {
            return Ok(exact[0].clone());
        }
        if exact.len() > 1 {
            bail!("multiple todos match id '{needle}' exactly — should not happen");
        }
        let prefix: Vec<&StoredTodo> = all
            .iter()
            .filter(|t| t.todo.frontmatter.id.starts_with(needle))
            .collect();
        if prefix.len() == 1 {
            return Ok(prefix[0].clone());
        }
        if prefix.len() > 1 {
            let ids: Vec<String> = prefix
                .iter()
                .map(|t| t.todo.frontmatter.id.clone())
                .collect();
            bail!("multiple todos match '{needle}': {}", ids.join(", "));
        }
        let contains: Vec<&StoredTodo> = all
            .iter()
            .filter(|t| t.todo.frontmatter.id.contains(needle))
            .collect();
        if contains.len() == 1 {
            return Ok(contains[0].clone());
        }
        if contains.len() > 1 {
            let ids: Vec<String> = contains
                .iter()
                .map(|t| t.todo.frontmatter.id.clone())
                .collect();
            bail!("multiple todos match '{needle}': {}", ids.join(", "));
        }
        Err(anyhow!("no todo found matching '{needle}'"))
    }
}

fn is_skipped_dir(name: &str) -> bool {
    matches!(name, ".todai" | ".archive" | ".git" | ".stfolder" | "node_modules")
        || name.starts_with(".sync-conflict-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::todo::{Frontmatter, Status, Todo};

    fn tmp_root() -> PathBuf {
        let id: String = nanoid::nanoid!(8);
        let p = std::env::temp_dir().join(format!("todai-test-{id}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_todo(id: &str, ctx: &str) -> Todo {
        Todo {
            frontmatter: Frontmatter {
                id: id.into(),
                title: "Test".into(),
                context: ctx.into(),
                status: Status::Pending,
                priority: Default::default(),
                created: Utc::now(),
                due: None,
                completed: None,
                completed_by: None,
                remind: vec![],
                tags: vec![],
                recur: None,
            },
            body: "body\n".into(),
        }
    }

    #[test]
    fn write_and_read_round_trip() {
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        let todo = make_todo("plant-carrots", "private:garden");
        let path = store.write(&todo).unwrap();
        assert!(path.ends_with("private/garden/plant-carrots.md"));
        let back = store.read(&path).unwrap();
        assert_eq!(back.todo.frontmatter.id, "plant-carrots");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn unique_id_appends_nanoid_on_collision() {
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        let ctx = Context::parse("private:garden").unwrap();
        let first = make_todo("plant-carrots", "private:garden");
        store.write(&first).unwrap();
        let next = store.unique_id("plant-carrots", &ctx).unwrap();
        assert_ne!(next, "plant-carrots");
        assert!(next.starts_with("plant-carrots-"));
        assert_eq!(next.len(), "plant-carrots-".len() + NANOID_LEN);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn list_all_works_when_root_basename_collides_with_skip_list() {
        // Regression: ~/.todai/ as the store root should still list todos,
        // even though ".todai" is in the internal skip list (for the metadata subdir).
        let parent = tmp_root();
        let root = parent.join(".todai");
        fs::create_dir_all(&root).unwrap();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        store.write(&make_todo("plant-carrots", "private:garden")).unwrap();
        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 1, "root named .todai should not prune the entire walk");
        assert_eq!(all[0].todo.frontmatter.id, "plant-carrots");
        fs::remove_dir_all(&parent).ok();
    }

    #[test]
    fn list_all_skips_internal_metadata_dir() {
        // The ".todai/" metadata subdir inside any store root must be skipped,
        // so config.toml and logs/ aren't mistaken for todo files.
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        // Drop a rogue .md inside the metadata dir — it must not be picked up.
        let meta_md = root.join(".todai/logs/noise.md");
        fs::create_dir_all(meta_md.parent().unwrap()).unwrap();
        fs::write(&meta_md, "not a todo").unwrap();
        store.write(&make_todo("real-todo", "private:inbox")).unwrap();
        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].todo.frontmatter.id, "real-todo");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn list_all_finds_todos_skips_conflicts() {
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        store.write(&make_todo("a", "work:assignments")).unwrap();
        store.write(&make_todo("b", "private:garden")).unwrap();
        let conflict_path = root.join("private/garden/a.sync-conflict-20260424.md");
        fs::create_dir_all(conflict_path.parent().unwrap()).unwrap();
        fs::write(&conflict_path, "garbage").unwrap();
        let all = store.list_all().unwrap();
        assert_eq!(all.len(), 2);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_by_exact_and_prefix() {
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        store.write(&make_todo("plant-carrots", "private:garden")).unwrap();
        store.write(&make_todo("buy-milk", "private:shoppinglist")).unwrap();
        let exact = store.find("plant-carrots").unwrap();
        assert_eq!(exact.todo.frontmatter.id, "plant-carrots");
        let prefix = store.find("plant").unwrap();
        assert_eq!(prefix.todo.frontmatter.id, "plant-carrots");
        fs::remove_dir_all(&root).ok();
    }
}
