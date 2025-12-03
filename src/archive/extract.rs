use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use crate::archive::{ArchiveError, header::HeaderError, index::UnifiedIndexEntry};

#[derive(Clone, Debug)]
pub enum ExtractMode {
    Parallel(usize),
    Sequential,
}

#[derive(Debug)]
pub enum ExtractError {
    FailedToCreateOutputDir(std::io::Error),
    ThreadPanicked(String),
    FailedToOpenArchive(std::io::Error),
    FailedToSeekArchiveFile(std::io::Error),
    FailedToCopyFromArchiveToOutputFile(std::io::Error),
}

#[derive(Debug)]
pub enum UnarchiveError {
    HeaderError(HeaderError),
    FileOverlapError(HashMap<String, Vec<String>>),
    ArchiveReadErrors(Vec<ArchiveError>),
    ArchiveExtractErrors(Vec<(PathBuf, ArchiveError)>),
}

pub fn create_directories_ahead_of_time<P: AsRef<Path>>(
    index: &std::collections::HashMap<String, UnifiedIndexEntry>,
    output_dir: &P,
) -> Result<(), std::io::Error> {
    if !output_dir.as_ref().exists() {
        std::fs::create_dir_all(output_dir.as_ref())?;
    }

    let mut seen_dirs = HashSet::new();

    for (filename, _entry) in index.iter() {
        let output_path = output_dir.as_ref().join(filename);
        if let Some(parent) = output_path.parent() {
            seen_dirs.insert(parent.to_path_buf());
        }
    }

    for dir in seen_dirs.iter() {
        std::fs::create_dir_all(dir)?;
    }

    Ok(())
}

#[inline]
pub fn sorted_entries(
    index: HashMap<String, UnifiedIndexEntry>,
) -> Vec<(String, UnifiedIndexEntry)> {
    // NOTE: Presumably reading the file contents in the direct
    // order should be faster on HDD drives. Unfortunately,
    // I don't have an HDD at hand, so I will keep it for
    // future testing.
    //
    // I checked it against NVMe and didn't see any change between
    // reading ordered vs random.
    let mut entries: Vec<_> = index.into_iter().collect();
    entries.sort_by_key(|(_, entry)| entry.offset);
    entries
}
