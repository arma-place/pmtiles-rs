#[cfg(feature = "async")]
use async_recursion::async_recursion;
#[cfg(feature = "async")]
use futures::io::{AsyncRead, AsyncReadExt, AsyncSeekExt};
use std::collections::HashMap;
use std::io::{Read, Result, Seek};
use std::ops::RangeBounds;

use ahash::RandomState;
use duplicate::duplicate_item;

use crate::{Compression, Directory};

/// A structure representing a range of bytes within a larger amount of bytes.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OffsetLength {
    /// Offset of first byte (in bytes)
    pub offset: u64,

    /// Number of bytes in the range
    pub length: u32,
}

/// Reads directories (root- & leaf-directories) from a reader and return all entries
/// as a [`std::collections::HashMap`], with the tile-id as the key and the offset & length as the value.
///
/// # Arguments
/// * `reader` - Reader with root- and leaf-directories
/// * `compression` - Compression of directories
/// * `root_dir_offset_length` - Offset and length (in bytes) of root directory section
/// * `leaf_dir_offset` - Offset (in bytes) of leaf directories section
/// * `filter_range` - Range of Tile IDs to load (use `..` to include all). This can improve performance in cases where only a limited range of tiles is needed, as whole leaf directories may be skipped during parsing.
///
/// # Errors
/// Will return [`Err`] if there was an error reading the bytes from the reader or while decompressing
/// a directory.
///
/// # Example
/// ```rust
/// # use deku::{bitvec::BitView, DekuRead};
/// # use pmtiles2::{util::read_directories, Compression, Header, PMTiles};
/// # use std::io::Read;
/// # let bytes: &[u8] = include_bytes!("../../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");
/// # let mut reader = std::io::Cursor::new(bytes);
/// let header = Header::from_reader(&mut reader).unwrap();
///
/// let entries_map = read_directories(
///     &mut reader,
///     header.internal_compression,
///     (header.root_directory_offset, header.root_directory_length),
///     header.leaf_directories_offset,
///     ..,
/// ).unwrap();
/// ```
pub fn read_directories(
    reader: &mut (impl Read + Seek),
    compression: Compression,
    root_dir_offset_length: (u64, u64),
    leaf_dir_offset: u64,
    filter_range: impl RangeBounds<u64>,
) -> Result<HashMap<u64, OffsetLength, RandomState>> {
    let mut tiles = HashMap::<u64, OffsetLength, RandomState>::default();

    read_dir_rec(
        reader,
        &mut tiles,
        compression,
        root_dir_offset_length,
        leaf_dir_offset,
        &filter_range,
    )?;

    Ok(tiles)
}

/// Async version of [`read_directories`](read_directories).
///
/// Reads directories (root- & leaf-directories) from a reader and return all entries
/// as a [`std::collections::HashMap`], with the tile-id as the key and the offset & length as the value.
///
/// # Arguments
/// * `reader` - Reader with root- and leaf-directories
/// * `compression` - Compression of directories
/// * `root_dir_offset_length` - Offset and length (in bytes) of root directory section
/// * `leaf_dir_offset` - Offset (in bytes) of leaf directories section
/// * `filter_range` - Range of Tile IDs to load (use `..` to include all). This can improve performance in cases where only a limited range of tiles is needed, as whole leaf directories may be skipped during parsing.
///
/// # Errors
/// Will return [`Err`] if there was an error reading the bytes from the reader or while decompressing
/// a directory.
///
/// # Example
/// ```rust
/// # use pmtiles2::{Header, Directory, Compression, util::read_directories_async};
/// # use futures::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
/// # tokio_test::block_on(async {
/// let bytes = include_bytes!("../../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");
/// let mut reader = futures::io::Cursor::new(bytes);
///
/// let header = Header::from_async_reader(&mut reader).await.unwrap();
///
/// let entries_map = read_directories_async(
///     &mut reader,
///     header.internal_compression,
///     (header.root_directory_offset, header.root_directory_length),
///     header.leaf_directories_offset,
///     ..,
/// ).await.unwrap();
/// # })
/// ```
#[allow(clippy::module_name_repetitions)]
#[cfg(feature = "async")]
pub async fn read_directories_async(
    reader: &mut (impl AsyncRead + Unpin + Send + AsyncReadExt + AsyncSeekExt),
    compression: Compression,
    root_dir_offset_length: (u64, u64),
    leaf_dir_offset: u64,
    filter_range: (impl RangeBounds<u64> + Sync + Send),
) -> Result<HashMap<u64, OffsetLength, RandomState>> {
    let mut tiles = HashMap::<u64, OffsetLength, RandomState>::default();

    read_dir_rec_async(
        reader,
        &mut tiles,
        compression,
        root_dir_offset_length,
        leaf_dir_offset,
        &filter_range,
    )
    .await?;

    Ok(tiles)
}

/// Get (inclusive) end of range bounds.
///
/// Will return [`None`] if range has no end bound.
fn range_end_inc(range: &impl RangeBounds<u64>) -> Option<u64> {
    match range.end_bound() {
        std::ops::Bound::Included(val) => Some(*val),
        std::ops::Bound::Excluded(val) => Some(*val - 1),
        std::ops::Bound::Unbounded => None,
    }
}

#[duplicate_item(
    fn_name              cfg_async_filter       async                      add_await(code) seek_start(reader, offset)                                 FilterRangeTraits                       input_traits                                                    read_directory(reader, len, compression);
    [read_dir_rec]       [cfg(all())]           []                         [code]          [reader.seek(std::io::SeekFrom::Start(offset))]            [(impl RangeBounds<u64>)]               [(impl Read + Seek)]                                            [Directory::from_reader(reader, len, compression)];
    [read_dir_rec_async] [cfg(feature="async")] [#[async_recursion] async] [code.await]    [reader.seek(futures::io::SeekFrom::Start(offset)).await]  [(impl RangeBounds<u64> + Sync + Send)] [(impl AsyncRead + Unpin + Send + AsyncReadExt + AsyncSeekExt)] [Directory::from_async_reader(reader, len, compression).await];
)]
#[cfg_async_filter]
async fn fn_name(
    reader: &mut input_traits,
    tiles: &mut HashMap<u64, OffsetLength, RandomState>,
    compression: Compression,
    (dir_offset, dir_length): (u64, u64),
    leaf_dir_offset: u64,
    filter_range: &FilterRangeTraits,
) -> Result<()> {
    seek_start([reader], [dir_offset])?;
    let directory = read_directory([reader], [dir_length], [compression])?;
    let range_end = range_end_inc(filter_range).unwrap_or(u64::MAX);

    for entry in &directory {
        if entry.is_leaf_dir_entry() {
            // skip leaf directory, if it starts after range
            if entry.tile_id > range_end {
                continue;
            }

            add_await([fn_name(
                reader,
                tiles,
                compression,
                (leaf_dir_offset + entry.offset, u64::from(entry.length)),
                leaf_dir_offset,
                filter_range,
            )])?;
            continue;
        }

        for tile_id in entry.tile_id_range() {
            if !filter_range.contains(&tile_id) {
                continue;
            }

            tiles.insert(
                tile_id,
                OffsetLength {
                    offset: entry.offset,
                    length: entry.length,
                },
            );
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use std::io::{Cursor, Result};

    use super::*;

    #[test]
    fn test_read_directories_basic() -> Result<()> {
        let bytes: &[u8] = include_bytes!("../../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");
        let mut reader = Cursor::new(bytes);

        let map = read_directories(&mut reader, Compression::GZip, (127, 246), 395, ..)?;

        assert_eq!(map.len(), 85);

        assert_eq!(
            map.get(&19).unwrap(),
            &OffsetLength {
                offset: 225_929,
                length: 11259
            }
        );

        assert_eq!(
            map.get(&59).unwrap(),
            &OffsetLength {
                offset: 422_070,
                length: 850
            }
        );

        Ok(())
    }

    #[test]
    fn test_read_directories_with_leaf() -> Result<()> {
        let bytes: &[u8] =
            include_bytes!("../../test/protomaps_vector_planet_odbl_z10_without_data.pmtiles");
        let mut reader = Cursor::new(bytes);

        let map = read_directories(&mut reader, Compression::GZip, (127, 389), 1173, ..)?;

        assert_eq!(map.len(), 1_398_101);

        assert_eq!(
            map.get(&1_027_840).unwrap(),
            &OffsetLength {
                offset: 1_105_402_834,
                length: 59
            }
        );

        assert_eq!(
            map.get(&0).unwrap(),
            &OffsetLength {
                offset: 0,
                length: 92574
            }
        );

        Ok(())
    }

    #[test]
    fn test_range_end_inc() {
        assert_eq!(range_end_inc(&(..)), None);
        assert_eq!(range_end_inc(&(..=3)), Some(3));
        assert_eq!(range_end_inc(&(..3)), Some(2));
        assert_eq!(range_end_inc(&(1..)), None);
    }
}
