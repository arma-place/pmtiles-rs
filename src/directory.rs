use integer_encoding::{VarIntReader, VarIntWriter};
use std::io::{Read, Result, Write};
use std::ops::{Index, IndexMut, Range};
use std::slice::{Iter, SliceIndex};

use crate::util::{compress, decompress};
use crate::Compression;

/// A structure representing a directory entry.
///
/// A entry includes information on where to find either a leaf directory or one/multiple tiles.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Entry {
    /// The first tile id this entry is valid for
    pub tile_id: u64,

    /// Offset (in bytes) of first byte of tile of leaf-directory data
    ///
    /// For tiles this offset is relative to the start of the tile data sections.
    /// For leaf directories this offset is relative to the start of the leaf directory sections.
    pub offset: u64,

    /// Amount of bytes
    pub length: u32,

    /// The run length indicates the amount of tiles this entry is valid for.
    /// A run length of `0` indicates that this is in fact a entry containing information
    /// of a leaf directory.
    pub run_length: u32,
}

impl Entry {
    /// Returns the range of tile ids this entry is valid for.
    pub const fn tile_id_range(&self) -> Range<u64> {
        self.tile_id..self.tile_id + self.run_length as u64
    }

    /// Returns `true` if this entry is for a leaf directory and
    /// `false` if this entry is for tile data.
    pub const fn is_leaf_dir_entry(&self) -> bool {
        self.run_length == 0
    }
}

/// A structure representing a directory.
///
/// A directory holds an arbitrary amount of [`Entry`]. You can use [`len`](Self::len), [`is_empty`](Self::is_empty) and
/// [`iter`](Self::iter) to obtain information about that list of entries.
///
/// Use [`from_reader`](Self::from_reader) and [`to_writer`](Self::to_writer) to read and write the directory from / to bytes.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Directory {
    entries: Vec<Entry>,
}

impl Directory {
    /// Returns the number of entries in the directory, also referred to as its 'length'.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the directory contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns an iterator over the directory.
    ///
    /// The iterator yields all entries from start to end.
    pub fn iter(&self) -> Iter<'_, Entry> {
        self.entries.iter()
    }

    /// Reads a directory from a [`std::io::Read`] and returns it.
    ///
    /// # Arguments
    /// * `input` - Reader including directory bytes
    /// * `length` - Length of the directory (in bytes)
    /// * `compression` - Compression of the  directory
    ///
    /// # Errors
    /// Will return [`Err`] if `compression` is set to [`Compression::Unknown`], the data is not compressed correctly
    /// according to `compression` or there was any kind of I/O error while reading from `input`.
    ///
    /// # Example
    /// ```rust
    /// # use pmtiles2::{Directory, Compression};
    /// # use std::io::{Cursor, Seek, SeekFrom};
    /// let bytes = include_bytes!("../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");
    /// let mut reader = Cursor::new(bytes);
    /// reader.seek(SeekFrom::Start(127)).unwrap();
    ///
    /// let directory = Directory::from_reader(&mut reader, 246, Compression::GZip).unwrap();
    /// ```
    #[allow(clippy::needless_range_loop)]
    pub fn from_reader(
        input: &mut impl Read,
        length: u64,
        compression: Compression,
    ) -> Result<Self> {
        let mut binding = input.take(length);
        let mut reader = decompress(compression, &mut binding)?;

        let num_entries = reader.read_varint::<usize>()?;

        let mut entries = Vec::<Entry>::with_capacity(num_entries);

        // read tile_id
        let mut last_id = 0u64;
        for _ in 0..num_entries {
            let tmp = reader.read_varint::<u64>()?;

            last_id += tmp;
            entries.push(Entry {
                tile_id: last_id,
                length: 0,
                offset: 0,
                run_length: 0,
            });
        }

        // read run_length
        for i in 0..num_entries {
            entries[i].run_length = reader.read_varint()?;
        }

        // read length
        for i in 0..num_entries {
            entries[i].length = reader.read_varint()?;
        }

        // read offset
        for i in 0..num_entries {
            let val = reader.read_varint::<u64>()?;

            entries[i].offset = if i > 0 && val == 0 {
                entries[i - 1].offset + u64::from(entries[i - 1].length)
            } else {
                val - 1
            };
        }

        Ok(Self { entries })
    }

    /// Writes the directory to a [`std::io::Write`].
    ///
    /// # Arguments
    /// * `output` - Writer to write directory to
    /// * `compression` - Compression to use
    ///
    /// # Errors
    /// Will return [`Err`] if `compression` is set to [`Compression::Unknown`] or there was
    /// any kind of I/O error while writing to `output`.
    ///
    /// # Example
    /// ```rust
    /// # use pmtiles2::{Directory, Compression};
    /// let directory: Directory = Vec::new().into();
    ///
    /// let mut output = std::io::Cursor::new(Vec::<u8>::new());
    ///
    /// directory.to_writer(&mut output, Compression::GZip).unwrap();
    /// ```
    pub fn to_writer(&self, output: &mut impl Write, compression: Compression) -> Result<()> {
        let mut writer = compress(compression, output)?;

        writer.write_varint(self.entries.len())?;

        // write tile_id
        let mut last_id = 0u64;
        for entry in &self.entries {
            writer.write_varint(entry.tile_id - last_id)?;
            last_id = entry.tile_id;
        }

        // write run_length
        for entry in &self.entries {
            writer.write_varint(entry.run_length)?;
        }

        // write length
        for entry in &self.entries {
            writer.write_varint(entry.length)?;
        }

        // write offset
        let mut next_byte = 0u64;
        for (index, entry) in self.entries.iter().enumerate() {
            let val = if index > 0 && entry.offset == next_byte {
                0
            } else {
                entry.offset + 1
            };

            writer.write_varint(val)?;

            next_byte = entry.offset + u64::from(entry.length);
        }

        Ok(())
    }
}

impl<I: SliceIndex<[Entry]>> Index<I> for Directory {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.entries.index(index)
    }
}

impl<I: SliceIndex<[Entry]>> IndexMut<I> for Directory {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.entries.index_mut(index)
    }
}

impl From<Vec<Entry>> for Directory {
    fn from(entries: Vec<Entry>) -> Self {
        Self { entries }
    }
}

impl From<Directory> for Vec<Entry> {
    fn from(val: Directory) -> Self {
        val.entries
    }
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)]
mod test {
    use std::io::{Cursor, Seek, SeekFrom};

    use crate::util::decompress_all;

    use super::*;

    const PM_TILES_BYTES: &[u8] =
        include_bytes!("../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");

    const ROOT_DIR_OFFSET: u64 = 127;
    const ROOT_DIR_LENGTH: u64 = 246;
    const ROOT_DIR_COMPRESSION: Compression = Compression::GZip;

    #[test]
    fn test_from_reader() -> Result<()> {
        let mut reader = Cursor::new(PM_TILES_BYTES);
        reader.seek(SeekFrom::Start(ROOT_DIR_OFFSET))?;

        let dir = Directory::from_reader(&mut reader, ROOT_DIR_LENGTH, ROOT_DIR_COMPRESSION)?;

        assert_eq!(reader.position(), ROOT_DIR_OFFSET + ROOT_DIR_LENGTH);
        assert_eq!(dir.entries.len(), 84);
        assert_eq!(
            dir.entries[0],
            Entry {
                tile_id: 0,
                offset: 0,
                length: 18404,
                run_length: 1
            }
        );

        assert_eq!(
            dir.entries[58],
            Entry {
                tile_id: 58,
                offset: 422_070,
                length: 850,
                run_length: 2
            }
        );

        assert_eq!(
            dir.entries[83],
            Entry {
                tile_id: 84,
                offset: 243_790,
                length: 914,
                run_length: 1
            }
        );

        Ok(())
    }

    #[test]
    fn test_to_writer() -> Result<()> {
        let mut reader = Cursor::new(PM_TILES_BYTES);
        reader.seek(SeekFrom::Start(ROOT_DIR_OFFSET))?;

        let dir = Directory::from_reader(&mut reader, ROOT_DIR_LENGTH, ROOT_DIR_COMPRESSION)?;

        let mut buf = Vec::<u8>::with_capacity(ROOT_DIR_LENGTH as usize);
        let mut writer = Cursor::new(&mut buf);
        dir.to_writer(&mut writer, ROOT_DIR_COMPRESSION)?;

        // we compare the decompressed versions of the directory, as the compressed
        // bytes may not match 100% due to different compression parameters
        let output = decompress_all(ROOT_DIR_COMPRESSION, &buf)?;
        let expected = decompress_all(
            ROOT_DIR_COMPRESSION,
            &PM_TILES_BYTES[ROOT_DIR_OFFSET as usize..(ROOT_DIR_OFFSET + ROOT_DIR_LENGTH) as usize],
        )?;

        assert_eq!(output, expected);

        Ok(())
    }
}
