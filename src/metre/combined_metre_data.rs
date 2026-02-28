use serde::{Deserialize, Serialize};
use vizia_plug::vizia::prelude::Data;
use crate::metre::interpolation::interpolation_data::InterpolationData;
use crate::metre::metre_data::MetreData;

/// Holds metric data for A and B and information used for interpolating between the two.
#[derive(Debug, Serialize, Deserialize, Clone, Data)]
pub struct CombinedMetreData {
    metre_a: MetreData,
    metre_b: MetreData,
    interpolation_data: InterpolationData,
}

impl Default for CombinedMetreData {
    fn default() -> Self {
       let mut result =
           Self {
               metre_a: MetreData::default(),
               metre_b: MetreData::default(),
               interpolation_data: InterpolationData::default(),
           };

        result.update_interpolation_data();

        result
    }
}

impl CombinedMetreData {

    pub fn new(metre_a: MetreData, metre_b: MetreData) -> Self {
        let mut result =
            Self {
                metre_a,
                metre_b,
                interpolation_data: InterpolationData::default(),
            };

        result.update_interpolation_data();

        result
    }

    pub fn metre_a(&self) -> &MetreData {
        &self.metre_a
    }

    pub fn metre_b(&self) -> &MetreData {
        &self.metre_b
    }

    pub fn set_metre_a(&mut self, metre_a: MetreData) {
        self.metre_a = metre_a;
        self.update_interpolation_data();
    }

    pub fn set_metre_b(&mut self, metre_b: MetreData) {
        self.metre_b = metre_b;
        self.update_interpolation_data();
    }

    pub fn interpolation_data(&self) -> &InterpolationData {
        &self.interpolation_data
    }

    pub fn get_interpolated_durations(&self, interpolation: f32) -> impl Iterator<Item = f32> + '_ {
        self.interpolation_data.get_interpolated_durations(interpolation)
    }

    pub fn get_interleaved_durations(&self, interpolation: f32) -> impl Iterator<Item = f32> + '_ {
        if interpolation <= 0.0 {
            self.metre_a.durations.iter().copied()
        } else if interpolation >= 1.0 {
            self.metre_b.durations.iter().copied()
        } else {
            self.interpolation_data.interleaved_durations().iter().copied()
        }
    }

    fn update_interpolation_data(&mut self) {
        self.interpolation_data =
            InterpolationData::new_from_durs_and_gnsm(&self.metre_a.durations, &self.metre_b.durations, &self.metre_a.gnsm, &self.metre_b.gnsm);
    }
}