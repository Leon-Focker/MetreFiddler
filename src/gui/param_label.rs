use nice_plug::prelude::*;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::param_base::ParamWidgetBase;

pub struct ParamLabel {
}

impl ParamLabel {
    pub fn new<'c, 'p, P, L>(cx: &'c mut Context, param: &'p P, label_fn: L) -> Handle<'c, Self>
    where
        'p: 'c,
        P: Param + 'static,
        L: 'static + Fn(f32) -> String,
    {
        let param_base = ParamWidgetBase::new(cx, param);

        Self {}
            .build(
                cx,
                ParamWidgetBase::build_view(param, move |cx, _param_data| {
                    let unmodulated_signal = param_base.unmodulated_signal(cx);

                    Binding::new(cx, unmodulated_signal, move|cx| {
                        Label::new(cx, label_fn(unmodulated_signal.get()))
                            .left(Stretch(1.0))
                            .right(Stretch(1.0));
                    });
                }),
            )
    }
}

impl View for ParamLabel {
    fn element(&self) -> Option<&'static str> {
        Some("param-label")
    }
}