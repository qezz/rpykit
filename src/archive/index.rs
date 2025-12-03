use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom},
};

use super::{ZLIB_HEADER, header::Header};

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct UnifiedIndexEntry {
    pub offset: u64,
    pub len: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "serde_bytes")]
    pub prefix: Option<Vec<u8>>,
}

impl UnifiedIndexEntry {
    pub fn real_len(&self) -> u64 {
        // Converting length (usize) to u64 should be enough in most cases
        self.len - self.prefix.as_ref().map_or(0, |v| v.len() as u64)
    }
}

// Index comes with values of lists with only 1 item. Therefore, `IndexUnpacked`
// is (hopefully) the same data, but unpacked.
pub type Index = HashMap<String, Vec<UnifiedIndexEntry>>;
pub type IndexUnpacked = HashMap<String, UnifiedIndexEntry>;

#[derive(Debug)]
pub enum IndexError {
    FailedToSeekReader(std::io::Error),
    FailedToReadToEnd(std::io::Error),
    FailedToParse(serde_pickle::Error),
    UnexpectedNumberOfItems { expected: usize, actual: usize },
    FailedToDecompress(std::io::Error),
    IndexThreadFailedToJoin,
}

pub fn parse_index(data: &[u8], key: u32) -> Result<IndexUnpacked, IndexError> {
    let key = key as u64; // it must always succeed as u32 is smaller than u64.
    let value: Index =
        serde_pickle::from_slice(data, Default::default()).map_err(IndexError::FailedToParse)?;

    let mut res = HashMap::with_capacity(value.len());

    for (fpath, mut entries) in value {
        if entries.len() != 1 {
            return Err(IndexError::UnexpectedNumberOfItems {
                expected: 1,
                actual: entries.len(),
            });
        }

        let mut entry = entries.pop().expect("at least one item must be present");
        entry.offset ^= key;
        entry.len ^= key;

        res.insert(fpath, entry);
    }

    Ok(res)
}

pub fn read_index<R: Read + Seek>(
    mut reader: R,
    header: &Header,
) -> Result<IndexUnpacked, IndexError> {
    reader
        .seek(SeekFrom::Start(header.index_offset))
        .map_err(IndexError::FailedToSeekReader)?;

    let mut index_data = Vec::new();
    reader
        .read_to_end(&mut index_data)
        .map_err(IndexError::FailedToReadToEnd)?;

    if !index_data.is_empty() && index_data[0] == ZLIB_HEADER {
        index_data = decompress_index_data(&index_data).map_err(IndexError::FailedToDecompress)?;
    }

    let index = parse_index(&index_data, header.key)?;

    Ok(index)
}

// Index should not be too big, compared to the actual data,
// so it's fine to allocate a new data slice for it.
pub fn decompress_index_data(index_data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Cursor;
    let mut decoder = flate2::read::ZlibDecoder::new(Cursor::new(&index_data));
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    Ok(decompressed)
}
