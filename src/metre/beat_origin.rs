use serde::{Deserialize, Serialize};
use vizia_plug::vizia::prelude::Data;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Data, PartialEq)]
pub enum BeatOrigin {
    MetreA,
    MetreB,
    Both,
}

impl BeatOrigin {
    pub fn to_opacity(self, interpolate: f32) -> u8 {
        let id = match self {
            BeatOrigin::MetreA => -1.0,
            BeatOrigin::MetreB => 0.0,
            BeatOrigin::Both => 1.0,
        };
        ((id + interpolate).abs().min(1.0) * 255.0).round() as u8
    }
}