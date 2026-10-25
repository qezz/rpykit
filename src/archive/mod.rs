//! Renpy Archive (RPA, .rpa) manipulation module

pub mod extract;
pub mod header;
pub mod index;
pub mod parallel;
pub mod sequential;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::str::{FromStr, Utf8Error};

use crate::archive::header::{Header, HeaderError};
use crate::archive::index::{IndexError, IndexUnpacked, read_index};

/// Size of the archive's magic string.
pub const RPA_MAGIC_STRING_SIZE: usize = 34;
/// Manually defined zLib header
pub const ZLIB_HEADER: u8 = 0x78;

/// RPA archive version
///
/// The version is usually derived from the file's magic string.
// TODO: Support v2
// TODO: Support custom headers
#[derive(Clone, Debug)]
pub enum Version {
    V3,
}

#[derive(Clone, Debug)]
pub struct VersionError {
    pub reason: String,
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RPA-3.0" => Ok(Version::V3),
            unsupported @ "RPA-2.0" => Err(VersionError {
                reason: format!("Unsupported RPA version: {unsupported}"),
            }),
            other => Err(VersionError {
                reason: format!("Unknown RPA version: {other}"),
            }),
        }
    }
}

#[derive(Debug)]
pub enum ArchiveError {
    FailedToOpenArchive(std::io::Error),
    FileTooSmall {
        expected_bytes: usize,
        err: std::io::Error,
    },
    MagicHeaderIsNotAString(Utf8Error),
    InvalidHeader(HeaderError),
    InvalidIndex(IndexError),
    ThreadPanicked(String),
}

// NOTE: Reader requires `Seek`, meaning you can't pass stdin to it.
//
// RPA archives store their index in the very end if the file, so one needs
// to buffer the whole archive in memory in order to be able to parse data
// coming from stdin.
pub fn read_header_and_index<R: Read + Seek>(
    mut reader: R,
) -> Result<(Header, IndexUnpacked), ArchiveError> {
    let mut magic_string_buf = [0u8; RPA_MAGIC_STRING_SIZE];

    reader
        .read_exact(&mut magic_string_buf)
        .map_err(|e| ArchiveError::FileTooSmall {
            expected_bytes: RPA_MAGIC_STRING_SIZE,
            err: e,
        })?;

    let magic = str::from_utf8(&magic_string_buf).map_err(ArchiveError::MagicHeaderIsNotAString)?;
    let header = Header::from_str(magic).map_err(ArchiveError::InvalidHeader)?;
    let index = read_index(reader, &header).map_err(ArchiveError::InvalidIndex)?;

    Ok((header, index))
}

pub struct Archive {
    pub path: PathBuf,
    pub header: Header,
    pub index: IndexUnpacked,
}

impl Archive {
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Archive, ArchiveError> {
        let file = File::open(path.as_ref()).map_err(ArchiveError::FailedToOpenArchive)?;

        let (header, index) = read_header_and_index(file)?;

        Ok(Archive {
            path: path.as_ref().to_path_buf(),
            header,
            index,
        })
    }
}

pub fn guess_number_of_threads() -> usize {
    match std::thread::available_parallelism() {
        Ok(n) => n.get(),
        Err(_) => 1,
    }
}

pub fn find_output_overlaps(
    archives: &HashMap<std::path::PathBuf, Archive>,
) -> HashMap<String, Vec<String>> {
    // TODO: should be possible to construct the actual list of input files (the
    // key, Vec<String>) only when the error condition is triggered.
    //
    // In a happy path case, we can live with counters as hashmap values.
    // Otherwise, we build the proper mapping.
    let mut freq_map: HashMap<String, Vec<String>> = HashMap::new();
    for (input_fname, index) in archives.iter() {
        for output_fname in index.index.keys() {
            freq_map
                .entry(output_fname.clone())
                .or_default()
                .push(input_fname.to_string_lossy().to_string());
        }
    }

    // If there are file with the same name coming from the different archives,
    // we through an error. Ingnoring this would result in user losing their
    // data.
    let mut overlaps = HashMap::new();
    for (output_file, input_files) in freq_map {
        if input_files.len() > 1 {
            overlaps.insert(output_file, input_files);
        }
    }
    overlaps
}
