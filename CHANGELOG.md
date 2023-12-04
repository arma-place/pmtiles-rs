# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[unreleased]: https://github.com/arma-place/pmtiles-rs/compare/v0.2.3...HEAD
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
