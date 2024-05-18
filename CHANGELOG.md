# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0]

### Breaking Changes
This release includes two breaking changes to align this implementation to the latest PMTiles specification (version 3.4).

#### PMTiles Metadata must be an JSON object
Version 3.2 of the PMTiles specification clarified that the JSON metadata must be a JSON object.

This brings along two breaking changes:
- `PMTiles::meta_data` is now of type `serde_json::Map<String, JSONValue>` (was `Option<serde_json::Value>` before)
- When encountering meta data that is not a JSON object while reading a PMTiles archive, an error is returned. This affects the following functions:
  - `PMTiles::from_reader`/ `PMTiles::from_async_reader`
  - `PMTiles::from_reader_partially` / `PMTiles::from_async_reader_partially`
  - `PMTiles::from_bytes`

#### Directory entries must have a length greater than 0
Version 3.4 of the PMTiles specification clarified that directory entries must have a length that is greater than `0`. This was implemented with the following breaking changes:

- `PMTiles::add_tile` now returns and `Result` and will error if it is being called with empty data
- If an entry with a length of `0` is encountered while reading an directory, an error is returned. This affects the following functions:
  - `PMTiles::from_reader`/ `PMTiles::from_async_reader`
  - `PMTiles::from_reader_partially` / `PMTiles::from_async_reader_partially`
  - `PMTiles::from_bytes`
  - `PMTiles::from_bytes_partially`
  - `util::read_directories`/ `util::read_directories_async`
  - `Directory::from_reader`/ `Directory::from_async_reader`
  - `Directory::from_bytes`
- Calling `Directory::to_writer` / `Directory::to_async_writer` on a `Directory` including an entry with a length of `0` will result in an error

### Fixed
- Writing async PMTiles archives is corrupt ([#10](https://github.com/arma-place/pmtiles-rs/issues/10))

### Added
- Added AVIF tile type (as per PMTiles specification version 3.1)
- Added recommended MIME type constant [`MIME_TYPE`](https://docs.rs/pmtiles2/0.3.0/pmtiles2/constant.MIME_TYPE.html) (as per PMTiles specification version 3.3)

### Changed
- Implement [`IntoIterator`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html) for `Directory`-struct
- Deprecate `Directory::iter` method in favor of `IntoIterator` trait implementation

Deprecate 

## [0.2.3]
- Add `from_bytes` associated function to `PMTiles`-, `Header`- and `Directory`-struct
- Add `from_bytes_partially` associated function to `PMTiles`-struct
- Add `find_entry_for_tile_id` method to `Directory`-struct
- Fix the MIME type for Mapbox Vector Tiles (previously `application/x-protobuf`, now `application/vnd.mapbox-vector-tile`)

## [0.2.2] - 2023-10-23
- Tweaks to documentation

## [0.2.1] - 2023-10-23
- Tweaks to documentation
- `clippy` & `cargofmt` fixes

## [0.2.0] - 2023-05-08

### Breaking Changes
- Added `filter_range` parameter to `read_directories` & `read_directories_async` (use `..` to have same behavior as before)

### Added

- Async support via the [`AsyncRead`](https://docs.rs/futures/latest/futures/io/trait.AsyncRead.html) and [`AsyncWrite`](https://docs.rs/futures/latest/futures/io/trait.AsyncWrite.html) traits (must be enabled via the `asnyc` feature flag):
  - Added `from_async_reader` & `to_async_writer` methods to `PMTiles`
  - Added `from_async_reader` & `to_async_writer` methods to `Directory`
  - Added `from_async_reader` & `to_async_writer` methods to `Header`
  - Added `get_tile_async`, `get_tile_by_id_async` & `new_async` methods to `Header`
  - Added `compress_async` & `decompress_async` utility functions
  - Added `read_directories_async` & `write_directories_async` utility functions
- Added `from_reader_partially` & `from_async_reader_partially` methods to `PMTiles`

### Changed
- Improved example of `util::read_directories`

## [0.1.6] - 2023-01-18

### Added 
- Added `serde` support for most public types (must be enabled via the `serde` feature flag)


## [0.1.5] - 2023-01-15

### Added 
- Added `from_reader` method to `Header`
- Added `to_writer` method to `Header`

## [0.1.4] - 2023-01-14

### Changed 
- Update `zstd` feature flags to allow `wasm-unknown-unknown` as a build target

## [0.1.3] - 2023-01-14

### Changed 
- Remove `getrandom` dependency to allow `wasm-unknown-unknown` as a build target

## [0.1.2] - 2023-01-13

### Fix 
- Fix broken link to utilities documentation in README

## [0.1.1] - 2023-01-12

### Changed 
- Improved wording of the documentation in several places

## [0.1.0] - 2023-01-12

Initial public release

[unreleased]: https://github.com/arma-place/pmtiles-rs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/arma-place/pmtiles-rs/compare/v0.2.3...v0.3.0
[0.2.3]: https://github.com/arma-place/pmtiles-rs/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/arma-place/pmtiles-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/arma-place/pmtiles-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/arma-place/pmtiles-rs/compare/v0.1.6...v0.2.0
[0.1.6]: https://github.com/arma-place/pmtiles-rs/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/arma-place/pmtiles-rs/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/arma-place/pmtiles-rs/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/arma-place/pmtiles-rs/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/arma-place/pmtiles-rs/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/arma-place/pmtiles-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/arma-place/pmtiles-rs/releases/tag/v0.1.0
