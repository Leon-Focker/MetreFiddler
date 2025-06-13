use serde::{Serialize, Deserialize};
use crate::metre::indispensability::rqq_to_indispensability_list;
use crate::metre::rqq::parse_rqq;

// *must* derive Serialize and Deserialize for persistence
#[derive(Debug, Serialize, Deserialize, Clone)] // Added Clone for easier use in Vizia
pub struct MetreData {
    pub input: String,
    pub value: Vec<usize>,
    pub max: usize,
}

impl Default for MetreData {
    fn default() -> Self {
        Self {
            input: String::from("(4 (1 1 1 1))"),
            value: vec![0, 3, 2, 1],
            max: 3,
        }
    }
}

pub fn parse_input(text: &str) -> Result<Vec<usize>, String> {
    rqq_to_indispensability_list(parse_rqq(text)?)
}

