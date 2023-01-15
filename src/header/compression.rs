use deku::prelude::*;

/// A compression, which is supported in `PMTiles` archives.
#[derive(DekuRead, DekuWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[deku(type = "u8")]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Compression {
    /// Unknown compression
    ///
    /// _This should almost never be used, because some reader
    /// implementations may not know how to handle this._
    Unknown = 0x00,

    /// No compression
    None,

    /// GZIP compression as defined in [RFC 1952](https://www.rfc-editor.org/rfc/rfc1952)
    GZip,

    /// Brotli compression as defined in [RFC 7932](https://www.rfc-editor.org/rfc/rfc7932)
    Brotli,

    /// Zstandard Compression as defined in [RFC 8478](https://www.rfc-editor.org/rfc/rfc8478)
    ZStd,
}

impl Compression {
    /// Returns a option containing the value to which the
    /// `Content-Encoding` HTTP header should be set, when serving
    /// tiles with this compression.
    ///
    /// Returns [`None`] if a concrete `Content-Encoding` could not be determined.
    pub const fn http_content_encoding(&self) -> Option<&'static str> {
        match self {
            Self::GZip => Some("gzip"),
            Self::Brotli => Some("br"),
            Self::ZStd => Some("zstd"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use deku::bitvec::{bitvec, BitSlice, BitVec, Lsb0};

    #[test]
    fn test_http_content_encoding() {
        assert_eq!(Compression::Unknown.http_content_encoding(), None);

        assert_eq!(Compression::None.http_content_encoding(), None);

        assert_eq!(Compression::GZip.http_content_encoding(), Some("gzip"));

        assert_eq!(Compression::Brotli.http_content_encoding(), Some("br"));

        assert_eq!(Compression::ZStd.http_content_encoding(), Some("zstd"));
    }

    #[test]
    fn test_deku_read() -> Result<(), DekuError> {
        let slice = BitSlice::from_slice(&[0]);
        let (rest, val) = Compression::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(val, Compression::Unknown);
        assert_eq!(rest.len(), 0);

        let slice = BitSlice::from_slice(&[1]);
        let (_, val) = Compression::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(val, Compression::None);

        let slice = BitSlice::from_slice(&[2]);
        let (_, val) = Compression::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(val, Compression::GZip);

        let slice = BitSlice::from_slice(&[3]);
        let (_, val) = Compression::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(val, Compression::Brotli);

        let slice = BitSlice::from_slice(&[4]);
        let (_, val) = Compression::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(val, Compression::ZStd);

        Ok(())
    }

    #[test]
    fn test_deku_write() -> Result<(), DekuError> {
        let mut output = BitVec::new();
        Compression::Unknown.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 0, 0, 0));

        let mut output = BitVec::new();
        Compression::None.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 0, 0, 1));

        let mut output = BitVec::new();
        Compression::GZip.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 0, 1, 0));

        let mut output = BitVec::new();
        Compression::Brotli.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 0, 1, 1));

        let mut output = BitVec::new();
        Compression::ZStd.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 1, 0, 0));

        Ok(())
    }
}
