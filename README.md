# PMTiles (for Rust) [![crates.io](https://img.shields.io/crates/v/pmtiles2?style=flat-square&logo=rust)](https://crates.io/crates/pmtiles2) [![docs.rs](https://img.shields.io/badge/docs.rs-pmtiles2-66c2a5.svg?logo=docs.rs&style=flat-square)](https://docs.rs/pmtiles2) [![build status](https://img.shields.io/github/actions/workflow/status/arma-place/pmtiles-rs/CI.yml?branch=master&style=flat-square)](https://github.com/arma-place/pmtiles-rs/actions?query=branch%3Amaster)

This crate includes a low level implementation of [the PMTiles format](https://github.com/protomaps/PMTiles) based on the standard [Read](https://doc.rust-lang.org/std/io/trait.Read.html) and [Write](https://doc.rust-lang.org/std/io/trait.Write.html) trait.

It also contains [some utilities](https://docs.rs/pmtiles2/util/), which might become handy when working with PMTiles archives. Among others these include function for (de-)compression with all algorithms supported by PMTiles, as well as function to convert from and to tile ids.

## Documentation
See [RustDoc Documentation](https://docs.rs/pmtiles2).

The documentation includes some examples.

## Installation

Add following lines to your Cargo.toml:
```toml
# Cargo.toml
[dependencies]
pmtiles2 = "0.1.0"
```
