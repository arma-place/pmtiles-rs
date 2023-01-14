pub use compression::*;
pub use lat_lng::*;
pub use tile_type::*;

mod compression;
mod lat_lng;
mod tile_type;

use deku::bitvec::{BitVec, BitView};
use deku::prelude::*;
use std::io::{Read, Write};

pub const HEADER_BYTES: u8 = 127;

/// A structure representing a `PMTiles` header.
#[derive(DekuRead, DekuWrite, Debug)]
#[deku(magic = b"PMTiles")]
#[deku(endian = "little")]
pub struct Header {
    /// Version of Specification (always 3)
    #[deku(assert_eq = "3")]
    pub spec_version: u8,

    /// Offset (in bytes) of root directory section from start of file
    pub root_directory_offset: u64,

    /// Length (in bytes) of root directory section
    pub root_directory_length: u64,

    /// Offset (in bytes) of metadata section from start of file
    pub json_metadata_offset: u64,

    /// Length (in bytes) of metadata section
    pub json_metadata_length: u64,

    /// Offset (in bytes) of leaf directories section from start of file
    pub leaf_directories_offset: u64,

    /// Length (in bytes) of leaf directories section
    pub leaf_directories_length: u64,

    /// Offset (in bytes) of tile data section from start of file
    pub tile_data_offset: u64,

    /// Length (in bytes) of tile data section
    pub tile_data_length: u64,

    /// Number of tiles, which are addressable in this PMTiles archive
    pub num_addressed_tiles: u64,

    /// Number of directory entries, that point to a tile
    pub num_tile_entries: u64,

    /// Number of distinct tile contents in the tile data section
    pub num_tile_content: u64,

    /// Indicates whether this archive is clustered, which means that
    /// all directory entries are ordered in ascending order by tile_ids
    #[deku(bits = 8)]
    pub clustered: bool,

    /// Compression of directories and meta data section
    pub internal_compression: Compression,

    /// Compression of tiles in this archive
    pub tile_compression: Compression,

    /// Type of tiles in this archive
    pub tile_type: TileType,

    /// Minimum zoom of all tiles this archive
    pub min_zoom: u8,

    /// Maximum zoom of all tiles this archive
    pub max_zoom: u8,

    /// Minimum latitude and longitude of bounds of available tiles in this archive
    pub min_pos: LatLng,

    /// Maximum latitude and longitude of bounds of available tiles in this archive
    pub max_pos: LatLng,

    /// Center zoom
    ///
    /// Implementations may use this to set the default zoom
    pub center_zoom: u8,

    /// Center latitude and longitude
    ///
    /// Implementations may use these values to set the default location
    pub center_pos: LatLng,
}

impl Header {
    /// Returns a option containing the value to which the `Content-Encoding`
    /// HTTP header should be set, when serving tiles from this archive.
    ///
    /// Returns [`None`] if a concrete `Content-Encoding` could not be determined.
    pub const fn http_content_type(&self) -> Option<&'static str> {
        self.tile_type.http_content_type()
    }

    /// Returns a option containing the value to which the `Content-Type` HTTP
    /// header should be set, when serving tiles from this archive.
    ///
    /// Returns [`None`] if a concrete `Content-Type` could not be determined.
    pub const fn http_content_encoding(&self) -> Option<&'static str> {
        self.tile_compression.http_content_encoding()
    }

    /// Reads a header from a [`std::io::Read`] and returns it.
    ///
    /// # Arguments
    /// * `input` - Reader
    ///
    /// # Errors
    /// Will return [`Err`] an I/O error occurred while reading from `input`.
    ///
    pub fn from_reader(input: &mut impl Read) -> std::io::Result<Self> {
        let mut buf = [0; HEADER_BYTES as usize];
        input.read_exact(&mut buf)?;

        let (_, header) = Self::read(buf.to_vec().view_bits(), ())?;

        Ok(header)
    }

    /// Writes the header to a [`std::io::Write`].
    ///
    /// # Arguments
    /// * `output` - Writer to write header to
    ///
    /// # Errors
    /// Will return [`Err`] if an I/O error occurred while writing to `output`.
    ///
    pub fn to_writer(&self, output: &mut impl Write) -> std::io::Result<()> {
        let mut bit_vec = BitVec::with_capacity(8 * HEADER_BYTES as usize);
        self.write(&mut bit_vec, ())?;
        output.write_all(bit_vec.as_raw_slice())?;

        Ok(())
    }
}

impl Default for Header {
    fn default() -> Self {
        Self {
            spec_version: 3,
            root_directory_offset: 0,
            root_directory_length: 0,
            json_metadata_offset: 0,
            json_metadata_length: 0,
            leaf_directories_offset: 0,
            leaf_directories_length: 0,
            tile_data_offset: 0,
            tile_data_length: 0,
            num_addressed_tiles: 0,
            num_tile_entries: 0,
            num_tile_content: 0,
            clustered: false,
            internal_compression: Compression::GZip,
            tile_compression: Compression::None,
            tile_type: TileType::Unknown,
            min_zoom: 0,
            max_zoom: 0,
            min_pos: LatLng {
                longitude: -180.0,
                latitude: -85.0,
            },
            max_pos: LatLng {
                longitude: 180.0,
                latitude: 85.0,
            },
            center_zoom: 0,
            center_pos: LatLng {
                longitude: 0.0,
                latitude: 0.0,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use deku::bitvec::{BitSlice, BitVec, BitView, Msb0};

    #[test]
    fn test_http_content_type() {
        let header = Header {
            tile_type: TileType::Mvt,
            ..Header::default()
        };

        assert_eq!(
            header.http_content_type(),
            TileType::Mvt.http_content_type()
        );
    }

    #[test]
    fn test_http_content_encoding() {
        let header = Header {
            internal_compression: Compression::Brotli,
            tile_compression: Compression::GZip,
            ..Header::default()
        };

        assert_eq!(
            header.http_content_encoding(),
            Compression::GZip.http_content_encoding()
        );
    }

    #[test]
    fn test_deku_read1() -> Result<(), DekuError> {
        let header_bytes = include_bytes!("../../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");
        let header_slice = BitSlice::<u8, Msb0>::from_slice(header_bytes);

        let (rest, header) = Header::read(header_slice, ())?;

        // header has to be exactly 127 bytes
        assert_eq!(rest.len(), header_slice.len() - 127 * 8);

        assert_eq!(header.spec_version, 3);
        assert_eq!(header.root_directory_offset, 127);
        assert_eq!(header.root_directory_length, 246);
        assert_eq!(header.json_metadata_offset, 373);
        assert_eq!(header.json_metadata_length, 22);
        assert_eq!(header.leaf_directories_offset, 395);
        assert_eq!(header.leaf_directories_length, 0);
        assert_eq!(header.tile_data_offset, 395);
        assert_eq!(header.tile_data_length, 715_657);
        assert_eq!(header.num_addressed_tiles, 85);
        assert_eq!(header.num_tile_entries, 84);
        assert_eq!(header.num_tile_content, 80);
        assert!(header.clustered);
        assert_eq!(header.internal_compression, Compression::GZip);
        assert_eq!(header.tile_compression, Compression::None);
        assert_eq!(header.tile_type, TileType::Png);
        assert_eq!(header.min_zoom, 0);
        assert_eq!(header.max_zoom, 3);
        assert_eq!(
            header.min_pos,
            LatLng {
                longitude: -180.0,
                latitude: -85.0
            }
        );
        assert_eq!(
            header.max_pos,
            LatLng {
                longitude: 180.0,
                latitude: 85.0
            }
        );
        assert_eq!(header.center_zoom, 0);
        assert_eq!(
            header.center_pos,
            LatLng {
                longitude: 0.0,
                latitude: 0.0
            }
        );

        Ok(())
    }

    #[test]
    fn test_deku_read2() -> Result<(), DekuError> {
        let header_bytes = include_bytes!("../../test/protomaps(vector)ODbL_firenze.pmtiles");
        let header_slice = BitSlice::<u8, Msb0>::from_slice(header_bytes);

        let (rest, header) = Header::read(header_slice, ())?;

        // header has to be exactly 127 bytes
        assert_eq!(rest.len(), header_slice.len() - 127 * 8);

        assert_eq!(header.spec_version, 3);
        assert_eq!(header.root_directory_offset, 127);
        assert_eq!(header.root_directory_length, 403);
        assert_eq!(header.json_metadata_offset, 530);
        assert_eq!(header.json_metadata_length, 266);
        assert_eq!(header.leaf_directories_offset, 796);
        assert_eq!(header.leaf_directories_length, 0);
        assert_eq!(header.tile_data_offset, 796);
        assert_eq!(header.tile_data_length, 3_938_905);
        assert_eq!(header.num_addressed_tiles, 108);
        assert_eq!(header.num_tile_entries, 108);
        assert_eq!(header.num_tile_content, 106);
        assert!(header.clustered);
        assert_eq!(header.internal_compression, Compression::GZip);
        assert_eq!(header.tile_compression, Compression::GZip);
        assert_eq!(header.tile_type, TileType::Mvt);
        assert_eq!(header.min_zoom, 0);
        assert_eq!(header.max_zoom, 14);
        assert_eq!(
            header.min_pos,
            LatLng {
                longitude: 11.154_026,
                latitude: 43.727_012_5
            }
        );
        assert_eq!(
            header.max_pos,
            LatLng {
                longitude: 11.328_939_5,
                latitude: 43.832_545_5
            }
        );
        assert_eq!(header.center_zoom, 0);
        assert_eq!(
            header.center_pos,
            LatLng {
                longitude: 11.241_482_7,
                latitude: 43.779_779
            }
        );

        Ok(())
    }

    #[test]
    fn test_deku_write() -> Result<(), DekuError> {
        let mut output = BitVec::new();
        Header {
            spec_version: 3,
            root_directory_offset: 127,
            root_directory_length: 246,
            json_metadata_offset: 373,
            json_metadata_length: 22,
            leaf_directories_offset: 395,
            leaf_directories_length: 0,
            tile_data_offset: 395,
            tile_data_length: 715_657,
            num_addressed_tiles: 85,
            num_tile_entries: 84,
            num_tile_content: 80,
            clustered: true,
            internal_compression: Compression::GZip,
            tile_compression: Compression::None,
            tile_type: TileType::Png,
            min_zoom: 0,
            max_zoom: 3,
            min_pos: LatLng {
                longitude: -180.0,
                latitude: -85.0,
            },
            max_pos: LatLng {
                longitude: 180.0,
                latitude: 85.0,
            },
            center_zoom: 0,
            center_pos: LatLng {
                longitude: 0.0,
                latitude: 0.0,
            },
        }
        .write(&mut output, ())?;

        assert_eq!(
            output,
            include_bytes!("../../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles")
                .split_at(127)
                .0
                .view_bits::<Msb0>()
        );

        Ok(())
    }
}
