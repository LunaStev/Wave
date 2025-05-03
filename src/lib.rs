pub mod runner;

use std::path::Path;

pub unsafe fn compile_and_run(path: &Path) {
    runner::run_wave_file(path);
}
