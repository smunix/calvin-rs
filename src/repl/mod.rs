//! Interactive REPL for the Calvin language.
//!
//! This module provides an interactive read-eval-print loop (REPL) for
//! evaluating Calvin expressions, similar to the hobbes `hi` program.

use crate::compiler::Compiler;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::path::PathBuf;

/// Configuration for the REPL.
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// The prompt string.
    pub prompt: String,
    /// The history file path.
    pub history_file: Option<PathBuf>,
    /// Whether to show types of results.
    pub show_types: bool,
    /// Whether to enable color output.
    pub color: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        ReplConfig {
            prompt: "> ".to_string(),
            history_file: Some(
                dirs_home().unwrap_or_else(|| PathBuf::from(".")).join(".calvin_history"),
            ),
            show_types: false,
            color: true,
        }
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Run the interactive REPL.
pub fn run(config: ReplConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut compiler = Compiler::new();
    let mut rl = DefaultEditor::new()?;

    // Load history
    if let Some(ref history_file) = config.history_file {
        let _ = rl.load_history(history_file);
    }

    println!("Calvin Interactive Shell");
    println!("Type :q to quit, :t <expr> for type info, :h for help");
    println!();

    loop {
        let readline = rl.readline(&config.prompt);
        match readline {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(&line);

                // Handle REPL commands
                if line.starts_with(':') {
                    match handle_command(&line, &mut compiler, &config) {
                        CommandResult::Continue => continue,
                        CommandResult::Quit => break,
                    }
                }

                // Handle definitions
                if line.starts_with("let ") && line.contains('=') && !line.contains(" in ") {
                    handle_definition(&line, &mut compiler);
                    continue;
                }

                // Evaluate expression
                match compiler.eval_str(&line) {
                    Ok(value) => {
                        println!("{}", value);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Bye!");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    if let Some(ref history_file) = config.history_file {
        let _ = rl.save_history(history_file);
    }

    Ok(())
}

enum CommandResult {
    Continue,
    Quit,
}

fn handle_command(line: &str, compiler: &mut Compiler, _config: &ReplConfig) -> CommandResult {
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    let cmd = parts[0];
    let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match cmd {
        ":q" | ":quit" => {
            println!("Bye!");
            return CommandResult::Quit;
        }
        ":h" | ":help" => {
            println!("Commands:");
            println!("  :q, :quit     Quit the REPL");
            println!("  :h, :help     Show this help");
            println!("  :t <expr>     Show the type of an expression");
            println!("  :env          Show all bindings");
            println!("  :load <file>  Load and evaluate a file");
            println!("  :reset        Reset the environment");
        }
        ":t" | ":type" => {
            if arg.is_empty() {
                eprintln!("Usage: :t <expression>");
            } else {
                // For now, just evaluate and show the type name
                match compiler.eval_str(arg) {
                    Ok(value) => {
                        println!("{} :: {}", value, value.type_name());
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
        }
        ":env" => {
            let names = compiler.bound_names();
            for name in names {
                println!("  {}", name);
            }
        }
        ":load" => {
            if arg.is_empty() {
                eprintln!("Usage: :load <filename>");
            } else {
                match std::fs::read_to_string(arg) {
                    Ok(contents) => {
                        for line in contents.lines() {
                            let line = line.trim();
                            if line.is_empty() || line.starts_with("--") {
                                continue;
                            }
                            if line.starts_with("let ") && line.contains('=') && !line.contains(" in ") {
                                handle_definition(line, compiler);
                            } else {
                                match compiler.eval_str(line) {
                                    Ok(value) => println!("{}", value),
                                    Err(e) => eprintln!("Error: {}", e),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading file: {}", e);
                    }
                }
            }
        }
        ":reset" => {
            *compiler = Compiler::new();
            println!("Environment reset.");
        }
        _ => {
            eprintln!("Unknown command: {}", cmd);
            eprintln!("Type :h for help");
        }
    }

    CommandResult::Continue
}

fn handle_definition(line: &str, compiler: &mut Compiler) {
    // Parse "let name = expr"
    let rest = line.strip_prefix("let ").unwrap_or(line);
    if let Some(eq_pos) = rest.find('=') {
        let name = rest[..eq_pos].trim();
        let expr = rest[eq_pos + 1..].trim();
        match compiler.define(name, expr) {
            Ok(()) => {
                println!("{} defined", name);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

use crate::compiler::value::Value;

/// Trait for REPL-compatible value display.
impl Value {
    /// Format a value for REPL display with optional type annotation.
    pub fn display_with_type(&self) -> String {
        format!("{} :: {}", self, self.type_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_config_default() {
        let config = ReplConfig::default();
        assert_eq!(config.prompt, "> ");
        assert!(!config.show_types);
    }
}
