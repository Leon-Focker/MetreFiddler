use nice_plug::prelude::*;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::param_base::ParamWidgetBase;

pub struct ParamBinding {}

impl ParamBinding {
    pub fn new<'c, 'p, P, F>(cx: &'c mut Context, param: &'p P, content: F) -> Handle<'c, Self>
    where
        'p: 'c,
        P: Param + 'static,
        F: Fn(&mut Context, f32) + 'static,
    {
        let param_base = ParamWidgetBase::new(cx, param);
        
        Self {}
            .build(
                cx,
                ParamWidgetBase::build_view(param, move |cx, param_data| {
                    let param_value = param_base.modulated_signal(cx);
                    
                    Binding::new(cx, param_value, move |cx| {
                        content(cx, param_value.get());
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