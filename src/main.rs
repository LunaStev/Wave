mod lexer;
mod parser;
mod error;

use std::{env, fs, process};
use colorex::Colorize;
use lexer::{Lexer};
use crate::lexer::TokenType;
use crate::parser::{extract_body, extract_parameters, function};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("{} {}",
                  "Usage:".color("255,71,71"),
                  "wave <command> [arguments]");

        eprintln!("{}",
                  "Commands:".color("145,161,2"));

        eprintln!("  {}    {}",
                  "run <file>".color("38,139,235"),
                  "Execute the specified Wave file");

        eprintln!("  {}     {}",
                  "--version".color("38,139,235"),
                  "Show the CLI version");
        process::exit(1);
    }

    match args[1].as_str() {
        "--version" | "-V" => {
            println!("{}",
                     VERSION.color("2,161,47"));
            return;
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("{} {}",
                          "Usage:".color("255,71,71"),
                          "wave run <file>");
                process::exit(1);
            }

            let file_path = &args[2];
            run_wave_file(file_path);
        }
        "help" => {
            println!("{}", "Options:".color("145,161,2"));
            println!("      {}       {}\n",
                     "run <file>".color("38,139,235"),
                     "Run the Wave code.");

            println!("{}", "Commands:".color("145,161,2"));
            println!("      {}    {}\n",
                     "-V, --version".color("38,139,235"),
                     "Verified the version of the Wave compiler.");
            return;
        }
        _ => {
            eprintln!("{} {}",
                      "Unknown command:".color("255,71,71"),
                      args[1]);
            eprintln!("{}",
                      "Use 'wave --version' or 'wave run <file>'".color("145,161,2"));
            process::exit(1);
        }
    }
}

fn run_wave_file(file_path: &str) {
    let code = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file {}: {}", file_path, err);
            process::exit(1);
        }
    };

    let mut lexer = Lexer::new(code.as_str());

    let tokens = lexer.tokenize();
    eprintln!("Tokens: \n{:#?}", &tokens);

    let function_name = tokens
        .iter()
        .find(|token| matches!(token.token_type, TokenType::Identifier(_)))
        .map(|token| token.lexeme.clone())
        .unwrap_or_default();

    let params = extract_parameters(&tokens[..], 0, tokens.len());

    let mut peekable_tokens = tokens.iter().peekable();

    let body = extract_body(&mut peekable_tokens);

    let ast = function(function_name, params, body);

    eprintln!("AST:\n{:#?}", &ast);
}