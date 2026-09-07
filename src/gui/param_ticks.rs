use vizia_plug::vizia::prelude::*;
use nice_plug::prelude::*;
use vizia_plug::widgets::param_base::ParamWidgetBase;
use crate::metre::interpolation::interpolation_data;
use crate::metre::metre_slot::MetreSlot;
use crate::metre::metre_slot::MetreSlot::Both;
use crate::metre::interpolation::interpolation_data::InterpolationData;
use crate::util::{get_durations};

pub struct ParamTicks {}

impl ParamTicks {
    pub fn new<'c, 'p, P>(
        cx: &'c mut Context,
        width_pixels: f32,
        interpolation_data: SyncSignal<InterpolationData>,
        interpolation_param: &'p P,
        interpolate_durs: SyncSignal<bool>,
    ) -> Handle<'c, Self>
    where
        'p: 'c,
        P: Param + 'static,
    {
        let param_base = ParamWidgetBase::new(cx, interpolation_param);

        Self {}
            .build(
                cx,
                ParamWidgetBase::build_view(interpolation_param, move |cx, _ | {
                    let interpolation = param_base.modulated_signal(cx);

                    Binding::new(cx, interpolation_data, move |cx| {
                        Binding::new(cx, interpolation, move |cx| {
                            Binding::new(cx, interpolate_durs, move |cx| {
                                let interpolation_data = interpolation_data.get();
                                let interpolation = interpolation.get();
                                let interpolate_durs = interpolate_durs.get();

                                Self::ticks(cx, width_pixels, interpolation_data, interpolation, interpolate_durs);
                            });
                        });
                    });
                }),
            )
            .width(Pixels(width_pixels))
            .hoverable(false)
    }

    fn ticks(
        cx: &mut Context,
        width_px: f32,
        interpolation_data: InterpolationData,
        interpolation: f32,
        interpolate_durs: bool,
    ) {
        HStack::new(cx, |cx| {
            Element::new(cx)
                .background_color(Color::black())
                .width(Pixels(1.0))
                .height(Pixels(10.0));

            let durations: Vec<f32>;
            let opacity_ids: Vec<MetreSlot>;

            if interpolate_durs {
                durations = interpolation_data.get_interpolated_durations(interpolation).collect();
                opacity_ids = vec![Both; durations.len()];
            } else {
                let starts = interpolation_data.unique_start_times();
                let inits = interpolation_data.unique_start_time_origins();

                durations = get_durations(starts).collect();
                opacity_ids = inits[1..].to_vec();
            };

            let sum: f32 = durations.iter().sum();
            let nr_of_ticks = durations.len();
            let mut current_sum: f32 = 0.0;
            let mut last_sum: f32 = 0.0;
            let nr_of_pixels = (width_px.round() as usize).saturating_sub(2).saturating_sub(nr_of_ticks);

            for (dur, origin) in durations.iter().zip(opacity_ids) {
                let float_pixels: f32 = dur / sum * nr_of_pixels as f32;
                current_sum += float_pixels;
                let width_in_pixels: f32 = current_sum.round() - last_sum.round();
                last_sum += float_pixels;

                let opacity = if interpolate_durs {
                    255
                } else {
                    // calculate opacity (init_opacity -1.0 -> MetreA, 0.0 -> MetreB, 1.0 -> both)
                    origin.calculate_opacity(interpolation)
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
        Some("param_ticks")
    }
}