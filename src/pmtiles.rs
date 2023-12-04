use std::{
    io::{Cursor, Read, Result, Seek, Write},
    ops::RangeBounds,
};

use duplicate::duplicate_item;
#[cfg(feature = "async")]
use futures::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use serde_json::{json, Value as JSONValue};

use crate::{
    header::{LatLng, HEADER_BYTES},
    tile_manager::TileManager,
    util::{compress, decompress, read_directories, tile_id, write_directories},
    Compression, Header, TileType,
};

#[cfg(feature = "async")]
use crate::util::{
    compress_async, decompress_async, read_directories_async, write_directories_async,
};

#[derive(Debug)]
/// A structure representing a `PMTiles` archive.
pub struct PMTiles<R> {
    /// Type of tiles
    pub tile_type: TileType,

    /// Compression of tiles
    pub tile_compression: Compression,

    /// Compression of directories and meta data
    pub internal_compression: Compression,

    /// Minimum zoom of all tiles this archive
    pub min_zoom: u8,

    /// Maximum zoom of all tiles this archive
    pub max_zoom: u8,

    /// Center zoom
    ///
    /// _Implementations may use this to set the default zoom_
    pub center_zoom: u8,

    /// Minimum longitude of bounds of available tiles
    pub min_longitude: f64,

    /// Minimum latitude of bounds of available tiles
    pub min_latitude: f64,

    /// Maximum longitude of bounds of available tiles
    pub max_longitude: f64,

    /// Maximum latitude of bounds of available tiles
    pub max_latitude: f64,

    /// Center longitude
    ///
    /// _Implementations may use the center longitude and latitude to set the default location_
    pub center_longitude: f64,

    /// Center latitude
    ///
    /// _Implementations may use the center longitude and latitude to set the default location_
    pub center_latitude: f64,

    /// JSON meta data of this archive
    pub meta_data: Option<JSONValue>,

    tile_manager: TileManager<R>,
}

impl<R> Default for PMTiles<R> {
    fn default() -> Self {
        Self {
            tile_type: TileType::Unknown,
            internal_compression: Compression::GZip,
            tile_compression: Compression::Unknown,
            min_zoom: 0,
            max_zoom: 0,
            center_zoom: 0,
            min_longitude: 0.0,
            min_latitude: 0.0,
            max_longitude: 0.0,
            max_latitude: 0.0,
            center_longitude: 0.0,
            center_latitude: 0.0,
            meta_data: None,
            tile_manager: TileManager::<R>::new(None),
        }
    }
}

impl PMTiles<Cursor<&[u8]>> {
    /// Constructs a new, empty `PMTiles` archive, with no meta data, an [`internal_compression`](Self::internal_compression) of GZIP and all numeric fields set to `0`.
    ///
    /// # Arguments
    /// * `tile_type` - Type of tiles in this archive
    /// * `tile_compression` - Compression of tiles in this archive
    pub fn new(tile_type: TileType, tile_compression: Compression) -> Self {
        Self {
            tile_type,
            tile_compression,
            ..Default::default()
        }
    }
}

#[cfg(feature = "async")]
impl PMTiles<futures::io::Cursor<&[u8]>> {
    /// Async version of [`new`](Self::new).
    ///
    /// Constructs a new, empty `PMTiles` archive, that works with asynchronous readers / writers.
    ///
    /// # Arguments
    /// * `tile_type` - Type of tiles in this archive
    /// * `tile_compression` - Compression of tiles in this archive
    pub fn new_async(tile_type: TileType, tile_compression: Compression) -> Self {
        Self {
            tile_type,
            tile_compression,
            ..Default::default()
        }
    }
}

impl<R> PMTiles<R> {
    /// Get vector of all tile ids in this `PMTiles` archive.
    pub fn tile_ids(&self) -> Vec<&u64> {
        self.tile_manager.get_tile_ids()
    }

    /// Adds a tile to this `PMTiles` archive.
    ///
    /// Note that the data should already be compressed if [`Self::tile_compression`] is set to a value other than [`Compression::None`].
    /// The data will **NOT** be compressed automatically.  
    /// The [`util`-module](crate::util) includes utilities to compress data.
    pub fn add_tile(&mut self, tile_id: u64, data: impl Into<Vec<u8>>) {
        self.tile_manager.add_tile(tile_id, data);
    }

    /// Removes a tile from this archive.
    pub fn remove_tile(&mut self, tile_id: u64) {
        self.tile_manager.remove_tile(tile_id);
    }

    /// Returns the number of addressed tiles in this archive.
    pub fn num_tiles(&self) -> usize {
        self.tile_manager.num_addressed_tiles()
    }
}

impl<R: Read + Seek> PMTiles<R> {
    /// Get data of a tile by its id.
    ///
    /// The returned data is the raw data, meaning It is NOT uncompressed automatically,
    /// if it was compressed in the first place.  
    /// If you need the uncompressed data, take a look at the [`util`-module](crate::util)
    ///
    /// Will return [`Ok`] with an value of [`None`] if no a tile with the specified tile id was found.
    ///
    /// # Errors
    /// Will return [`Err`] if the tile data was not read into memory yet and there was an error while
    /// attempting to read it.
    ///
    pub fn get_tile_by_id(&mut self, tile_id: u64) -> Result<Option<Vec<u8>>> {
        self.tile_manager.get_tile(tile_id)
    }

    /// Returns the data of the tile with the specified coordinates.
    ///
    /// See [`get_tile_by_id`](Self::get_tile_by_id) for further details on the return type.
    ///
    /// # Errors
    /// See [`get_tile_by_id`](Self::get_tile_by_id) for details on possible errors.
    pub fn get_tile(&mut self, x: u64, y: u64, z: u8) -> Result<Option<Vec<u8>>> {
        self.get_tile_by_id(tile_id(z, x, y))
    }
}

#[cfg(feature = "async")]
impl<R: AsyncRead + AsyncReadExt + Send + Unpin + AsyncSeekExt> PMTiles<R> {
    /// Async version of [`get_tile_by_id`](Self::get_tile_by_id).
    ///
    /// Get data of a tile by its id.
    ///
    /// The returned data is the raw data, meaning It is NOT uncompressed automatically,
    /// if it was compressed in the first place.  
    /// If you need the uncompressed data, take a look at the [`util`-module](crate::util)
    ///
    /// Will return [`Ok`] with an value of [`None`] if no a tile with the specified tile id was found.
    ///
    /// # Errors
    /// Will return [`Err`] if the tile data was not read into memory yet and there was an error while
    /// attempting to read it.
    ///
    pub async fn get_tile_by_id_async(&mut self, tile_id: u64) -> Result<Option<Vec<u8>>> {
        self.tile_manager.get_tile_async(tile_id).await
    }

    /// Async version of [`get_tile`](Self::get_tile).
    ///
    /// Returns the data of the tile with the specified coordinates.
    ///
    /// See [`get_tile_by_id_async`](Self::get_tile_by_id_async) for further details on the return type.
    ///
    /// # Errors
    /// See [`get_tile_by_id_async`](Self::get_tile_by_id_async) for details on possible errors.
    pub async fn get_tile_async(&mut self, x: u64, y: u64, z: u8) -> Result<Option<Vec<u8>>> {
        self.get_tile_by_id_async(tile_id(z, x, y)).await
    }
}

impl<R: Read + Seek> PMTiles<R> {
    fn parse_meta_data(compression: Compression, reader: &mut impl Read) -> Result<JSONValue> {
        let reader = decompress(compression, reader)?;

        let val: JSONValue = serde_json::from_reader(reader)?;

        Ok(val)
    }
}

#[cfg(feature = "async")]
impl<R: AsyncRead + AsyncSeekExt + Send + Unpin> PMTiles<R> {
    async fn parse_meta_data_async(
        compression: Compression,
        reader: &mut (impl AsyncRead + Unpin + Send),
    ) -> Result<JSONValue> {
        let mut reader = decompress_async(compression, reader)?;

        let mut output = Vec::with_capacity(2048);
        reader.read_to_end(&mut output).await?;

        let val: JSONValue = serde_json::from_slice(&output[..])?;

        Ok(val)
    }
}

#[duplicate_item(
    fn_name                  cfg_async_filter       async    add_await(code) SeekFrom                FilterRangeTraits                RTraits                                                  read_directories         parse_meta_data         from_reader;
    [from_reader_impl]       [cfg(all())]           []       [code]          [std::io::SeekFrom]     [RangeBounds<u64>]               [Read + Seek]                                            [read_directories]       [parse_meta_data]       [from_reader];
    [from_async_reader_impl] [cfg(feature="async")] [async]  [code.await]    [futures::io::SeekFrom] [RangeBounds<u64> + Sync + Send] [AsyncRead + AsyncReadExt + Send + Unpin + AsyncSeekExt] [read_directories_async] [parse_meta_data_async] [from_async_reader];
)]
#[cfg_async_filter]
impl<R: RTraits> PMTiles<R> {
    async fn fn_name(mut input: R, tiles_filter_range: impl FilterRangeTraits) -> Result<Self> {
        // HEADER
        let header = add_await([Header::from_reader(&mut input)])?;

        // META DATA
        let meta_data = if header.json_metadata_length == 0 {
            None
        } else {
            add_await([input.seek(SeekFrom::Start(header.json_metadata_offset))])?;

            let mut meta_data_reader = (&mut input).take(header.json_metadata_length);
            Some(add_await([Self::parse_meta_data(
                header.internal_compression,
                &mut meta_data_reader,
            )])?)
        };

        // DIRECTORIES
        let tiles = add_await([read_directories(
            &mut input,
            header.internal_compression,
            (header.root_directory_offset, header.root_directory_length),
            header.leaf_directories_offset,
            tiles_filter_range,
        )])?;

        let mut tile_manager = TileManager::new(Some(input));

        for (tile_id, info) in tiles {
            tile_manager.add_offset_tile(
                tile_id,
                header.tile_data_offset + info.offset,
                info.length,
            );
        }

        Ok(Self {
            tile_type: header.tile_type,
            internal_compression: header.internal_compression,
            tile_compression: header.tile_compression,
            min_zoom: header.min_zoom,
            max_zoom: header.max_zoom,
            center_zoom: header.center_zoom,
            min_longitude: header.min_pos.longitude,
            min_latitude: header.min_pos.latitude,
            max_longitude: header.max_pos.longitude,
            max_latitude: header.max_pos.latitude,
            center_longitude: header.center_pos.longitude,
            center_latitude: header.center_pos.latitude,
            meta_data,
            tile_manager,
        })
    }
}

#[duplicate_item(
    fn_name                cfg_async_filter       async    add_await(code) RTraits                                                  SeekFrom                WTraits                                    finish         compress         write_directories         to_writer;
    [to_writer_impl]       [cfg(all())]           []       [code]          [Read + Seek]                                            [std::io::SeekFrom]     [Write + Seek]                             [finish]       [compress]       [write_directories]       [to_writer];
    [to_async_writer_impl] [cfg(feature="async")] [async]  [code.await]    [AsyncRead + AsyncReadExt + Send + Unpin + AsyncSeekExt] [futures::io::SeekFrom] [AsyncWrite + Send + Unpin + AsyncSeekExt] [finish_async] [compress_async] [write_directories_async] [to_async_writer];
)]
#[cfg_async_filter]
impl<R: RTraits> PMTiles<R> {
    #[allow(clippy::wrong_self_convention)]
    async fn fn_name(self, output: &mut (impl WTraits)) -> Result<()> {
        let result = add_await([self.tile_manager.finish()])?;

        // ROOT DIR
        add_await([output.seek(SeekFrom::Current(i64::from(HEADER_BYTES)))])?;
        let root_directory_offset = u64::from(HEADER_BYTES);
        let leaf_directories_data = add_await([write_directories(
            output,
            &result.directory[0..],
            self.internal_compression,
            None,
        )])?;
        let root_directory_length = add_await([output.stream_position()])? - root_directory_offset;

        // META DATA
        let json_metadata_offset = root_directory_offset + root_directory_length;
        {
            let meta_val = self.meta_data.unwrap_or_else(|| json!({}));
            let mut compression_writer = compress(self.internal_compression, output)?;
            let vec = serde_json::to_vec(&meta_val)?;
            add_await([compression_writer.write_all(&vec)])?;

            add_await([compression_writer.flush()])?;
        }
        let json_metadata_length = add_await([output.stream_position()])? - json_metadata_offset;

        // LEAF DIRECTORIES
        let leaf_directories_offset = json_metadata_offset + json_metadata_length;
        add_await([output.write_all(&leaf_directories_data[0..])])?;
        drop(leaf_directories_data);
        let leaf_directories_length =
            add_await([output.stream_position()])? - leaf_directories_offset;

        // DATA
        let tile_data_offset = leaf_directories_offset + leaf_directories_length;
        add_await([output.write_all(&result.data[0..])])?;
        let tile_data_length = result.data.len() as u64;

        // HEADER
        let header = Header {
            spec_version: 3,
            root_directory_offset,
            root_directory_length,
            json_metadata_offset,
            json_metadata_length,
            leaf_directories_offset,
            leaf_directories_length,
            tile_data_offset,
            tile_data_length,
            num_addressed_tiles: result.num_addressed_tiles,
            num_tile_entries: result.num_tile_entries,
            num_tile_content: result.num_tile_content,
            clustered: true,
            internal_compression: self.internal_compression,
            tile_compression: self.tile_compression,
            tile_type: self.tile_type,
            min_zoom: self.min_zoom,
            max_zoom: self.max_zoom,
            min_pos: LatLng {
                longitude: self.min_longitude,
                latitude: self.min_latitude,
            },
            max_pos: LatLng {
                longitude: self.max_longitude,
                latitude: self.max_latitude,
            },
            center_zoom: self.center_zoom,
            center_pos: LatLng {
                longitude: self.center_longitude,
                latitude: self.center_latitude,
            },
        };

        add_await([output.seek(SeekFrom::Start(
            root_directory_offset - u64::from(HEADER_BYTES),
        ))])?; // jump to start of stream

        add_await([header.to_writer(output)])?;

        add_await([output.seek(SeekFrom::Start(
            (root_directory_offset - u64::from(HEADER_BYTES)) + tile_data_offset + tile_data_length,
        ))])?; // jump to end of stream

        Ok(())
    }
}

impl<R: Read + Seek> PMTiles<R> {
    /// Reads a `PMTiles` archive from a reader.
    ///
    /// This takes ownership of the reader, because tile data is only read when required.
    ///
    /// # Arguments
    /// * `input` - Reader
    ///
    /// # Errors
    /// Will return [`Err`] if there was any kind of I/O error while reading from `input`, the data
    /// stream was no valid `PMTiles` archive or the internal compression of the archive is set to "Unknown".
    ///
    ///
    /// # Example
    /// ```rust
    /// # use pmtiles2::{PMTiles};
    /// # let file_path = "./test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles";
    /// let mut file = std::fs::File::open(file_path).unwrap();
    ///
    /// let pm_tiles = PMTiles::from_reader(file).unwrap();
    /// ```
    pub fn from_reader(input: R) -> Result<Self> {
        Self::from_reader_impl(input, ..)
    }

    /// Same as [`from_reader`](Self::from_reader), but with an extra parameter.
    ///
    /// Reads a `PMTiles` archive from a reader, but only parses tile entries whose tile IDs are included in the filter
    /// range. Tiles that are not included in the range will appear as missing.
    ///
    /// This can improve performance in cases where only a limited range of tiles is needed, as whole leaf directories
    /// may be skipped during parsing.
    ///
    /// # Arguments
    /// * `input` - Reader
    /// * `tiles_filter_range` - Range of Tile IDs to load
    ///
    /// # Errors
    /// See [`from_reader`](Self::from_reader) for details on possible errors.
    ///
    /// # Example
    /// ```rust
    /// # use pmtiles2::{PMTiles};
    /// # let file_path = "./test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles";
    /// let mut file = std::fs::File::open(file_path).unwrap();
    ///
    /// let pm_tiles = PMTiles::from_reader_partially(file, ..).unwrap();
    /// ```
    pub fn from_reader_partially(
        input: R,
        tiles_filter_range: impl RangeBounds<u64>,
    ) -> Result<Self> {
        Self::from_reader_impl(input, tiles_filter_range)
    }

    /// Writes the archive to a writer.
    ///
    /// The archive is always deduped and the directory entries clustered to produce the smallest
    /// possible archive size.
    ///
    /// This takes ownership of the object so all data does not need to be copied.
    /// This prevents large memory consumption when writing large `PMTiles` archives.  
    ///
    /// # Arguments
    /// * `output` - Writer to write data to
    ///
    /// # Errors
    /// Will return [`Err`] if [`Self::internal_compression`] was set to [`Compression::Unknown`]
    /// or an I/O error occurred while writing to `output`.
    ///
    /// # Example
    /// Write the archive to a file.
    /// ```rust
    /// # use pmtiles2::{PMTiles, TileType, Compression};
    /// # let dir = temp_dir::TempDir::new().unwrap();
    /// # let file_path = dir.path().join("foo.pmtiles");
    /// let pm_tiles = PMTiles::new(TileType::Png, Compression::None);
    /// let mut file = std::fs::File::create(file_path).unwrap();
    /// pm_tiles.to_writer(&mut file).unwrap();
    /// ```
    pub fn to_writer(self, output: &mut (impl Write + Seek)) -> Result<()> {
        self.to_writer_impl(output)
    }
}

impl<T: AsRef<[u8]>> PMTiles<Cursor<T>> {
    /// Reads a `PMTiles` archive from anything that can be turned into a byte slice (e.g. [`Vec<u8>`]).
    ///
    /// # Arguments
    /// * `bytes` - Input bytes
    ///
    /// # Errors
    /// Will return [`Err`] if there was any kind of I/O error while reading from `input`, the data
    /// stream was no valid `PMTiles` archive or the internal compression of the archive is set to "Unknown".
    ///
    /// # Example
    /// ```rust
    /// # use pmtiles2::{PMTiles};
    /// let bytes = include_bytes!("../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");
    /// let pm_tiles = PMTiles::from_bytes(bytes).unwrap();
    /// ```
    ///
    pub fn from_bytes(bytes: T) -> std::io::Result<Self> {
        let reader = std::io::Cursor::new(bytes);

        Self::from_reader(reader)
    }

    /// Same as [`from_bytes`](Self::from_bytes), but with an extra parameter.
    ///
    /// Reads a `PMTiles` archive from something that can be turned into a byte slice (e.g. [`Vec<u8>`]),
    /// but only parses tile entries whose tile IDs are included in the filter range. Tiles that are not
    /// included in the range will appear as missing.
    ///
    /// This can improve performance in cases where only a limited range of tiles is needed, as whole leaf directories
    /// may be skipped during parsing.
    ///
    /// # Arguments
    /// * `bytes` - Input bytes
    /// * `tiles_filter_range` - Range of Tile IDs to load
    ///
    /// # Errors
    /// See [`from_bytes`](Self::from_bytes) for details on possible errors.
    ///
    /// # Example
    /// ```rust
    /// # use pmtiles2::{PMTiles};
    /// let bytes = include_bytes!("../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");
    /// let pm_tiles = PMTiles::from_bytes_partially(bytes, ..).unwrap();
    /// ```
    pub fn from_bytes_partially(
        bytes: T,
        tiles_filter_range: impl RangeBounds<u64>,
    ) -> Result<Self> {
        let reader = std::io::Cursor::new(bytes);

        Self::from_reader_partially(reader, tiles_filter_range)
    }
}

#[cfg(feature = "async")]
impl<R: AsyncRead + AsyncSeekExt + Send + Unpin> PMTiles<R> {
    /// Async version of [`from_reader`](Self::from_reader).
    ///
    /// Reads a `PMTiles` archive from a reader.
    ///
    /// This takes ownership of the reader, because tile data is only read when required.
    ///
    /// # Arguments
    /// * `input` - Reader
    ///
    /// # Errors
    /// Will return [`Err`] if there was any kind of I/O error while reading from `input`, the data
    /// stream was no valid `PMTiles` archive or the internal compression of the archive is set to "Unknown".
    ///
    ///
    /// # Example
    /// ```rust
    /// # use pmtiles2::PMTiles;
    /// # use futures::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
    /// # tokio_test::block_on(async {
    /// let bytes = include_bytes!("../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");
    /// let mut reader = futures::io::Cursor::new(bytes);
    ///
    /// let pm_tiles = PMTiles::from_async_reader(reader).await.unwrap();
    /// # })
    /// ```
    pub async fn from_async_reader(input: R) -> Result<Self> {
        Self::from_async_reader_impl(input, ..).await
    }

    /// Same as [`from_async_reader`](Self::from_async_reader), but with an extra parameter.
    ///
    /// Reads a `PMTiles` archive from a reader, but only parses tile entries whose tile IDs are included in the filter
    /// range. Tiles that are not included in the range will appear as missing.
    ///
    /// This can improve performance in cases where only a limited range of tiles is needed, as whole leaf directories
    /// may be skipped during parsing.
    ///
    /// # Arguments
    /// * `input` - Reader
    /// * `tiles_filter_range` - Range of Tile IDs to load
    ///
    /// # Errors
    /// See [`from_async_reader`](Self::from_async_reader) for details on possible errors.
    ///
    /// # Example
    /// ```rust
    /// # use pmtiles2::PMTiles;
    /// # use futures::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
    /// # tokio_test::block_on(async {
    /// let bytes = include_bytes!("../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");
    /// let mut reader = futures::io::Cursor::new(bytes);
    ///
    /// let pm_tiles = PMTiles::from_async_reader_partially(reader, ..).await.unwrap();
    /// # })
    /// ```
    pub async fn from_async_reader_partially(
        input: R,
        tiles_filter_range: (impl RangeBounds<u64> + Sync + Send),
    ) -> Result<Self> {
        Self::from_async_reader_impl(input, tiles_filter_range).await
    }

    /// Async version of [`to_writer`](Self::to_writer).
    ///
    /// Writes the archive to a writer.
    ///
    /// The archive is always deduped and the directory entries clustered to produce the smallest
    /// possible archive size.
    ///
    /// This takes ownership of the object so all data does not need to be copied.
    /// This prevents large memory consumption when writing large `PMTiles` archives.  
    ///
    /// # Arguments
    /// * `output` - Writer to write data to
    ///
    /// # Errors
    /// Will return [`Err`] if [`Self::internal_compression`] was set to [`Compression::Unknown`]
    /// or an I/O error occurred while writing to `output`.
    ///
    /// # Example
    /// Write the archive to a file.
    /// ```rust
    /// # use pmtiles2::{PMTiles, TileType, Compression};
    /// # use futures::io::{AsyncWrite, AsyncWriteExt, AsyncSeekExt};
    /// # use tokio_util::compat::TokioAsyncReadCompatExt;
    /// # let dir = temp_dir::TempDir::new().unwrap();
    /// # let file_path = dir.path().join("foo.pmtiles");
    /// # tokio_test::block_on(async {
    /// let pm_tiles = PMTiles::new_async(TileType::Png, Compression::None);
    /// let mut out_file = tokio::fs::File::create(file_path).await.unwrap().compat();
    /// pm_tiles.to_async_writer(&mut out_file).await.unwrap();
    /// # })
    /// ```
    pub async fn to_async_writer(
        self,
        output: &mut (impl AsyncWrite + AsyncSeekExt + Unpin + Send),
    ) -> Result<()> {
        self.to_async_writer_impl(output).await
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use serde_json::json;

    use super::*;

    const PM_TILES_BYTES: &[u8] =
        include_bytes!("../test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles");

    const PM_TILES_BYTES2: &[u8] = include_bytes!("../test/protomaps(vector)ODbL_firenze.pmtiles");

    #[test]
    fn test_parse_meta_data() -> Result<()> {
        let meta_data = PMTiles::<Cursor<Vec<u8>>>::parse_meta_data(
            Compression::GZip,
            &mut Cursor::new(&PM_TILES_BYTES[373..373 + 22]),
        )?;
        assert_eq!(meta_data, json!({}));

        let meta_data2 = PMTiles::<Cursor<Vec<u8>>>::parse_meta_data(
            Compression::GZip,
            &mut Cursor::new(&PM_TILES_BYTES2[530..530 + 266]),
        )?;

        assert_eq!(
            meta_data2,
            json!({
                "attribution":"<a href=\"https://protomaps.com\" target=\"_blank\">Protomaps</a> © <a href=\"https://www.openstreetmap.org\" target=\"_blank\"> OpenStreetMap</a>",
                "tilestats":{
                    "layers":[
                        {"geometry":"Polygon","layer":"earth"},
                        {"geometry":"Polygon","layer":"natural"},
                        {"geometry":"Polygon","layer":"land"},
                        {"geometry":"Polygon","layer":"water"},
                        {"geometry":"LineString","layer":"physical_line"},
                        {"geometry":"Polygon","layer":"buildings"},
                        {"geometry":"Point","layer":"physical_point"},
                        {"geometry":"Point","layer":"places"},
                        {"geometry":"LineString","layer":"roads"},
                        {"geometry":"LineString","layer":"transit"},
                        {"geometry":"Point","layer":"pois"},
                        {"geometry":"LineString","layer":"boundaries"},
                        {"geometry":"Polygon","layer":"mask"}
                    ]
                }
            })
        );

        Ok(())
    }

    #[test]
    fn test_from_reader() -> Result<()> {
        let mut reader = Cursor::new(PM_TILES_BYTES);

        let pm_tiles = PMTiles::from_reader(&mut reader)?;

        assert_eq!(pm_tiles.tile_type, TileType::Png);
        assert_eq!(pm_tiles.internal_compression, Compression::GZip);
        assert_eq!(pm_tiles.tile_compression, Compression::None);
        assert_eq!(pm_tiles.min_zoom, 0);
        assert_eq!(pm_tiles.max_zoom, 3);
        assert_eq!(pm_tiles.center_zoom, 0);
        assert!((-180.0 - pm_tiles.min_longitude).abs() < f64::EPSILON);
        assert!((-85.0 - pm_tiles.min_latitude).abs() < f64::EPSILON);
        assert!((180.0 - pm_tiles.max_longitude).abs() < f64::EPSILON);
        assert!((85.0 - pm_tiles.max_latitude).abs() < f64::EPSILON);
        assert!(pm_tiles.center_longitude < f64::EPSILON);
        assert!(pm_tiles.center_latitude < f64::EPSILON);
        assert_eq!(pm_tiles.meta_data, Some(json!({})));
        assert_eq!(pm_tiles.num_tiles(), 85);

        Ok(())
    }

    #[test]
    fn test_from_reader2() -> Result<()> {
        let mut reader = std::fs::File::open("./test/protomaps(vector)ODbL_firenze.pmtiles")?;

        let pm_tiles = PMTiles::from_reader(&mut reader)?;

        assert_eq!(pm_tiles.tile_type, TileType::Mvt);
        assert_eq!(pm_tiles.internal_compression, Compression::GZip);
        assert_eq!(pm_tiles.tile_compression, Compression::GZip);
        assert_eq!(pm_tiles.min_zoom, 0);
        assert_eq!(pm_tiles.max_zoom, 14);
        assert_eq!(pm_tiles.center_zoom, 0);
        assert!((pm_tiles.min_longitude - 11.154_026).abs() < f64::EPSILON);
        assert!((pm_tiles.min_latitude - 43.727_012_5).abs() < f64::EPSILON);
        assert!((pm_tiles.max_longitude - 11.328_939_5).abs() < f64::EPSILON);
        assert!((pm_tiles.max_latitude - 43.832_545_5).abs() < f64::EPSILON);
        assert!((pm_tiles.center_longitude - 11.241_482_7).abs() < f64::EPSILON);
        assert!((pm_tiles.center_latitude - 43.779_779).abs() < f64::EPSILON);
        assert_eq!(
            pm_tiles.meta_data,
            Some(json!({
                "attribution":"<a href=\"https://protomaps.com\" target=\"_blank\">Protomaps</a> © <a href=\"https://www.openstreetmap.org\" target=\"_blank\"> OpenStreetMap</a>",
                "tilestats":{
                    "layers":[
                        {"geometry":"Polygon","layer":"earth"},
                        {"geometry":"Polygon","layer":"natural"},
                        {"geometry":"Polygon","layer":"land"},
                        {"geometry":"Polygon","layer":"water"},
                        {"geometry":"LineString","layer":"physical_line"},
                        {"geometry":"Polygon","layer":"buildings"},
                        {"geometry":"Point","layer":"physical_point"},
                        {"geometry":"Point","layer":"places"},
                        {"geometry":"LineString","layer":"roads"},
                        {"geometry":"LineString","layer":"transit"},
                        {"geometry":"Point","layer":"pois"},
                        {"geometry":"LineString","layer":"boundaries"},
                        {"geometry":"Polygon","layer":"mask"}
                    ]
                }
            }))
        );
        assert_eq!(pm_tiles.num_tiles(), 108);

        Ok(())
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_from_reader3() -> Result<()> {
        let mut reader =
            std::fs::File::open("./test/protomaps_vector_planet_odbl_z10_without_data.pmtiles")?;

        let pm_tiles = PMTiles::from_reader(&mut reader)?;

        assert_eq!(pm_tiles.tile_type, TileType::Mvt);
        assert_eq!(pm_tiles.internal_compression, Compression::GZip);
        assert_eq!(pm_tiles.tile_compression, Compression::GZip);
        assert_eq!(pm_tiles.min_zoom, 0);
        assert_eq!(pm_tiles.max_zoom, 10);
        assert_eq!(pm_tiles.center_zoom, 0);
        assert!((-180.0 - pm_tiles.min_longitude).abs() < f64::EPSILON);
        assert!((-90.0 - pm_tiles.min_latitude).abs() < f64::EPSILON);
        assert!((180.0 - pm_tiles.max_longitude).abs() < f64::EPSILON);
        assert!((90.0 - pm_tiles.max_latitude).abs() < f64::EPSILON);
        assert!(pm_tiles.center_longitude < f64::EPSILON);
        assert!(pm_tiles.center_latitude < f64::EPSILON);
        assert_eq!(
            pm_tiles.meta_data,
            Some(json!({
                "attribution": "<a href=\"https://protomaps.com\" target=\"_blank\">Protomaps</a> © <a href=\"https://www.openstreetmap.org\" target=\"_blank\"> OpenStreetMap</a>",
                "name": "protomaps 2022-11-08T03:35:13Z",
                "tilestats": {
                    "layers": [
                        { "geometry": "Polygon", "layer": "earth" },
                        { "geometry": "Polygon", "layer": "natural" },
                        { "geometry": "Polygon", "layer": "land" },
                        { "geometry": "Polygon", "layer": "water" },
                        { "geometry": "LineString", "layer": "physical_line" },
                        { "geometry": "Polygon", "layer": "buildings" },
                        { "geometry": "Point", "layer": "physical_point" },
                        { "geometry": "Point", "layer": "places" },
                        { "geometry": "LineString", "layer": "roads" },
                        { "geometry": "LineString", "layer": "transit" },
                        { "geometry": "Point", "layer": "pois" },
                        { "geometry": "LineString", "layer": "boundaries" },
                        { "geometry": "Polygon", "layer": "mask" }
                    ]
                },
                "vector_layers": [
                    {
                        "fields": {},
                        "id": "earth"
                    },
                    {
                        "fields": {
                            "boundary": "string",
                            "landuse": "string",
                            "leisure": "string",
                            "name": "string",
                            "natural": "string"
                        },
                        "id": "natural"
                    },
                    {
                        "fields": {
                            "aeroway": "string",
                            "amenity": "string",
                            "area:aeroway": "string",
                            "highway": "string",
                            "landuse": "string",
                            "leisure": "string",
                            "man_made": "string",
                            "name": "string",
                            "place": "string",
                            "pmap:kind": "string",
                            "railway": "string",
                            "sport": "string"
                        },
                        "id": "land"
                    },
                    {
                        "fields": {
                            "landuse": "string",
                            "leisure": "string",
                            "name": "string",
                            "natural": "string",
                            "water": "string",
                            "waterway": "string"
                        },
                        "id": "water"
                    },
                    {
                        "fields": {
                            "natural": "string",
                            "waterway": "string"
                        },
                        "id": "physical_line"
                    },
                    {
                        "fields": {
                            "building:part": "string",
                            "height": "number",
                            "layer": "string",
                            "name": "string"
                        },
                        "id": "buildings"
                    },
                    {
                        "fields": {
                            "ele": "number",
                            "name": "string",
                            "natural": "string",
                            "place": "string"
                        },
                        "id": "physical_point"
                    },
                    {
                        "fields": {
                            "capital": "string",
                            "country_code_iso3166_1_alpha_2": "string",
                            "name": "string",
                            "place": "string",
                            "pmap:kind": "string",
                            "pmap:rank": "string",
                            "population": "string"
                        },
                        "id": "places"
                    },
                    {
                        "fields": {
                            "bridge": "string",
                            "highway": "string",
                            "layer": "string",
                            "oneway": "string",
                            "pmap:kind": "string",
                            "ref": "string",
                            "tunnel": "string"
                        },
                        "id": "roads"
                    },
                    {
                        "fields": {
                            "aerialway": "string",
                            "aeroway": "string",
                            "highspeed": "string",
                            "layer": "string",
                            "name": "string",
                            "network": "string",
                            "pmap:kind": "string",
                            "railway": "string",
                            "ref": "string",
                            "route": "string",
                            "service": "string"
                        },
                        "id": "transit"
                    },
                    {
                        "fields": {
                            "amenity": "string",
                            "cuisine": "string",
                            "name": "string",
                            "railway": "string",
                            "religion": "string",
                            "shop": "string",
                            "tourism": "string"
                        },
                        "id": "pois"
                    },
                    {
                        "fields": {
                            "pmap:min_admin_level": "number"
                        },
                        "id": "boundaries"
                    },
                    {
                        "fields": {},
                        "id": "mask"
                    }
                ]
            }))
        );
        assert_eq!(pm_tiles.num_tiles(), 1_398_101);

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_to_writer() -> Result<()> {
        todo!()
    }

    #[test]
    #[ignore]
    fn test_to_writer_with_leaf_directories() -> Result<()> {
        todo!()
    }
}
