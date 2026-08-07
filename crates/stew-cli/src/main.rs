use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result};

#[derive(Parser)]
#[command(name = "stew", version, about = "The Stew language compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a stew file directly through an interpreter
    Run {
        /// The path to the file to run
        file: String,
    },
    /// Parse a file and print the AST (useful for debugging)
    Parse {
        /// The path to the file to parse
        file: String,

        /// Print the raw Tree-sitter CST instead of the core AST
        #[arg(long)]
        cst: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run { file } => execute_run(file),
        Commands::Parse { file, cst } => execute_parse(file, cst),
    }
}

fn execute_parse(file: String, print_cst: bool) -> Result<()> {
    let source = std::fs::read_to_string(&file).into_diagnostic()?;
    if print_cst {
        let tree = stew_parser::parse_cst(&source)
            .ok_or_else(|| miette::miette!("Failed to parse source"))?;
        let raw_sexp = tree.root_node().to_sexp();
        println!("{}", stew_parser::format_sexp(&raw_sexp));
    } else {
        let program = stew_parser::parse(source)?;

        // {:#?} means to format it with newlines and indentation
        println!("{:#?}", program);
    }

    Ok(())
}

fn execute_run(_file: String) -> Result<()> {
    println!("Interpreter not yet implemented!");
    Ok(())
}
