use vizia_plug::vizia::prelude::*;
use crate::gui::param_binding::ParamBinding;
use crate::metre::interpolation::interpolation::InterpolationData;
use crate::util::dry_wet;

#[derive(Lens)]
pub struct ParamTicks {}

impl ParamTicks {
    pub fn new<L>(
        cx: &mut Context,
        interpolation_data: L,
        interpolate: f32,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = InterpolationData>,
    {
        Self {}
            .build(
                cx,
                |cx| {
                    Binding::new(cx, interpolation_data, move |cx, data| {
                        Self::ticks(
                            cx,
                            data,
                            interpolate,
                        );
                    });
                }
            )
            .hoverable(false)
    }

    fn ticks(
        cx: &mut Context,
        interpolation_data: impl Lens<Target = InterpolationData>,
        interpolate: f32,
    ) {
        HStack::new(cx, |cx| {
            Element::new(cx)
                .background_color(Color::black())
                .width(Pixels(1.0))
                .height(Pixels(10.0));

            for (dur_a, dur_b) in interpolation_data.get(cx).value {

                Element::new(cx)
                    .width(Stretch(dry_wet(dur_a, dur_b, interpolate)))
                    .height(Pixels(10.0));

                // TODO would we want the ticks to be different heights depending on indisp_val?
                Element::new(cx)
                    .background_color(Color::black())
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