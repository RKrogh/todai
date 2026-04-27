use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use std::path::Path;

use crate::commands::add::advance_by_rule;
use crate::context::Context;
use crate::store::Store;
use crate::todo::{Recur, Reminder, Status, Todo};

pub fn run(root: &Path, id: String) -> Result<()> {
    let store = Store::new(root);
    let mut stored = store.find(&id)?;

    if matches!(stored.todo.frontmatter.status, Status::Done) {
        println!("{} {} already done", "ok".dimmed(), stored.todo.frontmatter.id);
        return Ok(());
    }

    let now = Utc::now();
    let hostname = hostname();

    stored.todo.frontmatter.status = Status::Done;
    stored.todo.frontmatter.completed = Some(now);
    stored.todo.frontmatter.completed_by = Some(hostname.clone());
    store.write_to(&stored.todo, &stored.path)?;
    println!(
        "{} {} (completed by {})",
        "done".green().bold(),
        stored.todo.frontmatter.id,
        hostname
    );

    let recur = stored.todo.frontmatter.recur.clone();
    if let Some(rec) = recur {
        let next = spawn_next(&store, &stored.todo, &rec)?;
        if let Some(new_id) = next {
            println!("  {} spawned next instance: {}", "↻".cyan(), new_id);
        }
    }
    Ok(())
}

fn spawn_next(store: &Store, prev: &Todo, rec: &Recur) -> Result<Option<String>> {
    let new_due = match rec.next_due {
        Some(d) => d,
        None => match prev.frontmatter.due {
            Some(d) => advance_by_rule(d, rec.rule),
            None => return Ok(None),
        },
    };
    let new_next_due = advance_by_rule(new_due, rec.rule);

    let ctx = Context::parse(&prev.frontmatter.context)?;
    let base_slug = crate::store::Store::slug_from_title(&prev.frontmatter.title);
    let new_id = store.unique_id(&base_slug, &ctx)?;

    let new_reminders: Vec<Reminder> = prev
        .frontmatter
        .remind
        .iter()
        .map(|r| {
            let shift = new_due.signed_duration_since(prev.frontmatter.due.unwrap_or(new_due));
            Reminder {
                at: r.at + shift,
                message: r.message.clone(),
                notified_at: None,
                notify_mode: r.notify_mode,
            }
        })
        .collect();

    let mut next = prev.clone();
    next.frontmatter.id = new_id.clone();
    next.frontmatter.status = Status::Pending;
    next.frontmatter.created = Utc::now();
    next.frontmatter.due = Some(new_due);
    next.frontmatter.completed = None;
    next.frontmatter.completed_by = None;
    next.frontmatter.remind = new_reminders;
    next.frontmatter.recur = Some(Recur {
        rule: rec.rule,
        next_due: Some(new_next_due),
        prev_id: Some(prev.frontmatter.id.clone()),
    });

    store.write(&next)?;
    Ok(Some(new_id))
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}
