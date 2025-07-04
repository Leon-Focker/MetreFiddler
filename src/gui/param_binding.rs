use nih_plug::params::Param;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::param_base::ParamWidgetBase;

#[derive(Lens)]
pub struct ParamBinding {}

impl ParamBinding {
    pub fn new<L, Params, P, FMap, F>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
        content: F
    ) -> Handle<Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
        F: Fn(&mut Context, f32) + 'static,
    {
        Self {}
            .build(
                cx,
                ParamWidgetBase::build_view(params, params_to_param, move |cx, param_data| {
                    let param_value =
                        param_data.make_lens(|param| param.unmodulated_normalized_value());
                    
                    Binding::new(cx, param_value, move |cx, param| {
                        let test = param.get(cx);
                        content(cx, test);
                    });
                }),
            )
    }
}

impl View for ParamBinding {
    fn element(&self) -> Option<&'static str> {
        Some("param_binding")
    }
}