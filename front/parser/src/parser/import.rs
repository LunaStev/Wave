use std::collections::HashSet;
use std::path::{Path, PathBuf};
use crate::ast::ASTNode;
use crate::parse;
use lexer::Lexer;

pub fn local_import(path: &str, already_imported: &mut HashSet<String>, base_dir: &Path) -> Option<Vec<ASTNode>> {
    if already_imported.contains(path) {
        return Some(vec![]);
    }

    already_imported.insert(path.to_string());

    let target_file_name = format!("{}.wave", path);

    let found_path = find_wave_file_recursive(base_dir, &target_file_name)?;

    let content = std::fs::read_to_string(&found_path).ok()?;
    let mut lexer = Lexer::new(&content);
    let tokens = lexer.tokenize();
    let ast = parse(&tokens)?;

    Some(ast)
}

fn find_wave_file_recursive(dir: &Path, target_file_name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(found) = find_wave_file_recursive(&path, target_file_name) {
                return Some(found);
            }
        } else if path.is_file() {
            if path.file_name()?.to_str()? == target_file_name {
                return Some(path);
            }
        }
    }
    None
}