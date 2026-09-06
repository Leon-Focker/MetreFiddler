use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetreSlot {
    MetreA,
    MetreB,
    Both,
}

impl MetreSlot {
    pub fn calculate_opacity(self, interpolate: f32) -> u8 {
        let id = match self {
            MetreSlot::MetreA => -1.0,
            MetreSlot::MetreB => 0.0,
            MetreSlot::Both => 1.0,
        };
        ((id + interpolate).abs().min(1.0) * 255.0).round() as u8
    }
}

impl std::ops::Not for MetreSlot {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Self::MetreA => Self::MetreB,
            Self::MetreB => Self::MetreA,
            Self::Both => Self::Both,
        }
    }
}