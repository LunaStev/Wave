// SPDX-License-Identifier: MPL-2.0
// Build with rustc before Cargo/LLVM provisioning; use the compiler's inspector.
#[path = "../llvm/src/msvc/coff.rs"]
mod coff;

fn main() {
    let mut args = std::env::args().skip(1);
    let target = args
        .next()
        .expect("usage: check_msvc_inputs TARGET [--all-members] FILE...");
    let mut all_members = false;
    let mut count = 0;
    for file in args {
        if file == "--all-members" {
            all_members = true;
            continue;
        }
        let path = std::path::Path::new(&file);
        let result = if all_members {
            coff::validate_file_all_members(path, &target)
        } else {
            coff::validate_file(path, &target)
        };
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        println!("Verified {target}: {}", path.display());
        count += 1;
    }
    if count == 0 {
        eprintln!("at least one COFF input is required");
        std::process::exit(1);
    }
}
