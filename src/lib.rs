//! # `PMTiles`
//!
//! A low level implementation of [the `PMTiles` format](https://github.com/protomaps/PMTiles) based on the [`std::io::Read`] and [`std::io::Write`] traits.
//!
//! ## Examples
//!
//! ### Reading from a file
//! ```rust
//! use std::fs::File;
//! use pmtiles2::PMTiles;
//!
//! fn main () -> std::io::Result<()> {
//!     let file_path = "./test/stamen_toner(raster)CC-BY+ODbL_z3.pmtiles";
//!     
//!     let mut file = File::open(file_path)?; // file implements std::io::Read
//!     let pm_tiles = PMTiles::from_reader(file)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Writing to a file
//! ```rust
//! use std::fs::File;
//! use pmtiles2::{PMTiles, Compression, TileType};
//!
//! fn main () -> std::io::Result<()> {
//!     // create temp directory
//!     let dir = tempdir::TempDir::new("pmtiles")?;
//!     let file_path = dir.path().join("foo.pmtiles");
//!     
//!     let pm_tiles = PMTiles::new(TileType::Png, Compression::None);
//!
//!     // TODO: Add tiles to pm_tiles
//!
//!     let mut file = File::create(file_path)?; // file implements std::io::Write
//!     pm_tiles.to_writer(&mut file)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Building a `PMTiles` archive from scratch
//! ```rust
//! use pmtiles2::{PMTiles, TileType, Compression, util::tile_id};
//! use std::io::Cursor;
//!
//! let mut pm_tiles = PMTiles::new(TileType::Mvt, Compression::GZip);
//!
//! pm_tiles.add_tile(tile_id(0, 0, 0), vec![0 /* ... */]);
//! pm_tiles.add_tile(tile_id(1, 0, 0), vec![0 /* ... */]);
//! ```

#![warn(missing_docs)]
#![warn(clippy::cargo)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![allow(clippy::must_use_candidate)]

mod directory;
mod header;
mod pmtiles;
mod tile_manager;

/// Utilities for reading and writing `PMTiles` archives.
pub mod util;

pub use self::pmtiles::PMTiles;
pub use directory::{Directory, Entry};
pub use header::{Compression, Header, TileType};
