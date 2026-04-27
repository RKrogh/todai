use anyhow::{anyhow, bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    InProgress,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurRule {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotifyMode {
    Smart,
    Dumb,
}

impl Default for NotifyMode {
    fn default() -> Self { NotifyMode::Smart }
}

impl Default for Priority {
    fn default() -> Self { Priority::Normal }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub at: DateTime<Utc>,
    pub message: String,
    #[serde(default)]
    pub notified_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub notify_mode: NotifyMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recur {
    pub rule: RecurRule,
    #[serde(default)]
    pub next_due: Option<DateTime<Utc>>,
    #[serde(default)]
    pub prev_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub id: String,
    pub title: String,
    pub context: String,
    pub status: Status,
    #[serde(default)]
    pub priority: Priority,
    pub created: DateTime<Utc>,
    #[serde(default)]
    pub due: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed_by: Option<String>,
    #[serde(default)]
    pub remind: Vec<Reminder>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub recur: Option<Recur>,
}

#[derive(Debug, Clone)]
pub struct Todo {
    pub frontmatter: Frontmatter,
    pub body: String,
}

const DELIM: &str = "---";

pub fn parse(content: &str) -> Result<Todo> {
    let normalized = content.replace("\r\n", "\n");
    let rest = normalized
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("file does not start with '---' frontmatter delimiter"))?;
    let end = rest
        .find("\n---\n")
        .or_else(|| rest.strip_suffix("\n---").map(|_| rest.len() - 3))
        .ok_or_else(|| anyhow!("missing closing '---' frontmatter delimiter"))?;
    let yaml = &rest[..end];
    let body = rest[end..]
        .strip_prefix("\n---\n")
        .or_else(|| rest[end..].strip_prefix("\n---"))
        .unwrap_or("")
        .trim_start_matches('\n')
        .to_string();
    let frontmatter: Frontmatter = serde_saphyr::from_str(yaml)
        .with_context(|| "failed to parse YAML frontmatter")?;
    Ok(Todo { frontmatter, body })
}

pub fn render(todo: &Todo) -> Result<String> {
    let yaml = serde_saphyr::to_string(&todo.frontmatter)
        .context("failed to serialize frontmatter to YAML")?;
    let yaml_trimmed = yaml.trim_end_matches('\n');
    let body = todo.body.trim_start_matches('\n');
    let body_section = if body.is_empty() {
        String::new()
    } else {
        format!("{body}\n")
    };
    Ok(format!("{DELIM}\n{yaml_trimmed}\n{DELIM}\n\n{body_section}"))
}

impl Frontmatter {
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            bail!("id must not be empty");
        }
        if self.title.trim().is_empty() {
            bail!("title must not be empty");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_todo() -> Todo {
        Todo {
            frontmatter: Frontmatter {
                id: "book-1on1-with-erik".into(),
                title: "Book 1-on-1 with Erik".into(),
                context: "work:staff".into(),
                status: Status::Pending,
                priority: Priority::Normal,
                created: Utc.with_ymd_and_hms(2026, 4, 16, 9, 12, 0).unwrap(),
                due: Some(Utc.with_ymd_and_hms(2026, 5, 16, 14, 0, 0).unwrap()),
                completed: None,
                completed_by: None,
                remind: vec![Reminder {
                    at: Utc.with_ymd_and_hms(2026, 5, 9, 7, 0, 0).unwrap(),
                    message: "Schedule the 1-on-1 with Erik".into(),
                    notified_at: None,
                    notify_mode: NotifyMode::Smart,
                }],
                tags: vec!["meeting".into(), "erik".into()],
                recur: Some(Recur {
                    rule: RecurRule::Monthly,
                    next_due: Some(Utc.with_ymd_and_hms(2026, 6, 16, 14, 0, 0).unwrap()),
                    prev_id: None,
                }),
            },
            body: "Book a 1-on-1 with Erik.\n\n## Notes\n- previous topic: migration\n".into(),
        }
    }

    #[test]
    fn round_trip_full_todo() {
        let original = sample_todo();
        let rendered = render(&original).expect("render");
        let parsed = parse(&rendered).expect("parse");
        assert_eq!(parsed.frontmatter.id, original.frontmatter.id);
        assert_eq!(parsed.frontmatter.title, original.frontmatter.title);
        assert_eq!(parsed.frontmatter.context, original.frontmatter.context);
        assert_eq!(parsed.frontmatter.status, original.frontmatter.status);
        assert_eq!(parsed.frontmatter.priority, original.frontmatter.priority);
        assert_eq!(parsed.frontmatter.created, original.frontmatter.created);
        assert_eq!(parsed.frontmatter.due, original.frontmatter.due);
        assert_eq!(parsed.frontmatter.tags, original.frontmatter.tags);
        assert_eq!(parsed.frontmatter.remind.len(), 1);
        assert_eq!(parsed.frontmatter.remind[0].message, original.frontmatter.remind[0].message);
        assert!(parsed.frontmatter.recur.is_some());
        assert_eq!(parsed.body.trim(), original.body.trim());
    }

    #[test]
    fn parse_minimal_todo() {
        let src = "---\n\
                   id: plant-carrots\n\
                   title: Plant carrots\n\
                   context: private:garden\n\
                   status: pending\n\
                   created: 2026-04-24T09:00:00Z\n\
                   ---\n\
                   \n\
                   Body content here.\n";
        let parsed = parse(src).expect("parse minimal");
        assert_eq!(parsed.frontmatter.id, "plant-carrots");
        assert_eq!(parsed.frontmatter.priority, Priority::Normal);
        assert!(parsed.frontmatter.due.is_none());
        assert!(parsed.frontmatter.recur.is_none());
        assert_eq!(parsed.body.trim(), "Body content here.");
    }

    #[test]
    fn parse_rejects_missing_delim() {
        let src = "id: x\ntitle: y\n";
        assert!(parse(src).is_err());
    }
}
