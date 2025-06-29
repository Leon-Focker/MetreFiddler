use serde::{Serialize, Deserialize};
use crate::metre::indispensability::rqq_to_indispensability_list;
use crate::metre::rqq::parse_rqq;

// *must* derive Serialize and Deserialize for persistence
/// Holds all the important information for an RQQ defined metric structure.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetreData {
    pub input: String,
    pub value: Vec<usize>,
    pub durations: Vec<f32>,
    pub max: usize,
}

impl Default for MetreData {
    fn default() -> Self {
        let string = String::from("(4 (1 1 1 1))");
        parse_input(&string).unwrap()
    }
}

/// Parse a &str that defines a metric structure using RQQ to MetreData.
pub fn parse_input(text: &str) -> Result<MetreData, String> {
    let rqq = parse_rqq(text)?;
    let durations = rqq.to_durations(1.0)?;
    let sum: f32 = durations.iter().sum();
    let durations = durations.iter().map(|x| x / sum).collect::<Vec<f32>>();
    let value = rqq_to_indispensability_list(rqq)?;
    
    Ok(
        MetreData {
            input: String::from(text),
            durations,
            max: *value.iter().max().unwrap_or(&1),
            value,
        }
    )
}