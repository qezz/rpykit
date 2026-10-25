use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::{
    archive::{
        Archive, ArchiveError,
        extract::{ExtractError, UnarchiveError, create_directories_ahead_of_time, sorted_entries},
        find_output_overlaps,
        index::{IndexUnpacked, UnifiedIndexEntry},
    },
    utils::format_thread_join_error_best_effort,
};

pub fn read_archives(
    paths: Vec<PathBuf>,
    num_threads: usize,
) -> (HashMap<PathBuf, Archive>, Vec<ArchiveError>) {
    if paths.is_empty() {
        return (HashMap::new(), Vec::new());
    }

    let chunk_size = paths.len().div_ceil(num_threads);

    let handles: Vec<_> = paths
        .chunks(chunk_size)
        .map(|chunk| {
            let chunk = chunk.to_vec();
            std::thread::spawn(move || {
                chunk
                    .into_iter()
                    .map(|path| {
                        let result = Archive::read_from_file(&path);
                        (path, result)
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();

    let mut archives = HashMap::new();
    let mut errors: Vec<ArchiveError> = Vec::new();

    for handle in handles {
        match handle.join() {
            Ok(results) => {
                for (path, result) in results {
                    match result {
                        Ok(archive) => {
                            archives.insert(path, archive);
                        }
                        Err(e) => {
                            errors.push(e);
                        }
                    }
                }
            }
            Err(e) => errors.push(ArchiveError::ThreadPanicked(
                format_thread_join_error_best_effort(&*e),
            )),
        }
    }
    (archives, errors)
}

pub fn extract<P: AsRef<Path>>(
    path: P,
    output_dir: P,
    entries: Vec<(String, UnifiedIndexEntry)>,
    num_threads: usize,
) -> Vec<ExtractError> {
    if entries.is_empty() {
        return Vec::new();
    }

    let chunk_size = entries.len().div_ceil(num_threads);

    let path = path.as_ref().to_path_buf();
    let output_dir = output_dir.as_ref().to_path_buf();

    let handles: Vec<_> = entries
        .chunks(chunk_size)
        .map(|chunk| {
            let chunk = chunk.to_vec();
            let path = path.clone();
            let output_dir = output_dir.clone();

            std::thread::spawn(move || {
                let mut file = File::open(path.clone()).unwrap();

                for (filename, entry) in chunk {
                    let output_path = output_dir.join(&filename);

                    file.seek(SeekFrom::Start(entry.offset)).unwrap();

                    let mut output = File::create(output_path).unwrap();
                    let mut contents_reader = (&mut file).take(entry.real_len());
                    std::io::copy(&mut contents_reader, &mut output).unwrap();
                }
            })
        })
        .collect();

    let mut errors = Vec::new();
    for handle in handles {
        if let Err(e) = handle.join() {
            errors.push(ExtractError::ThreadPanicked(
                format_thread_join_error_best_effort(&*e),
            ));
        }
    }

    errors
}

pub fn extract_archive<P: AsRef<Path>>(
    path: P,
    index: IndexUnpacked,
    output_dir: P,
    num_threads: usize,
) -> Result<(), ExtractError> {
    create_directories_ahead_of_time(&index, &output_dir)
        .map_err(ExtractError::FailedToCreateOutputDir)?;

    let entries = sorted_entries(index);

    extract(path, output_dir, entries, num_threads);

    Ok(())
}

pub fn extract_all<P: AsRef<Path>>(
    paths: &[P],
    output_dir: P,
    num_threads: usize,
) -> Result<(), UnarchiveError> {
    assert!(num_threads > 0, "Number of threads must be > 0");

    let paths: Vec<_> = paths.iter().map(|x| x.as_ref().to_path_buf()).collect();

    let (archives, errs) = read_archives(paths, num_threads);
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
            num_threads,
        );

        results.push(res);
    }

    Ok(())
}
