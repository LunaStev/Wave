use std::process::Command;
use std::env;
use std::path::Path;
use dotenv::dotenv;

fn main() {
    dotenv().ok();

    let out_dir = env::var("OUT_DIR").expect("Failed to get OUT_DIR");
    let project_root = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR"));

    let go_version = env::var("GO_VERSION").unwrap_or_else(|_| "1.22.0".to_string());
    println!("Using Go version: {}", go_version);
}

fn build_c_cpp(project_root: &Path, out_dir: &str) {

}

fn build_go(project_root: &Path, out_dir: &str) {

}

fn build_kotlin(project_root: &Path, out_dir: &str) {

}

fn build_carbon(project_root: &Path, out_dir: &str) {

}

fn build_haskell(project_root: &Path, out_dir: &str) {

}

fn build_lisp(project_root: &Path, out_dir: &str) {

}

fn build_dart(project_root: &Path, out_dir: &str) {

}

fn build_ml(project_root: &Path, out_dir: &str) {

}

fn build_nim(project_root: &Path, out_dir: &str) {

}

fn build_python(project_root: &Path, out_dir: &str) {

}

fn build_mojo(project_root: &Path, out_dir: &str) {

}

fn build_zig(project_root: &Path, out_dir: &str) {

}


