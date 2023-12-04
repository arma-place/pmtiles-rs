use deku::prelude::*;

/// A tile type, which is supported in `PMTiles` archives.
#[derive(DekuRead, DekuWrite, Debug, Clone, Copy, PartialEq, Eq)]
#[deku(type = "u8")]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum TileType {
    #[allow(missing_docs)]
    Unknown = 0x00,

    /// Mapbox Vector Tiles as defined [here](https://github.com/mapbox/vector-tile-spec)
    Mvt,

    #[allow(missing_docs)]
    Png,

    #[allow(missing_docs)]
    Jpeg,

    #[allow(missing_docs)]
    WebP,
}

impl TileType {
    /// Returns a option containing the value to which the
    /// `Content-Type` HTTP header should be set, when serving
    /// tiles from this type.
    ///
    /// Returns [`None`] if a concrete `Content-Type` could not be determined.
    pub const fn http_content_type(&self) -> Option<&'static str> {
        match self {
            Self::Mvt => Some("application/vnd.mapbox-vector-tile"),
            Self::Png => Some("image/png"),
            Self::Jpeg => Some("image/jpeg"),
            Self::WebP => Some("image/webp"),
            Self::Unknown => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use deku::bitvec::{bitvec, BitSlice, BitVec, Lsb0};

    #[test]
    fn test_http_content_type() {
        assert_eq!(TileType::Unknown.http_content_type(), None);

        assert_eq!(
            TileType::Mvt.http_content_type(),
            Some("application/vnd.mapbox-vector-tile")
        );

        assert_eq!(TileType::Png.http_content_type(), Some("image/png"));

        assert_eq!(TileType::Jpeg.http_content_type(), Some("image/jpeg"));

        assert_eq!(TileType::WebP.http_content_type(), Some("image/webp"));
    }

    #[test]
    fn test_deku_read() -> Result<(), DekuError> {
        let slice = BitSlice::from_slice(&[0]);
        let (rest, tt0) = TileType::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(tt0, TileType::Unknown);
        assert_eq!(rest.len(), 0);

        let slice = BitSlice::from_slice(&[1]);
        let (_, tt1) = TileType::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(tt1, TileType::Mvt);

        let slice = BitSlice::from_slice(&[2]);
        let (_, tt2) = TileType::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(tt2, TileType::Png);

        let slice = BitSlice::from_slice(&[3]);
        let (_, tt3) = TileType::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(tt3, TileType::Jpeg);

        let slice = BitSlice::from_slice(&[4]);
        let (_, tt4) = TileType::read(slice, deku::ctx::Endian::Little)?;
        assert_eq!(tt4, TileType::WebP);

        Ok(())
    }

    #[test]
    fn test_deku_write() -> Result<(), DekuError> {
        let mut output = BitVec::new();
        TileType::Unknown.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 0, 0, 0));

        let mut output = BitVec::new();
        TileType::Mvt.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 0, 0, 1));

        let mut output = BitVec::new();
        TileType::Png.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 0, 1, 0));

        let mut output = BitVec::new();
        TileType::Jpeg.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 0, 1, 1));

        let mut output = BitVec::new();
        TileType::WebP.write(&mut output, deku::ctx::Endian::Little)?;
        assert_eq!(output, bitvec!(0, 0, 0, 0, 0, 1, 0, 0));

        Ok(())
    }
}
