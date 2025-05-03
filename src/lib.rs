pub mod runner;

use std::path::Path;
use colorex::Colorize;

pub unsafe fn compile_and_run(path: &Path) {
    runner::run_wave_file(path);
}

pub fn version_wave() {
    println!("{}", version::version().color("2,161,47"));
}
