use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::commands::list::print_human;
use crate::store::Store;
use crate::time_util::{is_overdue, is_today_local};
use crate::todo::Status;

pub fn run(root: &Path, json: bool) -> Result<()> {
    let store = Store::new(root);
    let mut items = store.list_all()?;

    items.retain(|t| {
        if !matches!(
            t.todo.frontmatter.status,
            Status::Pending | Status::InProgress
        ) {
            return false;
        }
        let due_today_or_overdue = t
            .todo
            .frontmatter
            .due
            .map(|d| is_today_local(d) || is_overdue(d))
            .unwrap_or(false);
        let reminder_today = t
            .todo
            .frontmatter
            .remind
            .iter()
            .any(|r| r.notified_at.is_none() && is_today_local(r.at));
        due_today_or_overdue || reminder_today
    });

    items.sort_by_key(|s| s.todo.frontmatter.due.map(|d| d.timestamp()).unwrap_or(i64::MAX));

    if json {
        let payload: Vec<_> = items
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.todo.frontmatter.id,
                    "title": s.todo.frontmatter.title,
                    "context": s.todo.frontmatter.context,
                    "due": s.todo.frontmatter.due,
                    "priority": s.todo.frontmatter.priority,
                    "path": s.path.display().to_string(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("{}", "Today:".bold());
        print_human(&items);
    }
    Ok(())
}
