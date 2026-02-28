use serde::{Serialize, Deserialize};
use crate::metre::indispensability::{gnsm_to_indispensability_list};
use crate::metre::rqq::parse_rqq;
use vizia_plug::vizia::prelude::Data;

// *must* derive Serialize and Deserialize for persistence
/// Holds all the important information for an RQQ defined metric structure.
#[derive(Debug, Serialize, Deserialize, Clone, Data)]
pub struct MetreData {
    pub string: String,
    pub value: Vec<usize>,
    pub gnsm: Vec<usize>,
    pub durations: Vec<f32>,
    pub max: usize,
}

impl Default for MetreData {
    fn default() -> Self {
        Self::try_from("(4 (1 1 1 1))")
            .expect("Hardcoded RQQ string for Default should never fail")
    }
}

/// Parse a &str that defines a metric structure using RQQ to MetreData.
impl TryFrom<&str> for MetreData {
    type Error = String;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let rqq = parse_rqq(text)?;
        let durations = rqq.to_durations(1.0)?;
        let sum: f32 = durations.iter().sum();
        let durations = durations.iter().map(|x| x / sum).collect::<Vec<f32>>();
        let gnsm = rqq.to_gnsm()?;
        let value = gnsm_to_indispensability_list(&gnsm)?;

        Ok(
            MetreData {
                string: text.to_string(),
                durations,
                max: *value.iter().max().unwrap_or(&1),
                value,
                gnsm,
            }
        )
    }
}