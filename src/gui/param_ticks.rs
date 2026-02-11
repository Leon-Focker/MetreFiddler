use nih_plug::{nih_dbg, nih_log};
use num_traits::real::Real;
use vizia_plug::vizia::prelude::*;
use crate::metre::interpolation::interpolation::InterpolationData;
use crate::util::{get_durations, get_start_times};

#[derive(Lens)]
pub struct ParamTicks {}

impl ParamTicks {
    pub fn new<L>(
        cx: &mut Context,
        width_pixels: f32,
        interpolation_data: L,
        interpolate: f32,
        interpolate_durs: bool,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = InterpolationData>,
    {
        Self {}
            .build(
                cx,
                |cx| {
                    Binding::new(cx, interpolation_data, move |cx, data| {
                        Self::ticks(cx, data, interpolate, interpolate_durs, width_pixels);
                    });
                }
            )
            .width(Pixels(width_pixels))
            .hoverable(false)
    }

    fn ticks(
        cx: &mut Context,
        interpolation_data: impl Lens<Target = InterpolationData>,
        interpolate: f32,
        interpolate_durs: bool,
        width_px: f32,
    ) {
        HStack::new(cx, |cx| {
            Element::new(cx)
                .background_color(Color::black())
                .width(Pixels(1.0))
                .height(Pixels(10.0));

            // TODO clean this up a bit

            let durations: Vec<f32>;
            let initial_opacity_values: Vec<f32>;

            if interpolate_durs {
                durations = interpolation_data.get(cx).get_durations(interpolate).collect();
                initial_opacity_values = vec![2.0; durations.len()];
            } else {
                let starts_and_ids = interpolation_data.get(cx).get_unique_start_times();

                let (starts, inits): (Vec<f32>, Vec<f32>) = starts_and_ids.iter().copied().unzip();
                durations = get_durations(&starts);
                initial_opacity_values = inits[1..].to_vec();
            };

            let sum: f32 = durations.iter().sum();
            let nr_of_ticks = durations.len();
            let mut current_sum: f32 = 0.0;
            let mut last_sum: f32 = 0.0;
            let nr_of_pixels = (width_px.round() as usize).saturating_sub(2).saturating_sub(nr_of_ticks);

            for (dur, init_opacity) in durations.iter().zip(initial_opacity_values) {
                let float_pixels: f32 = dur / sum * nr_of_pixels as f32;
                current_sum += float_pixels;
                let width_in_pixels: f32 = current_sum.round() - last_sum.round();
                last_sum += float_pixels;

                let opacity = if interpolate_durs {
                    255
                } else {
                    // calculate opacity (init_opacity -1.0 -> MetreA, 0.0 -> MetreB, 1.0 -> both)
                    ((init_opacity + interpolate).abs().min(1.0) * 255.0).round() as u8
                };
                let color: Color = Color::rgba(0,0,0, opacity);


                // Draw the empty Space and the Ticks
                Element::new(cx)
                    .width(Pixels(width_in_pixels))
                    .height(Pixels(10.0));

                // TODO would we want the ticks to be different heights depending on indisp_val?
                Element::new(cx)
                    .background_color(color)
                    .width(Pixels(1.0))
                    .height(Pixels(10.0));
            }
        })
            .padding_left(Pixels(1.0))
            .padding_right(Pixels(1.0))
            .alignment(Alignment::Center);
    }
}

impl View for ParamTicks {
    fn element(&self) -> Option<&'static str> {
        Some("param-ticks")
    }
}