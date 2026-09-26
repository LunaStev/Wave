// This file is part of the Wave language project.
// SPDX-License-Identifier: MPL-2.0

//! Protect every compiler input before the first artifact is written.
use crate::errors::CliError;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn same_file(source: &Path, output: &Path) -> Result<bool, CliError> {
    let source_path = fs::canonicalize(source)?;
    let output_path = match fs::canonicalize(output) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if source_path == output_path {
        return Ok(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let a = fs::metadata(source_path)?;
        let b = fs::metadata(output_path)?;
        Ok(a.dev() == b.dev() && a.ino() == b.ino())
    }
    #[cfg(windows)]
    {
        Ok(windows_identity(&source_path)? == windows_identity(&output_path)?)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(false)
    }
}

pub(crate) fn validate(
    inputs: &[PathBuf],
    outputs: &[(PathBuf, Option<&Path>)],
) -> Result<(), CliError> {
    for (output, passthrough) in outputs {
        // An identity copy is a no-op, including symlink and hard-link aliases.
        if let Some(source) = passthrough {
            if same_file(source, output)? {
                continue;
            }
        }
        for input in inputs {
            if same_file(input, output)? {
                return Err(CliError::usage(format!(
                    "output '{}' aliases compiler input '{}'; choose a separate output path",
                    output.display(),
                    input.display()
                )));
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn windows_identity(path: &Path) -> std::io::Result<(u32, [u32; 2])> {
    use std::os::windows::io::AsRawHandle;
    #[repr(C)]
    struct Information {
        attributes: u32,
        times: [u32; 6],
        volume: u32,
        size: [u32; 2],
        links: u32,
        index: [u32; 2],
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandle(handle: *mut std::ffi::c_void, info: *mut Information)
            -> i32;
    }
    let file = fs::File::open(path)?;
    let mut info = std::mem::MaybeUninit::<Information>::uninit();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let info = unsafe { info.assume_init() };
    Ok((info.volume, info.index))
}
