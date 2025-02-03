// Crossover: clean crossovers as a multi-out plugin
// Copyright (C) 2022-2024 Robbert van der Helm
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use nih_plug::debug::*;
use std::f32::consts;
use std::fmt::Debug;
use std::ops::{Add, Mul, Sub, Div};

pub const NEUTRAL_Q: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// A simple biquad filter with functions for generating coefficients for standard filter
/// types
///
/// Based on <https://en.wikipedia.org/wiki/Digital_biquad_filter#Transposed_direct_forms>.
///
/// The type parameter T  should be either an `f32` or a SIMD type.
#[derive(Clone, Copy, Debug)]
pub struct Biquad<T> {
    pub coefficients: BiquadCoefficients<T>,
    s1: T,
    s2: T,
}

/// Similar to Biquad but with two sets of coefficients, between which can be interpolated
#[derive(Clone, Copy, Debug)]
pub struct DoubleBiquad<T> {
    pub coefficients: BiquadCoefficients<T>,
    pub coefficients1: BiquadCoefficients<T>,
    pub coefficients2: BiquadCoefficients<T>,
    s1: T,
    s2: T,
}

/// The coefficients `[b0, b1, b2, a1, a2]` for [`Biquad`]. These coefficients are all
/// prenormalized, i.e. they have been divided by `a0`.
///
/// The type parameter T  should be either an `f32` or a SIMD type.
#[derive(Clone, Copy, Debug)]
pub struct BiquadCoefficients<T> {
    b0: T,
    b1: T,
    b2: T,
    a1: T,
    a2: T,
}

/// Either an `f32` or some SIMD vector type of `f32`s that can be used with our biquads.
pub trait SimdType:
    Mul<Output = Self> + Sub<Output = Self> + Add<Output = Self> + Div<Output = Self> + Copy + Sized
{
    fn from_f32(value: f32) -> Self;
}

impl<T: SimdType> Default for Biquad<T> {
    /// Before setting constants the filter should just act as an identity function.
    fn default() -> Self {
        Self {
            coefficients: BiquadCoefficients::identity(),
            s1: T::from_f32(0.0),
            s2: T::from_f32(0.0),
        }
    }
}

impl<T: SimdType> Default for DoubleBiquad<T> {
    /// Before setting constants the filter should just act as an identity function.
    fn default() -> Self {
        Self {
            coefficients: BiquadCoefficients::identity(),
            coefficients1: BiquadCoefficients::identity(),
            coefficients2: BiquadCoefficients::identity(),
            s1: T::from_f32(0.0),
            s2: T::from_f32(0.0),
        }
    }
}

impl<T: SimdType> Biquad<T> {
    /// Process a single sample.
    pub fn process(&mut self, sample: T) -> T {
        let result = self.coefficients.b0 * sample + self.s1;

        self.s1 = self.coefficients.b1 * sample - self.coefficients.a1 * result + self.s2;
        self.s2 = self.coefficients.b2 * sample - self.coefficients.a2 * result;

        result
    }

    /// Reset the state to zero, useful after making large, non-interpolatable changes to the
    /// filter coefficients.
    pub fn reset(&mut self) {
        self.s1 = T::from_f32(0.0);
        self.s2 = T::from_f32(0.0);
    }
}

// process and reset is the exact same code as for Biquad, how can I avoid that?
impl<T: SimdType> DoubleBiquad<T> {
    /// Process a single sample.
    pub fn process(&mut self, sample: T) -> T {
        let result = self.coefficients.b0 * sample + self.s1;

        self.s1 = self.coefficients.b1 * sample - self.coefficients.a1 * result + self.s2;
        self.s2 = self.coefficients.b2 * sample - self.coefficients.a2 * result;

        result
    }

    /// Reset the state to zero, useful after making large, non-interpolatable changes to the
    /// filter coefficients.
    pub fn reset(&mut self) {
        self.s1 = T::from_f32(0.0);
        self.s2 = T::from_f32(0.0);
    }
    pub fn interpolate(&mut self, interpolation: f32) {
        self.coefficients.b0 = drywet(self.coefficients1.b0, self.coefficients2.b0, interpolation);
        self.coefficients.b1 = drywet(self.coefficients1.b1, self.coefficients2.b1, interpolation);
        self.coefficients.b2 = drywet(self.coefficients1.b2, self.coefficients2.b2, interpolation);
        self.coefficients.a1 = drywet(self.coefficients1.a1, self.coefficients2.a1, interpolation);
        self.coefficients.a2 = drywet(self.coefficients1.a2, self.coefficients2.a2, interpolation);
    }
}
impl<T: SimdType> BiquadCoefficients<T> {
    /// Convert scalar coefficients into the correct vector type.
    pub fn from_f32s(scalar: BiquadCoefficients<f32>) -> Self {
        Self {
            b0: T::from_f32(scalar.b0),
            b1: T::from_f32(scalar.b1),
            b2: T::from_f32(scalar.b2),
            a1: T::from_f32(scalar.a1),
            a2: T::from_f32(scalar.a2),
        }
    }

    /// Filter coefficients that would cause the sound to be passed through as is.
    pub fn identity() -> Self {
        Self::from_f32s(BiquadCoefficients {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        })
    }

    /// Compute the coefficients for a low-pass filter.
    ///
    /// Based on <http://shepazu.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html>.
    pub fn lowpass(&mut self, sample_rate: f32, frequency: f32, q: f32) {
        nih_debug_assert!(sample_rate > 0.0);
        nih_debug_assert!(frequency > 0.0);
        nih_debug_assert!(frequency < sample_rate / 2.0);
        nih_debug_assert!(q > 0.0);

        let omega0 = consts::TAU * (frequency / sample_rate);
        let cos_omega0 = omega0.cos();
        let alpha = omega0.sin() / (2.0 * q);

        // We'll prenormalize everything with a0
        let a0 = 1.0 + alpha;
        self.b0 = T::from_f32((1.0 - cos_omega0) / 2.0 / a0);
        self.b1 = T::from_f32((1.0 - cos_omega0) / a0);
        self.b2 = T::from_f32(((1.0 - cos_omega0) / 2.0) / a0);
        self.a1 = T::from_f32((-2.0 * cos_omega0) / a0);
        self.a2 = T::from_f32((1.0 - alpha) / a0);
    }

    /// Compute the coefficients for a high-pass filter.
    ///
    /// Based on <http://shepazu.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html>.
    pub fn highpass(&mut self, sample_rate: f32, frequency: f32, q: f32) {
        nih_debug_assert!(sample_rate > 0.0);
        nih_debug_assert!(frequency > 0.0);
        nih_debug_assert!(frequency < sample_rate / 2.0);
        nih_debug_assert!(q > 0.0);

        let omega0 = consts::TAU * (frequency / sample_rate);
        let cos_omega0 = omega0.cos();
        let alpha = omega0.sin() / (2.0 * q);

        // We'll prenormalize everything with a0
        let a0 = 1.0 + alpha;
        self.b0 = T::from_f32(((1.0 + cos_omega0) / 2.0) / a0);
        self.b1 = T::from_f32(-(1.0 + cos_omega0) / a0);
        self.b2 = T::from_f32(((1.0 + cos_omega0) / 2.0) / a0);
        self.a1 = T::from_f32((-2.0 * cos_omega0) / a0);
        self.a2 = T::from_f32((1.0 - alpha) / a0);
    }

    /// Compute the coefficients for an all-pass filter.
    ///
    /// Based on <http://shepazu.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html>.
    pub fn allpass(&mut self, sample_rate: f32, frequency: f32, q: f32) {
        nih_debug_assert!(sample_rate > 0.0);
        nih_debug_assert!(frequency > 0.0);
        nih_debug_assert!(frequency < sample_rate / 2.0);
        nih_debug_assert!(q > 0.0);

        let omega0 = consts::TAU * (frequency / sample_rate);
        let cos_omega0 = omega0.cos();
        let alpha = omega0.sin() / (2.0 * q);

        // We'll prenormalize everything with a0
        let a0 = 1.0 + alpha;
        self.b0 = T::from_f32((1.0 - alpha) / a0);
        self.b1 = T::from_f32((-2.0 * cos_omega0) / a0);
        self.b2 = T::from_f32((1.0 + alpha) / a0);
        self.a1 = T::from_f32((-2.0 * cos_omega0) / a0);
        self.a2 = T::from_f32((1.0 - alpha) / a0);
    }

    /// Compute the coefficients for a bandpass filter.
    ///
    /// Based on <http://shepazu.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html>.
    pub fn bandpass(&mut self, sample_rate: f32, frequency: f32, q: f32) {
        nih_debug_assert!(sample_rate > 0.0);
        nih_debug_assert!(frequency > 0.0);
        nih_debug_assert!(frequency < sample_rate / 2.0);
        nih_debug_assert!(q > 0.0);

        let omega0 = consts::TAU * (frequency / sample_rate);
        let cos_omega0 = omega0.cos();
        let alpha = omega0.sin() / (2.0 * q);

        // We'll prenormalize everything with a0
        let a0 = 1.0 + alpha;
        self.b0 = T::from_f32(alpha / a0);
        self.b1 = T::from_f32(0.0);
        self.b2 = T::from_f32(-alpha / a0);
        self.a1 = T::from_f32((-2.0 * cos_omega0) / a0);
        self.a2 = T::from_f32((1.0 - alpha) / a0);
    }
}

impl SimdType for f32 {
    #[inline(always)]
    fn from_f32(value: f32) -> Self {
        value
    }
}

/*impl SimdType for f32x2 {
    #[inline(always)]
    fn from_f32(value: f32) -> Self {
        f32x2::splat(value)
    }
}*/

fn drywet<T: SimdType>(dry: T, wet: T, wetness: f32) -> T {
    let wetness_t = T::from_f32(wetness);
    let dry_factor = T::from_f32(1.0 - wetness);

    wet * wetness_t + dry * dry_factor
}