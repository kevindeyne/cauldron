mod cache;
mod commands;
mod fetch;
mod model;
mod util;
mod unpack;
mod junction_setup;
mod env_update;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cauldron", about = "Cauldron SDK Manager", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available vendors and versions for a category (e.g. java, maven)
    List {
        category: Option<String>,
    },
    /// Print the download URL for a package
    Install {
        category: Option<String>,
        vendor: Option<String>,
        version: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => {
            Cli::parse_from(["cauldron", "--help"]);
        }
        Some(Commands::List { category: None }) => {
            eprintln!("Usage: cauldron list <category>\nExample: cauldron list java");
        }
        Some(Commands::List { category: Some(cat) }) => {
            commands::list::run(&cat);
        }
        Some(Commands::Install { category: None, .. }) => {
            eprintln!("Usage: cauldron install <category> <vendor> <version>\nExample: cauldron install java corretto 21");
        }
        Some(Commands::Install { category: Some(cat), vendor: None, .. }) => {
            eprintln!("Usage: cauldron install {} <vendor> <version>", cat);
            eprintln!("Run 'cauldron list {}' to see available vendors and versions.", cat);
        }
        Some(Commands::Install { category: Some(cat), vendor: Some(v), version: None }) => {
            eprintln!("Usage: cauldron install {} {} <version>", cat, v);
            eprintln!("Run 'cauldron list {}' to see available versions.", cat);
        }
        Some(Commands::Install { category: Some(cat), vendor: Some(v), version: Some(ver) }) => {
            commands::install::run(&cat, &v, &ver);
        }
    }
}