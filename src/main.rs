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
    /// Create a new task with status and note
    Write {
        /// Task status (inbox, todo, doing, blocked, inreview, done)
        status: String,
        /// Task title/note
        note: String,
    },
    /// Declare session start for a task
    Doing {
        /// Task ID (8-char hash)
        id: String,
    },
    /// Mark task as reviewing with PR URL
    Reviewing {
        /// Task ID (8-char hash)
        id: String,
        /// Pull request URL
        pr_url: String,
    },
    /// List tasks (current state per ID)
    Get {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },
}

const VALID_STATUSES: &[&str] = &["inbox", "todo", "doing", "blocked", "inreview", "done"];

fn main() {
    let cli = Cli::parse();
    let store = TaskStore::default_path();
    let project = project::get_project();

    match cli.command {
        Commands::Write { status, note } => {
            if !VALID_STATUSES.contains(&status.as_str()) {
                eprintln!(
                    "Error: invalid status '{}'. Valid: {}",
                    status,
                    VALID_STATUSES.join(", ")
                );
                std::process::exit(1);
            }
            let id = gen_id();
            store.append(&TaskEntry {
                id: id.clone(),
                project,
                status,
                title: note,
                description: String::new(),
            });
            println!("TASK_ADD_{id}");
        }
        Commands::Doing { id } => {
            if !store.id_exists(&id) {
                eprintln!("Error: task '{id}' not found");
                std::process::exit(1);
            }
            let title = store.latest_title(&id);
            store.append(&TaskEntry {
                id: id.clone(),
                project,
                status: "doing".into(),
                title,
                description: String::new(),
            });
            println!("TASK_DOING_{id}");
        }
        Commands::Reviewing { id, pr_url } => {
            if !store.id_exists(&id) {
                eprintln!("Error: task '{id}' not found");
                std::process::exit(1);
            }
            let title = store.latest_title(&id);
            store.append(&TaskEntry {
                id: id.clone(),
                project,
                status: "inreview".into(),
                title,
                description: pr_url,
            });
            println!("TASK_REVIEWING_{id}");
        }
        Commands::Get { status } => {
            for task in store.current_tasks(status.as_deref()) {
                println!("{}", task.format_line());
            }
        }
    }
}
