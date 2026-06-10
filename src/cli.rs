use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "todai", version, about = "AI-powered cross-machine todo system")]
pub struct Cli {
    /// Override the todai store path (default: $TODAI_HOME or ~/.todai)
    #[arg(long, global = true)]
    pub path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Bootstrap a new todai store (idempotent)
    Init,

    /// Create a new todo
    Add {
        /// Todo title
        title: String,
        /// Context, e.g. "work:staff" (default: from config)
        #[arg(short, long)]
        context: Option<String>,
        /// Due date/time. Accepts YYYY-MM-DD, YYYY-MM-DDTHH:MM, or 'today'/'tomorrow'
        #[arg(short, long)]
        due: Option<String>,
        /// Priority: low | normal | high | urgent
        #[arg(short, long)]
        priority: Option<String>,
        /// Add a reminder. Same datetime format as --due. Repeatable.
        #[arg(long)]
        remind: Vec<String>,
        /// Add a tag. Repeatable.
        #[arg(long)]
        tag: Vec<String>,
        /// Recurrence: daily | weekly | monthly | yearly
        #[arg(long)]
        recur: Option<String>,
    },

    /// List todos with optional filters
    List {
        /// Filter by context (matches the prefix, e.g. "work" matches "work:staff")
        #[arg(short, long)]
        context: Option<String>,
        /// Filter by due window: today | this-week | overdue
        #[arg(short, long)]
        due: Option<String>,
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
        /// Include done/cancelled todos (default: pending only)
        #[arg(long)]
        all: bool,
        /// Output as JSON for scripting / agent consumption
        #[arg(long)]
        json: bool,
    },

    /// What's on fire today: due today + overdue + reminders firing today
    Today {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show a single todo (id, slug, or fuzzy match)
    Show {
        /// Todo identifier (id, slug, or fuzzy substring)
        id: String,
        /// Follow the prev_id chain for recurring tasks
        #[arg(long)]
        history: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Mark a todo as done; spawns the next instance for recurring tasks
    Done {
        /// Todo identifier (id, slug, or fuzzy substring)
        id: String,
    },

    /// Open a todo in $VISUAL/$EDITOR; re-validated before saving
    Edit {
        /// Todo identifier (id, slug, or fuzzy substring)
        id: String,
    },

    /// Sweep done/cancelled todos older than archive_after_days into .archive/
    Archive {
        /// Show what would be archived without moving anything
        #[arg(long)]
        dry_run: bool,
    },

    /// Mark a reminder as notified (used by the agent after sending)
    Notified {
        /// Todo identifier
        id: String,
        /// Index of the reminder in the remind array (0-based)
        #[arg(long, default_value_t = 0)]
        reminder: usize,
    },

    /// Push pending reminders out by a duration, e.g. 30m, 2h, 1d
    Snooze {
        /// Todo identifier
        id: String,
        /// Duration to push reminders out (m/h/d/w units)
        #[arg(long = "for")]
        for_: String,
    },
}
