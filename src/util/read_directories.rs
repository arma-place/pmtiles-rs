use std::collections::HashMap;
use std::io::{Read, Result, Seek, SeekFrom};

use ahash::RandomState;

use crate::{Compression, Directory};

/// A structure representing a range of bytes within a larger amount of bytes.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
/// let mut header_section = vec![0; 127];
/// reader.read_exact(&mut header_section).unwrap();
/// let (_, header) = Header::read(header_section.view_bits(), ()).unwrap();
///
/// let entries_map = read_directories(
///     &mut reader,
///     header.internal_compression,
///     (header.root_directory_offset, header.root_directory_length),
///     header.leaf_directories_offset,
/// ).unwrap();
/// ```
pub fn read_directories(
    reader: &mut (impl Read + Seek),
    compression: Compression,
    root_dir_offset_length: (u64, u64),
    leaf_dir_offset: u64,
) -> Result<HashMap<u64, OffsetLength, RandomState>> {
    let mut tiles = HashMap::<u64, OffsetLength, RandomState>::default();

    read_dir_rec(
        reader,
        &mut tiles,
        compression,
        root_dir_offset_length,
        leaf_dir_offset,
    )?;

    Ok(tiles)
}

fn read_dir_rec(
    reader: &mut (impl Read + Seek),
    tiles: &mut HashMap<u64, OffsetLength, RandomState>,
    compression: Compression,
    (dir_offset, dir_length): (u64, u64),
    leaf_dir_offset: u64,
) -> Result<()> {
    reader.seek(SeekFrom::Start(dir_offset))?;
    let directory = Directory::from_reader(reader, dir_length, compression)?;

    for entry in directory.iter() {
        if entry.is_leaf_dir_entry() {
            read_dir_rec(
                reader,
                tiles,
                compression,
                (leaf_dir_offset + entry.offset, u64::from(entry.length)),
                leaf_dir_offset,
            )?;
            continue;
        }

        for tile_id in entry.tile_id_range() {
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

        let map = read_directories(&mut reader, Compression::GZip, (127, 246), 395)?;

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

        let map = read_directories(&mut reader, Compression::GZip, (127, 389), 1173)?;

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
}
