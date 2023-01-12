use std::io::{Cursor, Read, Result, Seek, SeekFrom, Write};

use deku::{
    bitvec::{BitVec, BitView},
    DekuRead, DekuWrite,
};
use serde_json::{json, Value as JSONValue};

use crate::{
    header::LatLng,
    tile_manager::TileManager,
    util::{compress, decompress, read_directories, tile_id, write_directories},
    Compression, Header, TileType,
};

#[derive(Debug)]
/// A structure representing a `PMTiles` archive.
pub struct PMTiles<R>
where
    R: Read + Seek,
{
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

pub const HEADER_BYTES: u8 = 127;

impl PMTiles<Cursor<&[u8]>> {
    /// Constructs a new, empty `PMTiles` archive, with no meta data, an [`internal_compression`](Self::internal_compression) of GZIP and all numeric fields set to `0`.
    ///
    /// # Arguments
    /// * `tile_type` - Type of tiles in this archive
    /// * `tile_compression` - Compression of tiles in this archive
    pub fn new(tile_type: TileType, tile_compression: Compression) -> Self {
        Self {
            tile_type,
            internal_compression: Compression::GZip,
            tile_compression,
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
            tile_manager: TileManager::new(None),
        }
    }
}

impl Default for PMTiles<Cursor<Vec<u8>>> {
    fn default() -> Self {
        Self {
            tile_type: TileType::Unknown,
            internal_compression: Compression::None,
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
            tile_manager: TileManager::default(),
        }
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

    /// Get vector of all tile ids in this `PMTiles` archive.
    pub fn tile_ids(&self) -> Vec<&u64> {
        self.tile_manager.get_tile_ids()
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
    #[allow(clippy::cast_possible_truncation)]
    fn get_section(reader: &mut R, byte_offset: u64, byte_length: u64) -> Result<Vec<u8>> {
        reader.seek(SeekFrom::Start(byte_offset))?;
        let mut buf = vec![0; byte_length as usize];
        reader.read_exact(&mut buf)?;

        Ok(buf)
    }

    fn parse_meta_data(compression: Compression, reader: &mut impl Read) -> Result<JSONValue> {
        let reader = decompress(compression, reader)?;

        let val: JSONValue = serde_json::from_reader(reader)?;

        Ok(val)
    }

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
    ///
    pub fn from_reader(mut input: R) -> Result<Self> {
        // HEADER
        let header_section = Self::get_section(&mut input, 0, u64::from(HEADER_BYTES))?;
        let (_, header) = Header::read(header_section.view_bits(), ())?;

        // META DATA
        let meta_data = if header.json_metadata_length == 0 {
            None
        } else {
            input.seek(SeekFrom::Start(header.json_metadata_offset))?;

            let mut meta_data_reader = (&mut input).take(header.json_metadata_length);
            Some(Self::parse_meta_data(
                header.internal_compression,
                &mut meta_data_reader,
            )?)
        };

        // DIRECTORIES
        let tiles = read_directories(
            &mut input,
            header.internal_compression,
            (header.root_directory_offset, header.root_directory_length),
            header.leaf_directories_offset,
        )?;

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

impl<R: Read + Seek> PMTiles<R> {
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
    /// or there was any kind of I/O error while writing to `output`.
    ///
    /// # Example
    /// Write the archive to a file.
    /// ```rust
    /// # use pmtiles2::{PMTiles};
    /// # let dir = tempdir::TempDir::new("pmtiles").unwrap();
    /// # let file_path = dir.path().join("foo.pmtiles");
    /// let pm_tiles = PMTiles::default();
    /// let mut file = std::fs::File::create(file_path).unwrap();
    /// pm_tiles.to_writer(&mut file).unwrap();
    /// ```
    ///
    pub fn to_writer(self, output: &mut (impl Write + Seek)) -> Result<()> {
        let result = self.tile_manager.finish()?;

        // ROOT DIR
        output.seek(SeekFrom::Current(i64::from(HEADER_BYTES)))?;
        let root_directory_offset = u64::from(HEADER_BYTES);
        let leaf_directories_data = write_directories(
            output,
            &result.directory[0..],
            self.internal_compression,
            None,
        )?;
        let root_directory_length = output.stream_position()? - root_directory_offset;

        // META DATA
        let json_metadata_offset = root_directory_offset + root_directory_length;
        {
            let meta_val = self.meta_data.unwrap_or_else(|| json!({}));
            let mut compression_writer = compress(self.internal_compression, output)?;
            serde_json::to_writer(&mut compression_writer, &meta_val)?;
        }
        let json_metadata_length = output.stream_position()? - json_metadata_offset;

        // LEAF DIRECTORIES
        let leaf_directories_offset = json_metadata_offset + json_metadata_length;
        output.write_all(&leaf_directories_data[0..])?;
        drop(leaf_directories_data);
        let leaf_directories_length = output.stream_position()? - leaf_directories_offset;

        // DATA
        let tile_data_offset = leaf_directories_offset + leaf_directories_length;
        output.write_all(&result.data[0..])?;
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

        output.seek(SeekFrom::Start(
            root_directory_offset - u64::from(HEADER_BYTES),
        ))?; // jump to start of stream

        let mut bit_vec = BitVec::with_capacity(8 * HEADER_BYTES as usize);
        header.write(&mut bit_vec, ())?;
        output.write_all(bit_vec.as_raw_slice())?;

        output.seek(SeekFrom::Start(
            (root_directory_offset - u64::from(HEADER_BYTES)) + tile_data_offset + tile_data_length,
        ))?; // jump to end of stream

        Ok(())
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
    fn test_get_section() -> Result<()> {
        let mut reader = Cursor::new(PM_TILES_BYTES);

        let byte_offset = 127usize;
        let byte_length = 300usize;

        let section = PMTiles::get_section(&mut reader, byte_offset as u64, byte_length as u64)?;

        assert_eq!(
            section,
            &PM_TILES_BYTES[byte_offset..byte_offset + byte_length]
        );

        let res =
            PMTiles::get_section(&mut reader, PM_TILES_BYTES.len() as u64, byte_length as u64);

        assert!(res.is_err());

        Ok(())
    }

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
