use std::process::Command;
use std::env;
use std::path::Path;
use dotenv::dotenv;

fn main() {
    dotenv().ok();

    let out_dir = env::var("OUT_DIR").expect("Failed to get OUT_DIR");
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
    let project_root = Path::new(&cargo_manifest_dir);

    if cfg!(windows) {
        build_windows(&project_root, &out_dir);
    } else {
        build_linux(&project_root, &out_dir);
    }

    let go_version = env::var("GO_VERSION").unwrap_or_else(|_| "1.22.0".to_string());
    println!("Using Go version: {}", go_version);
}

fn build_windows(project_root: &Path, out_dir: &str) {
    build_c_cpp_windows(project_root, out_dir);
    build_go_windows(project_root, out_dir);
    build_kotlin_windows(project_root, out_dir);
    build_carbon_windows(project_root, out_dir);
    build_haskell_windows(project_root, out_dir);
    build_lisp_windows(project_root, out_dir);
    build_dart_windows(project_root, out_dir);
    build_ml_windows(project_root, out_dir);
    build_nim_windows(project_root, out_dir);
    build_python_windows(project_root, out_dir);
    build_mojo_windows(project_root, out_dir);
    build_zig_windows(project_root, out_dir);
}

fn build_linux(project_root: &Path, out_dir: &str) {
    build_c_cpp_linux(project_root, out_dir);
    build_go_linux(project_root, out_dir);
    build_kotlin_linux(project_root, out_dir);
    build_carbon_linux(project_root, out_dir);
    build_haskell_linux(project_root, out_dir);
    build_lisp_linux(project_root, out_dir);
    build_dart_linux(project_root, out_dir);
    build_ml_linux(project_root, out_dir);
    build_nim_linux(project_root, out_dir);
    build_python_linux(project_root, out_dir);
    build_mojo_linux(project_root, out_dir);
    build_zig_linux(project_root, out_dir);
}

fn build_c_cpp_windows(project_root: &Path, out_dir: &str) {

}

fn build_c_cpp_linux(project_root: &Path, out_dir: &str) {

}

fn build_go_windows(project_root: &Path, out_dir: &str) {

}

fn build_go_linux(project_root: &Path, out_dir: &str) {

}

fn build_kotlin_windows(project_root: &Path, out_dir: &str) {

}

fn build_kotlin_linux(project_root: &Path, out_dir: &str) {

}

fn build_carbon_windows(project_root: &Path, out_dir: &str) {

}

fn build_carbon_linux(project_root: &Path, out_dir: &str) {

}

fn build_haskell_windows(project_root: &Path, out_dir: &str) {

}

fn build_haskell_linux(project_root: &Path, out_dir: &str) {

}

fn build_lisp_windows(project_root: &Path, out_dir: &str) {

}

fn build_lisp_linux(project_root: &Path, out_dir: &str) {

}

fn build_dart_windows(project_root: &Path, out_dir: &str) {

}

fn build_dart_linux(project_root: &Path, out_dir: &str) {

}

fn build_ml_windows(project_root: &Path, out_dir: &str) {

}

fn build_ml_linux(project_root: &Path, out_dir: &str) {

}

fn build_nim_windows(project_root: &Path, out_dir: &str) {

}

fn build_nim_linux(project_root: &Path, out_dir: &str) {

}

fn build_python_windows(project_root: &Path, out_dir: &str) {

}

fn build_python_linux(project_root: &Path, out_dir: &str) {

}

fn build_mojo_windows(project_root: &Path, out_dir: &str) {

}

fn build_mojo_linux(project_root: &Path, out_dir: &str) {

}

fn build_zig_windows(project_root: &Path, out_dir: &str) {

}

fn build_zig_linux(project_root: &Path, out_dir: &str) {

}