use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::archive::{
    Archive, ArchiveError,
    extract::{ExtractError, UnarchiveError, create_directories_ahead_of_time, sorted_entries},
    find_output_overlaps,
    index::{IndexUnpacked, UnifiedIndexEntry},
};

pub fn read_archives(paths: Vec<PathBuf>) -> (HashMap<PathBuf, Archive>, Vec<ArchiveError>) {
    let results: Vec<_> = paths
        .into_iter()
        .map(|path| (path.clone(), Archive::read_from_file(path)))
        .collect();

    let mut archives = HashMap::new();
    let mut errors: Vec<ArchiveError> = Vec::new();

    for (path, res) in results {
        match res {
            Ok(archive) => {
                archives.insert(path, archive);
            }
            Err(e) => errors.push(e),
        }
    }

    (archives, errors)
}

pub fn extract<P: AsRef<Path>>(
    path: P,
    output_dir: P,
    entries: Vec<(String, UnifiedIndexEntry)>,
) -> Result<(), ExtractError> {
    let mut file = File::open(path).map_err(ExtractError::FailedToOpenArchive)?;

    for (filename, entry) in entries {
        let output_path = output_dir.as_ref().to_path_buf().join(&filename);

        file.seek(SeekFrom::Start(entry.offset))
            .map_err(ExtractError::FailedToSeekArchiveFile)?;

        let mut output =
            File::create(output_path).map_err(ExtractError::FailedToCreateOutputDir)?;

        let mut contents_reader = (&mut file).take(entry.real_len());

        std::io::copy(&mut contents_reader, &mut output)
            .map_err(ExtractError::FailedToCopyFromArchiveToOutputFile)?;
    }

    Ok(())
}

pub fn extract_archive<P: AsRef<Path>>(
    path: P,
    index: IndexUnpacked,
    output_dir: P,
) -> Result<(), ExtractError> {
    create_directories_ahead_of_time(&index, &output_dir)
        .map_err(ExtractError::FailedToCreateOutputDir)?;

    let entries = sorted_entries(index);

    extract(path, output_dir, entries)
}

pub fn extract_all<P: AsRef<Path>>(paths: &[P], output_dir: P) -> Result<(), UnarchiveError> {
    let paths: Vec<_> = paths.iter().map(|x| x.as_ref().to_path_buf()).collect();

    let (archives, errs) = read_archives(paths);
    if !errs.is_empty() {
        eprintln!("Encountered erros while extracting index:");
        for err in errs.iter() {
            eprintln!("  {:?}", err);
        }
        return Err(UnarchiveError::ArchiveReadErrors(errs));
    }

    let overlaps = find_output_overlaps(&archives);
    if !overlaps.is_empty() {
        return Err(UnarchiveError::FileOverlapError(overlaps));
    }

    let mut results = Vec::with_capacity(archives.len());

    for (path, archive) in archives.iter() {
        let res = extract_archive(
            path,
            archive.index.clone(),
            &output_dir.as_ref().to_path_buf(),
        );

        results.push(res);
    }

    Ok(())
}
