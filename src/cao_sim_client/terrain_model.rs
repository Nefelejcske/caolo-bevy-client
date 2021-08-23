use std::convert::{TryFrom, TryInto};

use super::cao_sim_model::AxialPos;
use super::cao_sim_model::TerrainTy;

pub fn terrain_payload_to_components<'a>(
    input: &'a [i64],
    layout: &'a [AxialPos],
) -> impl Iterator<Item = (AxialPos, TerrainTy)> + 'a {
    assert_eq!(input.len(), layout.len());

    layout.iter().copied().zip(
        input
            .iter()
            .copied()
            .map(|i| i.try_into().expect("Unhandled terrain type")),
    )
}

impl TryFrom<i64> for TerrainTy {
    type Error = i64;

    fn try_from(val: i64) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(TerrainTy::Empty),
            1 => Ok(TerrainTy::Plain),
            2 => Ok(TerrainTy::Wall),
            3 => Ok(TerrainTy::Bridge),
            _ => Err(val),
        }
    }
}

