use serde::{Serialize, Deserialize};
use crate::metre::indispensability::rqq_to_indispensability_list;
use crate::metre::rqq::parse_rqq;

// *must* derive Serialize and Deserialize for persistence
#[derive(Debug, Serialize, Deserialize, Clone)] // Added Clone for easier use in Vizia
pub struct MetreData {
    pub input: String,
    pub value: Vec<usize>,
    pub durations: Vec<f32>,
    pub max: usize,
}

impl Default for MetreData {
    fn default() -> Self {
        Self {
            input: String::from("(4 (1 1 1 1))"),
            value: vec![0, 3, 2, 1],
            max: 3,
            durations: vec![0.25; 4], // TODO calc!
        }
    }
}

pub fn parse_input(text: &str) -> Result<MetreData, String> {
    let rqq = parse_rqq(text)?;
    let durations = rqq.to_durations(1.0);
    let sum: f32 = durations.iter().sum();
    let durations = durations.iter().map(|x| x / sum).collect::<Vec<f32>>();
    let value = rqq_to_indispensability_list(rqq)?;
    dbg!(&durations);
    Ok(
        MetreData {
            input: String::from(text),
            durations,
            max: *value.iter().max().unwrap_or(&1),
            value,
        }
    )
}