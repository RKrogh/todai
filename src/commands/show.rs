use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::store::Store;
use crate::time_util::format_local;
use crate::todo::Status;

pub fn run(root: &Path, id: String, history: bool, json: bool) -> Result<()> {
    let store = Store::new(root);
    let stored = store.find(&id)?;

    if json {
        let payload = serde_json::json!({
            "id": stored.todo.frontmatter.id,
            "title": stored.todo.frontmatter.title,
            "context": stored.todo.frontmatter.context,
            "status": stored.todo.frontmatter.status,
            "priority": stored.todo.frontmatter.priority,
            "created": stored.todo.frontmatter.created,
            "due": stored.todo.frontmatter.due,
            "completed": stored.todo.frontmatter.completed,
            "completed_by": stored.todo.frontmatter.completed_by,
            "tags": stored.todo.frontmatter.tags,
            "remind": stored.todo.frontmatter.remind,
            "recur": stored.todo.frontmatter.recur,
            "body": stored.todo.body,
            "path": stored.path.display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let fm = &stored.todo.frontmatter;
    let status_label = match fm.status {
        Status::Pending => "pending".cyan(),
        Status::InProgress => "in_progress".yellow(),
        Status::Done => "done".green().dimmed(),
        Status::Cancelled => "cancelled".dimmed(),
    };
    println!("{} {} [{}]", "▶".bold(), fm.title.bold(), status_label);
    println!("  {:<14} {}", "id".dimmed(), fm.id);
    println!("  {:<14} {}", "context".dimmed(), fm.context);
    println!("  {:<14} {:?}", "priority".dimmed(), fm.priority);
    println!("  {:<14} {}", "created".dimmed(), format_local(fm.created));
    if let Some(d) = fm.due {
        println!("  {:<14} {}", "due".dimmed(), format_local(d));
    }
    if let Some(c) = fm.completed {
        println!("  {:<14} {}", "completed".dimmed(), format_local(c));
    }
    if let Some(by) = &fm.completed_by {
        println!("  {:<14} {}", "completed_by".dimmed(), by);
    }
    if !fm.tags.is_empty() {
        println!("  {:<14} {}", "tags".dimmed(), fm.tags.join(", "));
    }
    if !fm.remind.is_empty() {
        println!("  {}", "reminders:".dimmed());
        for r in &fm.remind {
            let state = if r.notified_at.is_some() { "sent" } else { "pending" };
            println!(
                "    {} [{:?}, {}] {}",
                format_local(r.at),
                r.notify_mode,
                state,
                r.message
            );
        }
    }
    if let Some(rec) = &fm.recur {
        let next = rec
            .next_due
            .map(format_local)
            .unwrap_or_else(|| "<unset>".to_string());
        println!("  {:<14} {:?} (next: {})", "recur".dimmed(), rec.rule, next);
        if let Some(prev) = &rec.prev_id {
            println!("  {:<14} {}", "prev_id".dimmed(), prev);
        }
    }
    println!("  {:<14} {}", "path".dimmed(), stored.path.display());

    if !stored.todo.body.trim().is_empty() {
        println!();
        println!("{}", "─".repeat(60).dimmed());
        println!("{}", stored.todo.body.trim_end());
    }

    if history {
        if let Some(prev_id) = fm.recur.as_ref().and_then(|r| r.prev_id.clone()) {
            println!();
            println!("{}", "History (prev_id chain):".bold());
            walk_history(&store, &prev_id)?;
        }
    }
    Ok(())
}

fn walk_history(store: &Store, start: &str) -> Result<()> {
    let mut next_id = start.to_string();
    loop {
        let found = match store.find(&next_id) {
            Ok(s) => s,
            Err(e) => {
                println!("  {} {}", "(broken link)".red(), e);
                break;
            }
        };
        let fm = &found.todo.frontmatter;
        let when = fm
            .completed
            .map(format_local)
            .unwrap_or_else(|| "(not completed)".to_string());
        println!("  ← {} {} [{}]", fm.id, fm.title.dimmed(), when);
        match fm.recur.as_ref().and_then(|r| r.prev_id.clone()) {
            Some(p) => next_id = p,
            None => break,
        }
    }
    Ok(())
}
