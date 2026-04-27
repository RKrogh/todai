use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::config;
use crate::context::Context;

pub fn run(root: &Path) -> Result<()> {
    let already_existed = root.join(config::CONFIG_RELATIVE_PATH).exists();

    fs::create_dir_all(root)?;
    fs::create_dir_all(root.join(".todai"))?;
    fs::create_dir_all(root.join(".todai/logs"))?;

    let cfg_path = config::write_default(root)?;
    let stignore_path = config::write_stignore(root)?;

    let cfg = config::load(root)?;
    let mut scaffolded = Vec::new();
    for ctx_str in &cfg.contexts.allowed {
        if let Ok(ctx) = Context::parse(ctx_str) {
            let dir = root.join(ctx.to_relative_path());
            if !dir.exists() {
                fs::create_dir_all(&dir)?;
                scaffolded.push(ctx_str.clone());
            }
        }
    }

    if already_existed {
        println!("{}", "todai store already initialized".yellow());
    } else {
        println!("{}", "todai store initialized".green().bold());
    }
    println!("  root        {}", root.display());
    println!("  config      {}", cfg_path.display());
    println!("  stignore    {}", stignore_path.display());
    if !scaffolded.is_empty() {
        println!("  contexts    {}", scaffolded.join(", "));
    }

    println!();
    println!("{}", "Layout note:".dimmed());
    println!(
        "  {} holds todai metadata (config, logs). Safe to ignore; the CLI",
        ".todai/ inside the store".dimmed()
    );
    println!("  skips it when listing. Todo files live in context folders alongside it.");

    println!();
    println!("{}", "Next steps:".bold());
    println!("  1. Share this folder with the Pi via Syncthing");
    println!("  2. Set ANTHROPIC_API_KEY and HA_TOKEN in your environment (or HA secrets)");
    println!("  3. Try: todai add \"My first todo\" --context private:inbox --due today");

    Ok(())
}
