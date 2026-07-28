use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result};

#[derive(Parser)]
#[command(name = "loom", version, about = "The Loom language compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a loom file directly through an interpreter
    Run {
        /// The path to the file to run
        file: String,
    },
    /// Parse a file and print the AST (useful for debugging)
    Parse {
        file: String,
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run { file } => execute_run(file),
        Commands::Parse { file } => execute_parse(file)
    }
}

fn execute_parse(file: String) -> Result<()> {
    let source = std::fs::read_to_string(&file).into_diagnostic()?;
    let tree = loom_parser::parse(source)?;

    let raw_sexp = tree.root_node().to_sexp();
    println!("{}", loom_parser::format_sexp(&raw_sexp));
    
    Ok(())
}

fn execute_run(file: String) -> Result<()> {
    println!("Interpreter not yet implemented!");
    Ok(())
}