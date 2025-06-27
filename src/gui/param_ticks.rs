use vizia_plug::vizia::prelude::*;

#[derive(Lens)]
pub struct ParamTicks {}

impl ParamTicks {
    pub fn new<L>(
        cx: &mut Context,
        durations: L,
    ) -> Handle<Self>
    where
        L: Lens<Target = Vec<f32>>
    {
        Self {}
            .build(
                cx,
                |cx| {
                    Self::ticks(
                        cx,
                        durations,
                    );
                }
            )
            .hoverable(false)
    }
    
    fn ticks(
        cx: &mut Context,
        durations: impl Lens<Target = Vec<f32>>,
    ) {                
        HStack::new(cx, |cx| {
            Element::new(cx)
                .background_color(Color::black())
                .width(Pixels(1.0))
                .height(Pixels(10.0));

            for dur in durations.get(cx) {
                Element::new(cx)
                    .width(Stretch(dur))
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