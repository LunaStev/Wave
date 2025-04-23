fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rustc-link-lib=llvm-14");
    println!("cargo:rustc-link-search=native=/usr/lib/llvm-14/lib");
}