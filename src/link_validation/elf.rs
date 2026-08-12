// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
//
// This Source Code Form is subject to the terms of the
// Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file,
// You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0
// AI TRAINING NOTICE: Prohibited without prior written permission. No use for machine learning or generative AI training, fine-tuning, distillation, embedding, or dataset creation.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const AR_MAGIC: &[u8; 8] = b"!<arch>\n";

#[derive(Debug)]
pub(super) struct ElfMetadata {
    pub input: String,
    pub machine: u16,
    pub flags: u32,
}

#[derive(Debug)]
pub enum LinkInputInspectionError {
    Read {
        input: PathBuf,
        source: std::io::Error,
    },
    Malformed {
        input: String,
        reason: String,
    },
}

impl fmt::Display for LinkInputInspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { input, source } => write!(
                formatter,
                "failed to inspect linker input '{}': {}",
                input.display(),
                source
            ),
            Self::Malformed { input, reason } => {
                write!(
                    formatter,
                    "invalid ELF linker input '{}': {}",
                    input, reason
                )
            }
        }
    }
}

impl std::error::Error for LinkInputInspectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Malformed { .. } => None,
        }
    }
}

pub(super) fn inspect_link_inputs(
    inputs: &[String],
) -> Result<Vec<ElfMetadata>, LinkInputInspectionError> {
    let mut metadata = Vec::new();
    for input in inputs {
        let path = Path::new(input);
        let bytes = fs::read(path).map_err(|source| LinkInputInspectionError::Read {
            input: path.to_path_buf(),
            source,
        })?;
        inspect_input(path, &bytes, &mut metadata)?;
    }
    Ok(metadata)
}

fn inspect_input(
    path: &Path,
    bytes: &[u8],
    metadata: &mut Vec<ElfMetadata>,
) -> Result<(), LinkInputInspectionError> {
    if bytes.starts_with(ELF_MAGIC) {
        metadata.push(read_elf(&path.display().to_string(), bytes)?);
    } else if bytes.starts_with(AR_MAGIC) {
        inspect_archive(path, bytes, metadata)?;
    }
    // LLVM bitcode and linker scripts do not carry ELF e_flags.
    Ok(())
}

fn read_elf(display: &str, bytes: &[u8]) -> Result<ElfMetadata, LinkInputInspectionError> {
    if bytes.len() < 20 || &bytes[..4] != ELF_MAGIC {
        return Err(malformed(display, "truncated ELF header"));
    }
    let little_endian = match bytes[5] {
        1 => true,
        2 => false,
        _ => return Err(malformed(display, "invalid ELF data encoding")),
    };
    let machine = read_u16(&bytes[18..20], little_endian);
    let flags_offset = match bytes[4] {
        1 => 36,
        2 => 48,
        _ => return Err(malformed(display, "invalid ELF class")),
    };
    let Some(raw_flags) = bytes.get(flags_offset..flags_offset + 4) else {
        return Err(malformed(display, "truncated ELF e_flags"));
    };
    Ok(ElfMetadata {
        input: display.to_string(),
        machine,
        flags: read_u32(raw_flags, little_endian),
    })
}

fn inspect_archive(
    path: &Path,
    bytes: &[u8],
    metadata: &mut Vec<ElfMetadata>,
) -> Result<(), LinkInputInspectionError> {
    let display = path.display().to_string();
    let mut offset = AR_MAGIC.len();
    let mut long_names: Option<&[u8]> = None;
    while offset < bytes.len() {
        let Some(header) = bytes.get(offset..offset + 60) else {
            return Err(malformed(&display, "truncated archive header"));
        };
        if &header[58..60] != b"`\n" {
            return Err(malformed(&display, "invalid archive member header"));
        }
        let size_text = std::str::from_utf8(&header[48..58])
            .map_err(|_| malformed(&display, "invalid archive member size"))?;
        let size = size_text
            .trim()
            .parse::<usize>()
            .map_err(|_| malformed(&display, "invalid archive member size"))?;
        let data_start = offset + 60;
        let Some(member_data) = bytes.get(data_start..data_start + size) else {
            return Err(malformed(&display, "truncated archive member"));
        };
        let raw_name = std::str::from_utf8(&header[..16])
            .map_err(|_| malformed(&display, "invalid archive member name"))?
            .trim();

        let (name, payload) = if raw_name == "//" {
            long_names = Some(member_data);
            (None, member_data)
        } else if raw_name == "/" || raw_name == "/SYM64/" {
            (None, member_data)
        } else if let Some(length) = raw_name.strip_prefix("#1/") {
            let length = length
                .parse::<usize>()
                .map_err(|_| malformed(&display, "invalid BSD archive member name"))?;
            let Some(name_bytes) = member_data.get(..length) else {
                return Err(malformed(&display, "truncated BSD archive member name"));
            };
            let name = String::from_utf8_lossy(name_bytes)
                .trim_end_matches('\0')
                .to_string();
            (Some(name), &member_data[length..])
        } else if let Some(name_offset) = raw_name.strip_prefix('/') {
            let name_offset = name_offset
                .parse::<usize>()
                .map_err(|_| malformed(&display, "invalid GNU archive name offset"))?;
            let table = long_names
                .ok_or_else(|| malformed(&display, "archive long-name table is missing"))?;
            let tail = table
                .get(name_offset..)
                .ok_or_else(|| malformed(&display, "archive long-name offset is out of range"))?;
            let end = tail
                .windows(2)
                .position(|window| window == b"/\n")
                .unwrap_or(tail.len());
            (
                Some(String::from_utf8_lossy(&tail[..end]).into_owned()),
                member_data,
            )
        } else {
            (
                Some(raw_name.trim_end_matches('/').to_string()),
                member_data,
            )
        };

        if let Some(name) = name {
            if payload.starts_with(ELF_MAGIC) {
                metadata.push(read_elf(&format!("{}({})", path.display(), name), payload)?);
            }
        }
        offset = data_start + size + (size & 1);
    }
    Ok(())
}

fn malformed(input: &str, reason: &str) -> LinkInputInspectionError {
    LinkInputInspectionError::Malformed {
        input: input.to_string(),
        reason: reason.to_string(),
    }
}

fn read_u16(bytes: &[u8], little_endian: bool) -> u16 {
    let bytes = [bytes[0], bytes[1]];
    if little_endian {
        u16::from_le_bytes(bytes)
    } else {
        u16::from_be_bytes(bytes)
    }
}

fn read_u32(bytes: &[u8], little_endian: bool) -> u32 {
    let bytes = [bytes[0], bytes[1], bytes[2], bytes[3]];
    if little_endian {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive_header(name: &str, size: usize) -> Vec<u8> {
        let header = format!("{name:<16}{:<12}{:<6}{:<6}{:<8}{size:<10}`\n", 0, 0, 0, 0);
        assert_eq!(header.len(), 60);
        header.into_bytes()
    }

    #[test]
    fn reads_nul_padded_bsd_extended_member_names() {
        let member_name = b"main.o\0\0\0\0\0\0";
        let mut elf = vec![0u8; 52];
        elf[..4].copy_from_slice(ELF_MAGIC);
        elf[4] = 1;
        elf[5] = 1;
        elf[18..20].copy_from_slice(&243u16.to_le_bytes());
        elf[36..40].copy_from_slice(&2u32.to_le_bytes());

        let size = member_name.len() + elf.len();
        let mut archive = AR_MAGIC.to_vec();
        archive.extend(archive_header("#1/12", size));
        archive.extend(member_name);
        archive.extend(elf);

        let mut metadata = Vec::new();
        inspect_archive(Path::new("libmixed.a"), &archive, &mut metadata).unwrap();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].input, "libmixed.a(main.o)");
        assert_eq!(metadata[0].machine, 243);
        assert_eq!(metadata[0].flags, 2);
    }
}
