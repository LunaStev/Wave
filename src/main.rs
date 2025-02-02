mod lexer;
mod parser;
mod error;

use std::{env, fs, process};
use lexer::{Lexer, Token};
use crate::lexer::TokenType;
use crate::parser::{extract_body, extract_parameters, function};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("\x1b[31mUsage:\x1b[0m wave <command> [arguments]");
        eprintln!("\x1b[33mCommands:\x1b[0m");
        eprintln!("\x1b[34m  run <file>\x1b[0m    Execute the specified Wave file");
        eprintln!("\x1b[34m  --version\x1b[0m     Show the CLI version");
        process::exit(1);
    }

    match args[1].as_str() {
        "--version" | "-V" => {
            println!("{}", VERSION.color("2,161,47"));
            return;
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("\x1b[31mUsage:\x1b[0m wave run <file>");
                process::exit(1);
            }

            let file_path = &args[2];
            run_wave_file(file_path);
        }
        "help" => {
            println!("\x1b[33mOptions:\x1b[0m");
            println!("\x1b[34m      run <file>\x1b[0m       Run the Wave code.\n");

            println!("\x1b[33mCommands:\x1b[0m");
            println!("\x1b[34m      -V, --version\x1b[0m    Verified the version of the Wave compiler.\n");
            return;
        }
        _ => {
            eprintln!("\x1b[31mUnknown command:\x1b[0m {}", args[1]);
            eprintln!("\x1b[33mUse 'wave --version' or 'wave run <file>'\x1b[0m");
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
        .find(|token| matches!(token.token_type, TokenType::IDENTIFIER(_)))
        .map(|token| token.lexeme.clone())
        .unwrap_or_default();

    let params = extract_parameters(&tokens[..], 0, tokens.len());

    let mut peekable_tokens = tokens.iter().peekable();

    let body = extract_body(&mut peekable_tokens);

    let ast = function(function_name, params, body);

    eprintln!("AST:\n{:#?}", &ast);
}