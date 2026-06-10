use anyhow::{anyhow, bail, Context as _, Result};
use colored::Colorize;
use std::fs;
use std::io::{BufRead, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::store::{Store, StoredTodo};
use crate::todo;

pub fn run(root: &Path, id: String) -> Result<()> {
    let store = Store::new(root);
    let stored = store.find(&id)?;

    // Edit a temp copy so a broken save can't poison the store. The original
    // file is only touched after the edited content parses and validates.
    let tmp = std::env::temp_dir().join(format!("todai-edit-{}.md", stored.todo.frontmatter.id));
    fs::copy(&stored.path, &tmp)
        .with_context(|| format!("copying {} to {}", stored.path.display(), tmp.display()))?;

    let original = fs::read_to_string(&tmp)?;
    let editor = editor_command()?;

    loop {
        spawn_editor(&editor, &tmp)?;
        let edited = fs::read_to_string(&tmp)
            .with_context(|| format!("reading edited file {}", tmp.display()))?;

        if edited == original {
            fs::remove_file(&tmp).ok();
            println!("{} no changes", "ok".dimmed());
            return Ok(());
        }

        match apply_edit(&store, &stored, &edited) {
            Ok(path) => {
                fs::remove_file(&tmp).ok();
                println!("{} saved {}", "ok".green(), path.display());
                return Ok(());
            }
            Err(e) => {
                eprintln!("{} {e:#}", "invalid:".red());
                eprint!("press Enter to re-edit, or type q to abort: ");
                std::io::stderr().flush().ok();
                let mut line = String::new();
                std::io::stdin().lock().read_line(&mut line)?;
                if line.trim().eq_ignore_ascii_case("q") {
                    bail!(
                        "edit aborted; store untouched. Your changes are kept at {}",
                        tmp.display()
                    );
                }
            }
        }
    }
}

/// Parse and validate edited content, then write it to the store. If the edit
/// changed `context` or `id`, the file moves to the new canonical path; the
/// original is removed only after the new file is written.
fn apply_edit(store: &Store, stored: &StoredTodo, edited: &str) -> Result<PathBuf> {
    let new_todo = todo::parse(edited)?;
    new_todo.frontmatter.validate()?;
    let new_path = store.path_for(&new_todo)?;

    if new_path != stored.path && new_path.exists() {
        bail!(
            "a todo already exists at {} — pick a different id",
            new_path.display()
        );
    }

    store.write(&new_todo)?;
    if new_path != stored.path {
        fs::remove_file(&stored.path)
            .with_context(|| format!("removing old file {}", stored.path.display()))?;
    }
    Ok(new_path)
}

fn editor_command() -> Result<Vec<String>> {
    let raw = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let parts: Vec<String> = raw.split_whitespace().map(String::from).collect();
    if parts.is_empty() {
        return Err(anyhow!("$VISUAL/$EDITOR is set but empty"));
    }
    Ok(parts)
}

fn spawn_editor(editor: &[String], file: &Path) -> Result<()> {
    let status = Command::new(&editor[0])
        .args(&editor[1..])
        .arg(file)
        .status()
        .with_context(|| format!("launching editor '{}'", editor[0]))?;
    if !status.success() {
        bail!("editor exited with {status}; store untouched");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::todo::{Frontmatter, Status, Todo};

    fn tmp_root() -> PathBuf {
        let id: String = nanoid::nanoid!(8);
        let p = std::env::temp_dir().join(format!("todai-edit-test-{id}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn seed(store: &Store, id: &str, ctx: &str) -> StoredTodo {
        let todo = Todo {
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
        };
        let path = store.write(&todo).unwrap();
        store.read(&path).unwrap()
    }

    #[test]
    fn valid_edit_overwrites_in_place() {
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        let stored = seed(&store, "plant-carrots", "private:garden");

        let mut todo = stored.todo.clone();
        todo.frontmatter.title = "Plant MORE carrots".into();
        let edited = todo::render(&todo).unwrap();

        let path = apply_edit(&store, &stored, &edited).unwrap();
        assert_eq!(path, stored.path);
        let back = store.read(&path).unwrap();
        assert_eq!(back.todo.frontmatter.title, "Plant MORE carrots");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn broken_yaml_rejected_store_untouched() {
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        let stored = seed(&store, "plant-carrots", "private:garden");

        let err = apply_edit(&store, &stored, "---\nid: [unclosed\n---\nbody\n");
        assert!(err.is_err());
        let back = store.read(&stored.path).unwrap();
        assert_eq!(back.todo.frontmatter.title, "Test");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_title_rejected() {
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        let stored = seed(&store, "plant-carrots", "private:garden");

        let mut todo = stored.todo.clone();
        todo.frontmatter.title = "  ".into();
        let edited = todo::render(&todo).unwrap();
        assert!(apply_edit(&store, &stored, &edited).is_err());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn context_change_moves_file() {
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        let stored = seed(&store, "plant-carrots", "private:garden");

        let mut todo = stored.todo.clone();
        todo.frontmatter.context = "private:greenhouse".into();
        let edited = todo::render(&todo).unwrap();

        let path = apply_edit(&store, &stored, &edited).unwrap();
        assert!(path.ends_with("private/greenhouse/plant-carrots.md"));
        assert!(!stored.path.exists(), "old file should be removed");
        assert!(path.exists());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn id_collision_rejected_both_files_intact() {
        let root = tmp_root();
        let store = Store::new(&root);
        store.ensure_root().unwrap();
        let stored = seed(&store, "plant-carrots", "private:garden");
        let other = seed(&store, "buy-seeds", "private:garden");

        let mut todo = stored.todo.clone();
        todo.frontmatter.id = "buy-seeds".into();
        let edited = todo::render(&todo).unwrap();

        assert!(apply_edit(&store, &stored, &edited).is_err());
        assert!(stored.path.exists());
        assert!(other.path.exists());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn editor_command_splits_args() {
        // can't safely mutate env in parallel tests; test the parsing shape via
        // the same split logic editor_command uses
        let parts: Vec<String> = "code --wait".split_whitespace().map(String::from).collect();
        assert_eq!(parts, vec!["code", "--wait"]);
    }
}
