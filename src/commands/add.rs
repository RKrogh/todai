use anyhow::{anyhow, Result};
use chrono::Utc;
use colored::Colorize;
use std::path::Path;

use crate::config;
use crate::context::Context;
use crate::store::Store;
use crate::time_util::parse_user_datetime;
use crate::todo::{Frontmatter, NotifyMode, Priority, Recur, RecurRule, Reminder, Status, Todo};

#[allow(clippy::too_many_arguments)]
pub fn run(
    root: &Path,
    title: String,
    context: Option<String>,
    due: Option<String>,
    priority: Option<String>,
    remind: Vec<String>,
    tag: Vec<String>,
    recur: Option<String>,
) -> Result<()> {
    let cfg = config::load(root)?;
    let store = Store::new(root);
    store.ensure_root()?;

    let context_str = context.unwrap_or(cfg.general.default_context);
    let ctx = Context::parse(&context_str)?;

    if !cfg.contexts.allowed.is_empty() && !cfg.contexts.allowed.contains(&context_str) {
        eprintln!(
            "{} context '{}' not in [contexts.allowed]",
            "warning:".yellow().bold(),
            context_str
        );
    }

    let base_slug = Store::slug_from_title(&title);
    let id = store.unique_id(&base_slug, &ctx)?;

    let due_dt = match due.as_deref() {
        Some(s) => Some(parse_user_datetime(s)?),
        None => None,
    };

    let priority = match priority.as_deref() {
        None => Priority::Normal,
        Some("low") => Priority::Low,
        Some("normal") => Priority::Normal,
        Some("high") => Priority::High,
        Some("urgent") => Priority::Urgent,
        Some(other) => return Err(anyhow!("unknown priority '{other}'")),
    };

    let mut reminders = Vec::with_capacity(remind.len());
    for r in &remind {
        let at = parse_user_datetime(r)?;
        reminders.push(Reminder {
            at,
            message: title.clone(),
            notified_at: None,
            notify_mode: match cfg.notifications.default_notify_mode.as_str() {
                "dumb" => NotifyMode::Dumb,
                _ => NotifyMode::Smart,
            },
        });
    }

    let recur = match recur.as_deref() {
        None => None,
        Some(s) => {
            let rule = match s {
                "daily" => RecurRule::Daily,
                "weekly" => RecurRule::Weekly,
                "monthly" => RecurRule::Monthly,
                "yearly" => RecurRule::Yearly,
                other => return Err(anyhow!("unknown recur rule '{other}'")),
            };
            let next_due = due_dt.map(|d| advance_by_rule(d, rule));
            Some(Recur {
                rule,
                next_due,
                prev_id: None,
            })
        }
    };

    let todo = Todo {
        frontmatter: Frontmatter {
            id: id.clone(),
            title,
            context: ctx.as_str(),
            status: Status::Pending,
            priority,
            created: Utc::now(),
            due: due_dt,
            completed: None,
            completed_by: None,
            remind: reminders,
            tags: tag,
            recur,
        },
        body: String::new(),
    };

    let path = store.write(&todo)?;
    println!("{} {} ({})", "added".green().bold(), id, path.display());
    Ok(())
}

pub fn advance_by_rule(dt: chrono::DateTime<Utc>, rule: RecurRule) -> chrono::DateTime<Utc> {
    use chrono::{Datelike, Duration};
    match rule {
        RecurRule::Daily => dt + Duration::days(1),
        RecurRule::Weekly => dt + Duration::weeks(1),
        RecurRule::Monthly => add_months(dt, 1),
        RecurRule::Yearly => dt
            .with_year(dt.year() + 1)
            .unwrap_or_else(|| dt + Duration::days(365)),
    }
}

fn add_months(dt: chrono::DateTime<Utc>, months: i32) -> chrono::DateTime<Utc> {
    use chrono::{Datelike, NaiveDate, TimeZone};
    let year = dt.year();
    let month0 = dt.month0() as i32 + months;
    let new_year = year + month0.div_euclid(12);
    let new_month = month0.rem_euclid(12) as u32 + 1;
    let day = dt.day();
    let new_date = NaiveDate::from_ymd_opt(new_year, new_month, day)
        .or_else(|| {
            let mut d = day;
            while d > 28 {
                d -= 1;
                if let Some(date) = NaiveDate::from_ymd_opt(new_year, new_month, d) {
                    return Some(date);
                }
            }
            None
        })
        .unwrap_or_else(|| dt.date_naive());
    let naive = new_date.and_time(dt.time());
    Utc.from_utc_datetime(&naive)
}
