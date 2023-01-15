use deku::{
    bitvec::{BitSlice, BitVec, Msb0},
    prelude::*,
};

#[derive(DekuRead, DekuWrite, Debug, PartialEq)]
#[deku(endian = "endian", ctx = "_endian: deku::ctx::Endian")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LatLng {
    #[deku(
        reader = "Self::read_lat_lon(deku::rest)",
        writer = "Self::write_lat_lon(deku::output, self.longitude)"
    )]
    pub longitude: f64,

    #[deku(
        reader = "Self::read_lat_lon(deku::rest)",
        writer = "Self::write_lat_lon(deku::output, self.latitude)"
    )]
    pub latitude: f64,
}

const LAT_LONG_FACTOR: f64 = 10_000_000.0;

impl LatLng {
    fn read_lat_lon(rest: &BitSlice<u8, Msb0>) -> Result<(&BitSlice<u8, Msb0>, f64), DekuError> {
        let (rest, value) = i32::read(rest, ())?;
        Ok((rest, f64::from(value) / LAT_LONG_FACTOR))
    }

    #[allow(clippy::cast_possible_truncation)]
    fn write_lat_lon(output: &mut BitVec<u8, Msb0>, field: f64) -> Result<(), DekuError> {
        let value = (field * LAT_LONG_FACTOR) as i32;
        value.write(output, ())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use deku::bitvec::BitView;

    #[test]
    fn test_read_lat_lon() -> Result<(), DekuError> {
        let (_, val) = LatLng::read_lat_lon(BitSlice::from_slice(&[0x00, 0x2E, 0xB6, 0x94]))?;
        assert!((-180.0 - val).abs() < f64::EPSILON);

        let (_, val) = LatLng::read_lat_lon(BitSlice::from_slice(&[0x00, 0xD2, 0x49, 0x6B]))?;
        assert!((180.0 - val).abs() < f64::EPSILON);

        let (_, val) = LatLng::read_lat_lon(BitSlice::from_slice(&[0x00, 0x00, 0x0, 0x00]))?;
        assert!((0.0 - val).abs() < f64::EPSILON);

        Ok(())
    }

    #[test]
    fn test_write_lat_lon() -> Result<(), DekuError> {
        let mut output = BitVec::with_capacity(32);
        LatLng::write_lat_lon(&mut output, -180.0)?;
        assert_eq!(output, [0x00u8, 0x2E, 0xB6, 0x94].view_bits::<Msb0>());

        let mut output = BitVec::with_capacity(32);
        LatLng::write_lat_lon(&mut output, 180.0)?;
        assert_eq!(output, [0x00u8, 0xD2, 0x49, 0x6B].view_bits::<Msb0>());

        let mut output = BitVec::with_capacity(32);
        LatLng::write_lat_lon(&mut output, 0.0)?;
        assert_eq!(output, [0x00u8, 0x00, 0x0, 0x00].view_bits::<Msb0>());

        Ok(())
    }

    #[test]
    fn test_deku_read() -> Result<(), DekuError> {
        let slice = BitSlice::from_slice(&[0x00, 0x2E, 0xB6, 0x94, 0x80, 0x07, 0x56, 0xCD]);
        let (rest, ll) = LatLng::read(slice, deku::ctx::Endian::Little)?;

        assert_eq!(rest.len(), 0);
        assert!((-180.0 - ll.longitude).abs() < f64::EPSILON);
        assert!((-85.0 - ll.latitude).abs() < f64::EPSILON);

        Ok(())
    }

    #[test]
    fn test_deku_write() -> Result<(), DekuError> {
        let mut output = BitVec::with_capacity(64);
        LatLng {
            longitude: -180.0,
            latitude: -85.0,
        }
        .write(&mut output, deku::ctx::Endian::Little)?;

        assert_eq!(
            output,
            [0x00u8, 0x2E, 0xB6, 0x94, 0x80, 0x07, 0x56, 0xCD].view_bits::<Msb0>()
        );

        Ok(())
    }
}
