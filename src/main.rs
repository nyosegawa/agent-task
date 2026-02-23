mod init;
mod lang;
mod project;
mod store;

use clap::{Parser, Subcommand};
use store::{TaskEntry, TaskStore, gen_id};

#[derive(Parser)]
#[command(name = "task", about = "Lightweight task management for coding agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new task
    Create {
        /// Task title
        title: String,
        /// Task description
        description: Option<String>,
        /// Initial status (default: todo)
        #[arg(long, default_value = "todo")]
        status: String,
    },
    /// Update task status
    Update {
        /// Task ID (8-char hex)
        id: String,
        /// New status
        status: String,
        /// Transition note (block reason, PR URL, etc.)
        note: Option<String>,
        /// Update description
        #[arg(long)]
        description: Option<String>,
    },
    /// List tasks
    List {
        /// Filter by status
        status: Option<String>,
        /// Show all projects (default: current project only)
        #[arg(long)]
        all: bool,
    },
    /// Show task detail and state transition history
    Get {
        /// Task ID (8-char hex)
        id: String,
    },
    /// Set or show expected language for the current project
    Lang {
        /// Language code (e.g., ja, en). Omit to show current setting.
        code: Option<String>,
        /// Remove language setting
        #[arg(long)]
        unset: bool,
    },
    /// Inject instruction snippet into agent config files
    Init {
        /// Inject into global config files instead of project-local
        #[arg(long)]
        global: bool,
    },
}

fn validate_length(value: &str, field: &str, max: usize) {
    if value.chars().count() > max {
        eprintln!(
            "Error: {field} exceeds {max} chars ({} chars given)",
            value.chars().count()
        );
        std::process::exit(1);
    }
}

fn main() {
    let cli = Cli::parse();
    let store = TaskStore::default_path();
    let project = project::get_project();

    match cli.command {
        Commands::Create {
            title,
            description,
            status,
        } => {
            validate_length(&title, "title", 50);
            if let Some(ref d) = description {
                validate_length(d, "description", 500);
            }
            let lang_config = lang::LangConfig::new(store.lang_config_path());
            if let Some(expected) = lang_config.get(&project) {
                if let Err(e) = lang::validate_language(&title, &expected) {
                    eprintln!("Error: title {e}");
                    std::process::exit(1);
                }
                if let Some(ref d) = description
                    && let Err(e) = lang::validate_language(d, &expected)
                {
                    eprintln!("Error: description {e}");
                    std::process::exit(1);
                }
            }
            let id = gen_id();
            store.append(&TaskEntry::new(
                id.clone(),
                project,
                status,
                title,
                description.unwrap_or_default(),
                String::new(),
            ));
            println!("TASK_ADD_{id}");
        }
        Commands::Update {
            id,
            status,
            note,
            description,
        } => {
            if !store.id_exists(&id) {
                eprintln!("Error: task '{id}' not found");
                std::process::exit(1);
            }
            if let Some(ref n) = note {
                validate_length(n, "note", 200);
            }
            if let Some(ref d) = description {
                validate_length(d, "description", 500);
            }
            let prev = store.latest_entry(&id).unwrap();
            let new_description = description.unwrap_or(prev.description);
            store.append(&TaskEntry::new(
                id.clone(),
                project,
                status.clone(),
                prev.title,
                new_description,
                note.unwrap_or_default(),
            ));
            println!("TASK_{}_{id}", status.to_uppercase());
        }
        Commands::List { status, all } => {
            let project_filter = if all { None } else { Some(project.as_str()) };
            let tasks = store.current_tasks(project_filter, status.as_deref());
            if tasks.is_empty() {
                return;
            }
            println!("{:<10} {:<8} {:<24} TITLE", "ID", "STATUS", "PROJECT");
            for task in tasks {
                println!(
                    "{:<10} {:<8} {:<24} {}",
                    task.id, task.status, task.project, task.title
                );
            }
        }
        Commands::Get { id } => {
            let entries = store.entries_for_id(&id);
            if entries.is_empty() {
                eprintln!("Error: task '{id}' not found");
                std::process::exit(1);
            }
            let latest = entries.last().unwrap();
            println!("{} | {} | {}", latest.id, latest.project, latest.title);
            if !latest.description.is_empty() {
                for line in latest.description.lines() {
                    println!("  {line}");
                }
                println!();
            }
            for entry in &entries {
                if entry.note.is_empty() {
                    println!("  {:<28} {}", entry.ts, entry.status);
                } else {
                    let note_display: String = entry
                        .note
                        .lines()
                        .enumerate()
                        .map(|(i, l)| {
                            if i == 0 {
                                l.to_string()
                            } else {
                                format!("\n{:>42}{l}", "")
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    println!("  {:<28} {:<10} {}", entry.ts, entry.status, note_display);
                }
            }
        }
        Commands::Lang { code, unset } => {
            let lang_config = lang::LangConfig::new(store.lang_config_path());
            if unset {
                lang_config.unset(&project);
                println!("Language setting removed.");
            } else if let Some(code) = code {
                if lang::resolve_lang(&code).is_none() {
                    eprintln!("Error: unsupported language code: '{code}'");
                    std::process::exit(1);
                }
                lang_config.set(&project, &code);
                println!("Language set to '{code}'.");
            } else {
                match lang_config.get(&project) {
                    Some(lang) => println!("{lang}"),
                    None => println!("Language not set."),
                }
            }
        }
        Commands::Init { global } => {
            let result = init::run_init(global);
            if !result.injected.is_empty() {
                for path in &result.injected {
                    println!("Injected: {path}");
                }
            } else if result.up_to_date > 0 {
                println!("Already up-to-date.");
            } else if !result.candidates.is_empty() {
                println!(
                    "No instruction files found. Create one of these and run again:\n  {}",
                    result.candidates.join(", ")
                );
            } else {
                println!("Already up-to-date.");
            }
        }
    }
}
