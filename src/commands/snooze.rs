use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use colored::Colorize;
use std::path::Path;

use crate::store::Store;

pub fn run(root: &Path, id: String, for_str: String) -> Result<()> {
    let delta = parse_duration(&for_str)?;
    let store = Store::new(root);
    let mut stored = store.find(&id)?;
    let now = Utc::now();
    let mut shifted = 0usize;
    for r in stored.todo.frontmatter.remind.iter_mut() {
        if r.notified_at.is_some() {
            continue;
        }
        r.at = if r.at < now { now + delta } else { r.at + delta };
        shifted += 1;
    }
    if shifted == 0 {
        println!(
            "{} no pending reminders on {}",
            "ok".dimmed(),
            stored.todo.frontmatter.id
        );
        return Ok(());
    }
    store.write_to(&stored.todo, &stored.path)?;
    println!(
        "{} snoozed {} reminder(s) on {} by {}",
        "ok".green(),
        shifted,
        stored.todo.frontmatter.id,
        for_str
    );
    Ok(())
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return Err(anyhow!("empty duration"));
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let n: i64 = num_str
        .parse()
        .map_err(|_| anyhow!("bad duration '{s}', expected like 1h, 30m, 2d"))?;
    match unit {
        "m" => Ok(Duration::minutes(n)),
        "h" => Ok(Duration::hours(n)),
        "d" => Ok(Duration::days(n)),
        "w" => Ok(Duration::weeks(n)),
        other => Err(anyhow!("unknown duration unit '{other}', expected m/h/d/w")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_units() {
        assert_eq!(parse_duration("30m").unwrap(), Duration::minutes(30));
        assert_eq!(parse_duration("2h").unwrap(), Duration::hours(2));
        assert_eq!(parse_duration("3d").unwrap(), Duration::days(3));
        assert_eq!(parse_duration("1w").unwrap(), Duration::weeks(1));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("xyz").is_err());
        assert!(parse_duration("5x").is_err());
    }
}
