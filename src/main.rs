mod schema;
mod validate;
mod index;
mod supersede;
mod dedupe;
mod stale;
mod gc;
mod migrate;
mod show;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "memctl", about = "Claude Code memory hygiene CLI")]
struct Cli {
    /// Path to memory directory
    #[arg(long, default_value = ".")]
    path: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate all memory file schemas
    Validate,
    /// Generate MEMORY.md index
    Index,
    /// Mark a memory as superseded
    Supersede {
        /// File being superseded
        old: String,
        /// Replacement file
        new: String,
    },
    /// Find near-duplicate memories
    Dedupe,
    /// List expired memories
    Stale,
    /// Archive expired/superseded memories
    Gc {
        /// Show what would be archived without doing it
        #[arg(long)]
        dry_run: bool,
    },
    /// Backfill missing frontmatter fields
    Migrate {
        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Show a memory by name
    Show {
        /// Name or filename to look up
        query: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let dir = &cli.path;

    let result = match cli.command {
        Command::Validate => validate::run(dir).map(|ok| if ok { 0 } else { 1 }),
        Command::Index => index::run(dir).map(|_| 0),
        Command::Supersede { old, new } => supersede::run(dir, &old, &new).map(|_| 0),
        Command::Dedupe => dedupe::run(dir).map(|found| if found { 1 } else { 0 }),
        Command::Stale => stale::run(dir).map(|found| if found { 1 } else { 0 }),
        Command::Gc { dry_run } => gc::run(dir, dry_run).map(|_| 0),
        Command::Migrate { dry_run } => migrate::run(dir, dry_run).map(|_| 0),
        Command::Show { query } => show::run(dir, &query).map(|_| 0),
    };

    match result {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("error: {:#}", e);
            process::exit(2);
        }
    }
}
