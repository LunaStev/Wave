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
