use vizia_plug::vizia::prelude::*;

use crate::util::dry_wet;

#[derive(Lens)]
pub struct ParamTicks {}

impl ParamTicks {
    pub fn new<L,M>(
        cx: &mut Context,
        durations_a: L,
        durations_b: M,
        interpolate: f32,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = Vec<f32>>,
        M: Lens<Target = Vec<f32>>,
    {
        Self {}
            .build(
                cx,
                |cx| {
                    Self::ticks(
                        cx,
                        durations_a,
                        durations_b,
                        interpolate,
                    );
                }
            )
            .hoverable(false)
    }
    
    fn ticks(
        cx: &mut Context,
        durations_a: impl Lens<Target = Vec<f32>>,
        durations_b: impl Lens<Target = Vec<f32>>,
        interpolate: f32,
    ) {
        let max_len = durations_a.get(cx).len().max(durations_b.get(cx).len());

        HStack::new(cx, |cx| {
            Element::new(cx)
                .background_color(Color::black())
                .width(Pixels(1.0))
                .height(Pixels(10.0));

            for i in 0..max_len {
                // cannot use .get() because the lens impl overloads it....?
                let dur_a = *durations_a.get(cx).iter().nth(i).unwrap_or(&0.0);
                let dur_b = *durations_b.get(cx).iter().nth(i).unwrap_or(&0.0);

                Element::new(cx)
                    // TODO cooler (smarter) interpolation?
                    .width(Stretch(dry_wet(dur_a, dur_b, interpolate)))
                    .height(Pixels(10.0));

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