//! `hi` - Calvin Interactive Shell
//!
//! An interactive interpreter for Calvin expressions, similar to the
//! hobbes `hi` program. Supports expression evaluation, variable
//! definitions, and REPL commands.

use clap::Parser;
use std::path::PathBuf;

/// Calvin Interactive Shell - evaluate expressions interactively.
#[derive(Parser, Debug)]
#[command(name = "hi", version, about = "Calvin Interactive Shell")]
struct Args {
    /// Evaluate an expression and exit.
    #[arg(short, long)]
    eval: Option<String>,

    /// Load a file before starting the REPL.
    #[arg(short, long)]
    load: Option<PathBuf>,

    /// Connect to a remote Calvin server.
    #[arg(short, long)]
    connect: Option<String>,

    /// Set the REPL prompt.
    #[arg(short, long, default_value = "> ")]
    prompt: String,

    /// Disable color output.
    #[arg(long)]
    no_color: bool,

    /// Show types of evaluated expressions.
    #[arg(short, long)]
    types: bool,

    /// Port for the built-in network REPL server.
    #[arg(short = 's', long)]
    serve: Option<u16>,
}

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // If --eval is provided, evaluate and exit
    if let Some(expr) = &args.eval {
        let mut compiler = calvin_rs::compiler::Compiler::new();
        match compiler.eval_str(expr) {
            Ok(value) => {
                if args.types {
                    println!("{}", value.display_with_type());
                } else {
                    println!("{}", value);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // If --serve is provided, start a network server in the background
    if let Some(port) = args.serve {
        let config = calvin_rs::net::server::ServerConfig {
            port,
            ..Default::default()
        };
        let server = calvin_rs::net::server::Server::new(config);
        std::thread::spawn(move || {
            if let Err(e) = server.start() {
                eprintln!("Server error: {}", e);
            }
        });
        println!("Network REPL server started on port {}", port);
    }

    // Start the REPL
    let config = calvin_rs::repl::ReplConfig {
        prompt: args.prompt,
        show_types: args.types,
        color: !args.no_color,
        ..Default::default()
    };

    if let Err(e) = calvin_rs::repl::run(config) {
        eprintln!("REPL error: {}", e);
        std::process::exit(1);
    }
}
