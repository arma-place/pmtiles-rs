use std::{error::Error, fmt};

use hilbert_2d::Variant;

const MAX_Z: u8 = 32;

/// An error indicating that the specified tile id has a
/// z value greater than the maximum allowed z value.
#[derive(Debug, Copy, Clone)]
pub struct MaxZError;

impl fmt::Display for MaxZError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "The maximum allowed value for Z is {MAX_Z}")
    }
}

impl Error for MaxZError {}

/// Converts z/x/y coordinates to a tile id.
///
/// # Arguments
///
/// * `z` - The z coordinate (lod)
/// * `x` - The x coordinate
/// * `y` - The y coordinate
pub fn tile_id(z: u8, x: u64, y: u64) -> u64 {
    if z == 0 {
        return 0;
    }

    let base_id: u64 = 1 + (1..z).map(|i| 4u64.pow(u32::from(i))).sum::<u64>();

    #[allow(clippy::cast_possible_truncation)]
    let tile_id =
        hilbert_2d::xy2h_discrete(x as usize, y as usize, z as usize, Variant::Hilbert) as u64;

    base_id + tile_id
}

fn find_z(tile_id: u64) -> Result<u8, MaxZError> {
    let mut z = 0u8;
    let mut acc = 1u64;

    for i in 1u8..MAX_Z {
        let num_tiles = 4u64.pow(u32::from(i));
        acc += num_tiles;

        if acc > tile_id {
            z = i;
            break;
        }
    }

    if z == 0 {
        return Err(MaxZError {});
    }

    Ok(z)
}

/// Converts a tile id to z/x/y coordinates.
///
/// # Arguments
/// * `tile_id` - The tile id
///
/// # Errors
/// Will return [`Err`] if `tile_id` has a too large z coordinate.
pub fn zxy(tile_id: u64) -> Result<(u8, u64, u64), MaxZError> {
    if tile_id == 0 {
        return Ok((0, 0, 0));
    }

    let z = find_z(tile_id)?;

    let base_id: u64 = 1 + (1..z).map(|i| 4u64.pow(u32::from(i))).sum::<u64>();

    #[allow(clippy::cast_possible_truncation)]
    let (x, y) =
        hilbert_2d::h2xy_discrete((tile_id - base_id) as usize, z as usize, Variant::Hilbert);

    Ok((z, x as u64, y as u64))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tile_id() {
        assert_eq!(tile_id(0, 0, 0), 0);
        assert_eq!(tile_id(1, 0, 0), 1);
        assert_eq!(tile_id(1, 0, 1), 2);
        assert_eq!(tile_id(1, 1, 1), 3);
        assert_eq!(tile_id(1, 1, 0), 4);
        assert_eq!(tile_id(2, 0, 0), 5);
    }

    #[test]
    fn test_xyz() -> Result<(), MaxZError> {
        assert_eq!(zxy(0)?, (0, 0, 0));
        assert_eq!(zxy(1)?, (1, 0, 0));
        assert_eq!(zxy(2)?, (1, 0, 1));
        assert_eq!(zxy(3)?, (1, 1, 1));
        assert_eq!(zxy(4)?, (1, 1, 0));
        assert_eq!(zxy(5)?, (2, 0, 0));
        assert_eq!(zxy(19_078_479)?, (12, 3423, 1763));

        Ok(())
    }

    #[test]
    fn test_extremes() -> Result<(), MaxZError> {
        for z in 0u8..MAX_Z {
            let dim: u64 = (1 << z) - 1;
            assert_eq!(zxy(tile_id(z, 0, 0))?, (z, 0, 0));
            assert_eq!(zxy(tile_id(z, dim, 0))?, (z, dim, 0));
            assert_eq!(zxy(tile_id(z, 0, dim))?, (z, 0, dim));
            assert_eq!(zxy(tile_id(z, dim, dim))?, (z, dim, dim));
        }

        Ok(())
    }
}
