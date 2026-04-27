use anyhow::{anyhow, Result};
use chrono::Utc;
use colored::Colorize;
use std::path::Path;

use crate::store::Store;

pub fn run(root: &Path, id: String, reminder: usize) -> Result<()> {
    let store = Store::new(root);
    let mut stored = store.find(&id)?;
    let total = stored.todo.frontmatter.remind.len();
    if reminder >= total {
        return Err(anyhow!(
            "reminder index {reminder} out of range; todo has {total} reminder(s)"
        ));
    }
    let r = &mut stored.todo.frontmatter.remind[reminder];
    if r.notified_at.is_some() {
        println!(
            "{} reminder {} on {} was already marked notified",
            "ok".dimmed(),
            reminder,
            stored.todo.frontmatter.id
        );
        return Ok(());
    }
    r.notified_at = Some(Utc::now());
    store.write_to(&stored.todo, &stored.path)?;
    println!(
        "{} reminder {} on {} marked notified",
        "ok".green(),
        reminder,
        stored.todo.frontmatter.id
    );
    Ok(())
}
