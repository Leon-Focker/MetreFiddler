use nih_plug::prelude::Param;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::param_base::ParamWidgetBase;

// TODO
/// A toggleable button that integrates with NIH-plug's [`Param`] types. Only makes sense with
/// [`BoolParam`][nih_plug::prelude::BoolParam]s. Clicking on the button will toggle between the
/// parameter's minimum and maximum value. The `:checked` pseudoclass indicates whether or not the
/// button is currently pressed.
#[derive(Lens)]
pub struct ParamLabel {}

impl ParamLabel {
    /// Creates a new [`ParamLabel`] for the given parameter. See
    /// [`ParamLabel`][super::ParamLabel] for more information on this function's arguments.
    pub fn new<L, Params, P, FMap, FLabel>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
        label_fn: FLabel,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
        FLabel: Fn(f32) -> String + 'static,
    {
        let label_fn_boxed = Box::new(label_fn);
        
        Self {}
            .build(
                cx,
                ParamWidgetBase::build_view(params, params_to_param, move |cx, param_data| {
                    let unmodulated_normalized_value_lens =
                        param_data.make_lens(|param| param.unmodulated_normalized_value());
                    Binding::new(cx, unmodulated_normalized_value_lens, move |cx, param| {
                        Label::new(cx, &label_fn_boxed(param.get(cx)))
                            .left(Stretch(1.0))
                            .right(Stretch(1.0));
                    })
                }),
            )
    }
}

impl View for ParamLabel {
}