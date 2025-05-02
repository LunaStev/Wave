use std::{env, fs, process, process::Command};
use std::path::Path;
use colorex::Colorize;
use lexer::Lexer;
use llvm_temporary::llvm_temporary::llvm_backend::*;
use llvm_temporary::llvm_temporary::llvm_codegen::*;
use ::parser::*;
use ::parser::ast::{ASTNode, StatementNode};
use ::parser::import::local_import;

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
        "run" => unsafe {
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

unsafe fn run_wave_file(file_path: &str) {
    let code = fs::read_to_string(file_path).expect("Failed to read file");

    let mut lexer = Lexer::new(&code);
    let tokens = lexer.tokenize();

    let mut ast = parse(&tokens).expect("Failed to parse Wave code");

    let file_path = Path::new(file_path);
    let base_dir = file_path.parent().unwrap();

    let mut already_imported = HashSet::new();
    let mut extended_ast = vec![];

    for node in &ast {
        if let ASTNode::Statement(StatementNode::Import(path)) = node {
            if !path.starts_with("std::") {
                if let Some(mut imported_nodes) = local_import(&path, &mut already_imported, base_dir) {
                    extended_ast.append(&mut imported_nodes);
                } else {
                    eprintln!("❌ Failed to import '{}'", path);
                    process::exit(1);
                }
            }
        } else {
            extended_ast.push(node.clone());
        }
    }

    ast = extended_ast;

    // println!("{}\n", code);
    // println!("AST:\n{:#?}", ast);

    let ir = generate_ir(&ast);
    let path = Path::new(file_path);
    let file_stem = path.file_stem().unwrap().to_str().unwrap();
    let machine_code_path = compile_ir_to_machine_code(&ir, file_stem);

    if machine_code_path.is_empty() {
        eprintln!("Failed to generate machine code");
        return;
    }

    let output = Command::new(machine_code_path)
        .output()
        .expect("Failed to execute machine code");

    // println!("Generated LLVM IR:\n{}", ir);
    println!("{}", String::from_utf8_lossy(&output.stdout));
}
