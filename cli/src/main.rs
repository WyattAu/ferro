use clap::{Parser, Subcommand};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "ferro")]
#[command(about = "Ferro CLI - CalDAV/CardDAV client")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Calendar operations
    Calendars {
        #[command(subcommand)]
        command: CalendarCommands,
    },
    /// Event operations
    Events {
        #[command(subcommand)]
        command: EventCommands,
    },
    /// Contact operations
    Contacts {
        #[command(subcommand)]
        command: ContactCommands,
    },
    /// Configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum CalendarCommands {
    /// List calendars
    List,
    /// Create calendar
    Create {
        /// Calendar name
        #[arg(short, long)]
        name: String,
    },
    /// Delete calendar
    Delete {
        /// Calendar ID
        #[arg(short, long)]
        id: String,
    },
}

#[derive(Subcommand)]
enum EventCommands {
    /// List events
    List {
        /// Calendar ID
        #[arg(short, long)]
        calendar: String,
    },
    /// Create event
    Create {
        /// Calendar ID
        #[arg(short, long)]
        calendar: String,
        /// Event summary
        #[arg(short, long)]
        summary: String,
        /// Start time
        #[arg(short, long)]
        start: String,
        /// End time
        #[arg(short, long)]
        end: String,
    },
}

#[derive(Subcommand)]
enum ContactCommands {
    /// List contacts
    List,
    /// Create contact
    Create {
        /// Contact name
        #[arg(short, long)]
        name: String,
        /// Email address
        #[arg(short, long)]
        email: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get config value
    Get {
        /// Config key
        key: String,
    },
    /// Set config value
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Calendars { command } => match command {
            CalendarCommands::List => {
                println!("Listing calendars...");
            }
            CalendarCommands::Create { name } => {
                println!("Creating calendar: {}", name);
            }
            CalendarCommands::Delete { id } => {
                println!("Deleting calendar: {}", id);
            }
        },
        Commands::Events { command } => match command {
            EventCommands::List { calendar } => {
                println!("Listing events for calendar: {}", calendar);
            }
            EventCommands::Create {
                calendar,
                summary,
                start,
                end,
            } => {
                println!("Creating event: {} in calendar {}", summary, calendar);
                println!("  Start: {}", start);
                println!("  End: {}", end);
            }
        },
        Commands::Contacts { command } => match command {
            ContactCommands::List => {
                println!("Listing contacts...");
            }
            ContactCommands::Create { name, email } => {
                println!("Creating contact: {}", name);
                if let Some(email) = email {
                    println!("  Email: {}", email);
                }
            }
        },
        Commands::Config { command } => match command {
            ConfigCommands::Get { key } => {
                println!("Getting config: {}", key);
            }
            ConfigCommands::Set { key, value } => {
                println!("Setting config: {} = {}", key, value);
            }
        },
    }
}
