# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Async support via the [`AsyncRead`](https://docs.rs/futures/latest/futures/io/trait.AsyncRead.html) and [`AsyncWrite`](https://docs.rs/futures/latest/futures/io/trait.AsyncWrite.html) traits (must be enabled via the `asnyc` feature flag):
  - Added `from_async_reader` & `to_async_writer` methods to `PMTiles`
  - Added `from_async_reader` & `to_async_writer` methods to `Directory`
  - Added `from_async_reader` & `to_async_writer` methods to `Header`
  - Added `get_tile_async`, `get_tile_by_id_async` & `new_async` methods to `Header`
  - Added `compress_async` & `decompress_async` utility functions
  - Added `read_directories_async` & `write_directories_async` utility functions

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

[unreleased]: https://github.com/arma-place/pmtiles-rs/compare/0.1.6...HEAD
[0.1.6]: https://github.com/arma-place/pmtiles-rs/compare/0.1.5...0.1.6
[0.1.5]: https://github.com/arma-place/pmtiles-rs/compare/0.1.4...0.1.5
[0.1.4]: https://github.com/arma-place/pmtiles-rs/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/arma-place/pmtiles-rs/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/arma-place/pmtiles-rs/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/arma-place/pmtiles-rs/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/arma-place/pmtiles-rs/releases/tag/0.1.0