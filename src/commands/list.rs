use anyhow::{anyhow, Result};
use colored::Colorize;
use serde::Serialize;
use std::path::Path;

use crate::context::Context;
use crate::store::{StoredTodo, Store};
use crate::time_util::{format_local_short, is_overdue, is_today_local};
use crate::todo::{Priority, Status};

#[derive(Debug, Clone, Copy)]
pub enum DueFilter {
    Today,
    ThisWeek,
    Overdue,
}

impl DueFilter {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "today" => Ok(Self::Today),
            "this-week" | "thisweek" => Ok(Self::ThisWeek),
            "overdue" => Ok(Self::Overdue),
            other => Err(anyhow!("unknown due filter '{other}'")),
        }
    }
}

pub fn run(
    root: &Path,
    context: Option<String>,
    due: Option<String>,
    tag: Option<String>,
    all: bool,
    json: bool,
) -> Result<()> {
    let store = Store::new(root);
    let mut items = store.list_all()?;

    if !all {
        items.retain(|t| matches!(t.todo.frontmatter.status, Status::Pending | Status::InProgress));
    }
    if let Some(ctx_str) = &context {
        let prefix = Context::parse(ctx_str)?;
        items.retain(|t| {
            Context::parse(&t.todo.frontmatter.context)
                .map(|c| c.matches_prefix(&prefix))
                .unwrap_or(false)
        });
    }
    if let Some(filter) = due.as_deref().map(DueFilter::parse).transpose()? {
        items.retain(|t| match (filter, t.todo.frontmatter.due) {
            (DueFilter::Today, Some(d)) => is_today_local(d),
            (DueFilter::ThisWeek, Some(d)) => is_within_local_week(d),
            (DueFilter::Overdue, Some(d)) => is_overdue(d),
            _ => false,
        });
    }
    if let Some(t) = &tag {
        items.retain(|x| x.todo.frontmatter.tags.iter().any(|tg| tg == t));
    }

    items.sort_by(|a, b| sort_key(&a.todo.frontmatter).cmp(&sort_key(&b.todo.frontmatter)));

    if json {
        print_json(&items)?;
    } else {
        print_human(&items);
    }
    Ok(())
}

fn sort_key(fm: &crate::todo::Frontmatter) -> (u8, i64, String) {
    let priority_rank = match fm.priority {
        Priority::Urgent => 0,
        Priority::High => 1,
        Priority::Normal => 2,
        Priority::Low => 3,
    };
    let due_ts = fm.due.map(|d| d.timestamp()).unwrap_or(i64::MAX);
    (priority_rank, due_ts, fm.id.clone())
}

fn is_within_local_week(dt: chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::{Datelike, Duration, Local};
    let today = Local::now().date_naive();
    let weekday = today.weekday().num_days_from_monday() as i64;
    let monday = today - Duration::days(weekday);
    let next_monday = monday + Duration::days(7);
    let local = dt.with_timezone(&Local).date_naive();
    local >= monday && local < next_monday
}

#[derive(Serialize)]
struct JsonItem<'a> {
    #[serde(flatten)]
    frontmatter: &'a crate::todo::Frontmatter,
    path: String,
}

fn print_json(items: &[StoredTodo]) -> Result<()> {
    let payload: Vec<JsonItem> = items
        .iter()
        .map(|s| JsonItem {
            frontmatter: &s.todo.frontmatter,
            path: s.path.display().to_string(),
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

pub fn print_human(items: &[StoredTodo]) {
    if items.is_empty() {
        println!("{}", "(no todos)".dimmed());
        return;
    }
    for s in items {
        let fm = &s.todo.frontmatter;
        let status_label = match fm.status {
            Status::Pending => "pending".cyan(),
            Status::InProgress => "in_prog".yellow(),
            Status::Done => "done".green().dimmed(),
            Status::Cancelled => "cancel".dimmed(),
        };
        let prio = match fm.priority {
            Priority::Urgent => "U".red().bold(),
            Priority::High => "H".red(),
            Priority::Normal => " ".normal(),
            Priority::Low => "l".dimmed(),
        };
        let due = match fm.due {
            Some(d) => {
                let s = format_local_short(d);
                if is_overdue(d) && matches!(fm.status, Status::Pending | Status::InProgress) {
                    format!("due {s}").red().to_string()
                } else if is_today_local(d) {
                    format!("due {s}").yellow().to_string()
                } else {
                    format!("due {s}").normal().to_string()
                }
            }
            None => String::new(),
        };
        println!(
            "  [{}] {} {:<28} {:<24} {}",
            status_label,
            prio,
            fm.id,
            fm.context,
            due
        );
    }
}
