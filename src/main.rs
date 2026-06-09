use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod config;
mod context;
mod store;
mod time_util;
mod todo;

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    let root = config::resolve_root(args.path.as_deref())?;

    match args.command {
        cli::Command::Init => commands::init::run(&root)?,
        cli::Command::Add {
            title,
            context,
            due,
            priority,
            remind,
            tag,
            recur,
        } => commands::add::run(&root, title, context, due, priority, remind, tag, recur)?,
        cli::Command::List {
            context,
            due,
            tag,
            all,
            json,
        } => commands::list::run(&root, context, due, tag, all, json)?,
        cli::Command::Today { json } => commands::today::run(&root, json)?,
        cli::Command::Show { id, history, json } => commands::show::run(&root, id, history, json)?,
        cli::Command::Done { id } => commands::done::run(&root, id)?,
        cli::Command::Archive { dry_run } => commands::archive::run(&root, dry_run)?,
        cli::Command::Notified { id, reminder } => commands::notified::run(&root, id, reminder)?,
        cli::Command::Snooze { id, for_ } => commands::snooze::run(&root, id, for_)?,
    }
    Ok(())
}
