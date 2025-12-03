use std::{num::ParseIntError, str::FromStr};

use super::{Version, VersionError};

#[derive(Clone, Debug)]
pub enum HeaderError {
    InvalidMagicString { magic: String, details: String },
    InvalidVersion(VersionError),
    ParseKey(ParseIntError),
    ParseOffset(ParseIntError),
}

impl From<VersionError> for HeaderError {
    fn from(e: VersionError) -> HeaderError {
        HeaderError::InvalidVersion(e)
    }
}

/// Archive header, read from the file's magic string. It does not include
/// file index.
#[derive(Clone, Debug)]
pub struct Header {
    /// Archive version
    pub version: Version,
    /// Points to the start of the file index in the archive.
    ///
    /// `index_offset` is derived from 16 bytes containing hexadecimal
    /// characters, making its max value to be 2^64.
    ///
    /// References:
    /// - <https://github.com/renpy/renpy/blob/804e385c8c5a0a56e934060bbf80a25deea5361b/renpy/update/common.py#L188>
    pub index_offset: u64,
    /// Key is used for some deobfuscation shenanigans. The most common value is
    /// `0x42424242`, and it's hardcoded in the renpy archiver itself.
    ///
    /// `key` is derived from 8 bytes containing hexadecimal characters, making
    /// its max value to be 2^32.
    ///
    /// References:
    /// - <https://github.com/renpy/renpy/blob/804e385c8c5a0a56e934060bbf80a25deea5361b/renpy/update/common.py#L189>
    /// - <https://github.com/renpy/renpy/blob/804e385c8c5a0a56e934060bbf80a25deea5361b/launcher/game/archiver.rpy#L50>
    pub key: u32,
}

impl FromStr for Header {
    type Err = HeaderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: Make it's not very important that the magic string ends with a
        // newline, and this restriction can be lifted off.
        if !s.ends_with('\n') {
            return Err(HeaderError::InvalidMagicString {
                magic: s.into(),
                details: format!("Magic string must end with '\n', got: {:?}", s),
            });
        }

        let parts: Vec<&str> = s.trim().split(' ').collect();
        if parts.len() != 3 {
            return Err(HeaderError::InvalidMagicString {
                magic: s.into(),
                details: format!(
                    "Expected 3 parts to metadata in magic string, got: {:?}",
                    parts
                ),
            });
        }

        let (version_str, index_offset_str, key_str) = (parts[0], parts[1], parts[2]);

        let version: Version = version_str.parse()?;
        let key = u32::from_str_radix(key_str, 16).map_err(HeaderError::ParseKey)?;
        let index_offset =
            u64::from_str_radix(index_offset_str, 16).map_err(HeaderError::ParseOffset)?;

        Ok(Header {
            version,
            key,
            index_offset,
        })
    }
}
