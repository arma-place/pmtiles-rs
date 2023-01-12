use crate::Compression;

use brotli::{CompressorWriter as BrotliEncoder, Decompressor as BrotliDecoder};
use flate2::{read::GzDecoder, write::GzEncoder};
use zstd::{Decoder as ZSTDDecoder, Encoder as ZSTDEncoder};

use std::io::{Cursor, Error, ErrorKind, Read, Result, Write};

/// Returns a new instance of [`std::io::Write`] that will emit compressed data to the underlying writer.
///
/// # Arguments
/// * `compression` - Compression to use
/// * `writer` - Underlying writer to write compressed data to
///
/// # Errors
/// Will return [`Err`] if `compression` is set to [`Compression::Unknown`] or an error occurred
/// while creating the zstd encoder.
///
/// # Example
/// ```rust
/// # use pmtiles2::{util::compress, Compression};
/// let mut output = Vec::<u8>::new();
///
/// let mut writer = compress(Compression::GZip, &mut output).unwrap();
///
/// let data_to_compress: Vec<u8> = vec![1, 3, 3, 7, 0, 4, 2, 0, 6, 9];
/// writer.write_all(&data_to_compress).unwrap();
///
/// writer.flush().unwrap(); // do not forget to flush writer to make sure it is done writing
/// ```
pub fn compress<'a>(
    compression: Compression,
    writer: &'a mut impl Write,
) -> Result<Box<dyn Write + 'a>> {
    match compression {
        Compression::Unknown => Err(Error::new(
            ErrorKind::Other,
            "Cannot compress for Compression Unknown",
        )),
        Compression::None => Ok(Box::new(writer)),
        Compression::GZip => Ok(Box::new(GzEncoder::new(
            writer,
            flate2::Compression::default(),
        ))),
        Compression::Brotli => Ok(Box::new(BrotliEncoder::new(writer, 4096, 11, 24))),
        Compression::ZStd => Ok(Box::new(ZSTDEncoder::new(writer, 0)?.auto_finish())),
    }
}

/// Compresses a byte slice and returns the result as a new [`Vec<u8>`].
///
/// # Arguments
/// * `compression` - Compression to use
/// * `data` - Data to compress
///
/// # Errors
/// Will return [`Err`] if `compression` is set to [`Compression::Unknown`], there was an error
/// while creating the zstd encoder or an error occurred while writing to `data`.
#[allow(clippy::module_name_repetitions)]
pub fn compress_all(compression: Compression, data: &[u8]) -> Result<Vec<u8>> {
    let mut destination = Vec::<u8>::new();

    {
        let mut writer = compress(compression, &mut destination)?;

        writer.write_all(data)?;

        writer.flush()?;
    }

    Ok(destination)
}

/// Returns a new instance of [`std::io::Read`] that will emit uncompressed data from an the underlying reader.
///
/// # Arguments
/// * `compression` - Compression to use
/// * `compressed_data` - Underlying reader with compressed data
///
/// # Errors
/// Will return [`Err`] if `compression` is set to [`Compression::Unknown`],there was an
/// error while creating the zstd decoder.
///
/// # Example
/// ```rust
/// # use pmtiles2::{util::decompress, Compression};
/// # let data = include_bytes!("../../test/compress/data.json.gz");
/// let mut data_reader = std::io::Cursor::new(data);
///
/// let mut reader = decompress(Compression::GZip, &mut data_reader).unwrap();
///
/// let mut destination = Vec::<u8>::new();
///
/// reader.read_to_end(&mut destination).unwrap();
/// ```
pub fn decompress<'a>(
    compression: Compression,
    compressed_data: &'a mut impl Read,
) -> Result<Box<dyn Read + 'a>> {
    match compression {
        Compression::Unknown => Err(Error::new(
            ErrorKind::Other,
            "Cannot decompress for Compression Unknown",
        )),
        Compression::None => Ok(Box::new(compressed_data)),
        Compression::GZip => Ok(Box::new(GzDecoder::new(compressed_data))),
        Compression::Brotli => Ok(Box::new(BrotliDecoder::new(compressed_data, 4096))),
        Compression::ZStd => Ok(Box::new(ZSTDDecoder::new(compressed_data)?)),
    }
}

/// Decompresses a byte slice and returns the result as a new [`Vec<u8>`].
///
/// # Arguments
/// * `compression` - Compression to use
/// * `data` - Data to decompress
///
/// # Errors
/// Will return [`Err`] if...
/// - `compression` is set to [`Compression::Unknown`]
/// - there was an error while creating the zstd decoder
/// - there was an error reading the `data`
/// - `data` is not compressed correctly
///
pub fn decompress_all(compression: Compression, data: &[u8]) -> Result<Vec<u8>> {
    let mut data_reader = Cursor::new(data);

    let mut reader = decompress(compression, &mut data_reader)?;

    let mut destination = Vec::<u8>::new();

    reader.read_to_end(&mut destination)?;

    Ok(destination)
}

#[cfg(test)]
mod test {
    use super::*;

    const DATA_UNCOMPRESSED: &[u8] = include_bytes!("../../test/compress/data.json");
    const DATA_GZIP: &[u8] = include_bytes!("../../test/compress/data.json.gz");
    const DATA_BR: &[u8] = include_bytes!("../../test/compress/data.json.br");
    const DATA_ZST: &[u8] = include_bytes!("../../test/compress/data.json.zst");

    #[test]
    fn decompress_all_unknown() {
        let res = decompress_all(Compression::Unknown, &Vec::new());
        assert!(res.is_err());
    }

    #[test]
    fn decompress_all_none() -> Result<()> {
        let data = decompress_all(Compression::None, DATA_UNCOMPRESSED)?;
        assert_eq!(data, DATA_UNCOMPRESSED);
        Ok(())
    }

    #[test]
    fn decompress_all_gzip() -> Result<()> {
        let data = decompress_all(Compression::GZip, DATA_GZIP)?;
        assert_eq!(data, DATA_UNCOMPRESSED);
        Ok(())
    }

    #[test]
    fn decompress_all_brotli() -> Result<()> {
        let data = decompress_all(Compression::Brotli, DATA_BR)?;
        assert_eq!(data, DATA_UNCOMPRESSED);
        Ok(())
    }

    #[test]
    fn decompress_all_zstd() -> Result<()> {
        let data = decompress_all(Compression::ZStd, DATA_ZST)?;
        assert_eq!(data, DATA_UNCOMPRESSED);
        Ok(())
    }

    #[test]
    fn compress_all_unknown() {
        let res = compress_all(Compression::Unknown, &Vec::new());
        assert!(res.is_err());
    }

    #[test]
    fn compress_all_none() -> Result<()> {
        let data = compress_all(Compression::None, DATA_UNCOMPRESSED)?;
        assert_eq!(data, DATA_UNCOMPRESSED);
        Ok(())
    }

    #[test]
    fn compress_all_gzip() -> Result<()> {
        let data = compress_all(Compression::GZip, DATA_UNCOMPRESSED)?;
        assert_eq!(data, DATA_GZIP);
        Ok(())
    }

    #[test]
    fn compress_all_brotli() -> Result<()> {
        let data = compress_all(Compression::Brotli, DATA_UNCOMPRESSED)?;
        assert_eq!(data, DATA_BR);
        Ok(())
    }

    #[test]
    fn compress_all_zstd() -> Result<()> {
        let data = compress_all(Compression::ZStd, DATA_UNCOMPRESSED)?;
        assert_eq!(data, DATA_ZST);
        Ok(())
    }
}
