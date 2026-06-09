//! Sweep done/cancelled todos older than `general.archive_after_days` into `.archive/`.
//!
//! `done` only flags a todo (sets status + completed timestamp); the file stays put so the
//! user keeps a short "undo" window. This command is the second stage: it moves anything that
//! has been done/cancelled longer than the grace period out of the active context folders.
//! Intended to be run periodically (e.g. by the agent's morning tick), not on every `done`.

use anyhow::Result;
use chrono::{Duration, Utc};
use colored::Colorize;
use std::path::Path;

use crate::config;
use crate::store::Store;
use crate::todo::Status;

pub fn run(root: &Path, dry_run: bool) -> Result<()> {
    let cfg = config::load(root)?;
    if !cfg.general.archive_done {
        println!(
            "{} archiving disabled (general.archive_done = false); todos stay flagged in place",
            "skip".dimmed()
        );
        return Ok(());
    }

    let store = Store::new(root);
    let cutoff = Utc::now() - Duration::days(cfg.general.archive_after_days as i64);

    let mut archived = 0u32;
    let mut skipped_no_date = 0u32;
    for stored in store.list_all()? {
        let fm = &stored.todo.frontmatter;
        if !matches!(fm.status, Status::Done | Status::Cancelled) {
            continue;
        }
        let Some(completed) = fm.completed else {
            // Done/cancelled but no completion timestamp: can't age it, leave it alone.
            skipped_no_date += 1;
            continue;
        };
        if completed >= cutoff {
            continue; // still inside the grace window
        }

        if dry_run {
            println!(
                "  {} {} (completed {})",
                "would archive".yellow(),
                fm.id,
                completed.format("%Y-%m-%d")
            );
        } else {
            let new_path = store.archive(&stored)?;
            println!("  {} {} -> {}", "archived".green(), fm.id, new_path.display());
        }
        archived += 1;
    }

    let tail = if dry_run { "would be archived" } else { "archived" };
    println!(
        "{} {} todo(s) {} (done/cancelled > {} days)",
        "done".bold(),
        archived,
        tail,
        cfg.general.archive_after_days
    );
    if skipped_no_date > 0 {
        println!(
            "  {} {} done/cancelled todo(s) had no completed date; left in place",
            "note".dimmed(),
            skipped_no_date
        );
    }
    Ok(())
}
