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
    // Internal: used by elevated subprocess to clean system PATH entries
    #[arg(long, hide = true)]
    clean_system_path: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available vendors and versions for a category (e.g. java, maven)
    List {
        category: Option<String>,
    },
    /// Installs the specified version for a category and optional vendor (e.g java corretto or maven), use 'list' to see options
    Install {
        /// Category, optional vendor, and version
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    // Elevated subprocess entrypoint
    if let Some(entries) = cli.clean_system_path {
        env_update::clean_system_path_elevated(&entries);
        return;
    }

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
        Some(Commands::Install { args }) => {
            match args.as_slice() {
                [] => {
                    eprintln!("Usage: cauldron install <category> [<vendor>] <version>\nExample: cauldron install java corretto 21 or cauldron install maven 3.4.0");
                }
                [cat] => {
                    eprintln!("Usage: cauldron install {} [<vendor>] <version>", cat);
                    eprintln!("Run 'cauldron list {}' to see available vendors and versions.", cat);
                }
                [cat, ver] => {
                    let vendor = match crate::fetch::fetch_tool_config(&cat) {
                        Ok(config) => config.default_vendor.unwrap_or(cat.clone()),
                        Err(e) => {
                            eprintln!("Failed to fetch config for {}: {}", cat, e);
                            std::process::exit(1);
                        }
                    };
                    commands::install::run(&cat, &vendor, &ver);
                }
                [cat, ven, ver] => {
                    commands::install::run(&cat, &ven, &ver);
                }
                _ => {
                    eprintln!("Too many arguments. Usage: cauldron install <category> [<vendor>] <version>");
                }
            }
        }
    }
}