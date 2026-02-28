// This is a modified copy of nih-plugs param_slider.rs
// ! A slider that integrates with NIH-plug's [`Param`] types.
use vizia_plug::vizia::prelude::*;

// TODO this is a lot of repeated code from param_slider_knob...

/// A slider that integrates with NIH-plug's [`Param`] types. Use the
/// [`set_style()`][ParamSliderExt::set_style()] method to change how the value gets displayed.
#[derive(Lens)]
#[allow(dead_code)]
pub struct ParamDisplayKnob {
    /// A specific label to use instead of displaying the parameter's value.
    label_override: Option<String>,
    /// Whether the widget is drawn vertical or horizontal.
    vertical: bool,
}

impl ParamDisplayKnob {
    pub fn new<L>(
        cx: &mut Context,
        value: L,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = f32>
    {
        Self {
            label_override: None,
            vertical: false,
        }
            .build(
                cx,
                |cx| {
                    Binding::new(cx, ParamDisplayKnob::vertical, move |cx, vertical| {
                        let vertical = vertical.get(cx);
                        
                        ZStack::new(cx, |cx| {
                            Self::slider_bar(
                                cx,
                                vertical,
                            );
                            Self::slider_fill_view(
                                cx,
                                vertical,
                                value,
                            );
                        })
                            .hoverable(false);
                    });
                }
            )
            // To override the css styling:
            .border_color(RGBA::rgba(250, 250, 250, 0))
            .background_color(RGBA::rgba(250, 250, 250, 0))
            .width(Pixels(20.0))
            .height(Pixels(180.0))
    }


    /// The black base line
    fn slider_bar(
        cx: &mut Context,
        vertical: bool
    ) {
        VStack::new(cx, |cx| {
            Element::new(cx)
                .background_color(Color::black())
                .height(
                    if vertical {
                        Percentage(100.0)
                    } else {
                        Pixels(2.0)
                    }
                )
                .width(
                    if vertical {
                        Pixels(2.0)
                    } else {
                        Percentage(100.0)
                    }
                );
        })
            .alignment(Alignment::Center);
    }

    /// Create the fill part of the slider.
    fn slider_fill_view(
        cx: &mut Context,
        vertical: bool,
        fill_start_delta_lens: impl Lens<Target = f32>,
    ) {
        if vertical {
            VStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    Element::new(cx)
                        .background_color(RGBA::rgba(172, 53, 53, 255))
                        .width(Pixels(10.0))
                        .height(Pixels(10.0))
                        .corner_radius(Percentage(50.0))
                        // Hovering is handled on the param slider as a whole, this
                        // should not affect that
                        .hoverable(false);
                })
                    .padding_top(fill_start_delta_lens.map(|delta| {
                        Percentage((1.0 - delta) * 100.0)
                    }))
                    .alignment(Alignment::TopCenter);
            })
                .padding_top(Pixels(-5.0))
                .padding_bottom(Pixels(5.0));
        } else {
            VStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    Element::new(cx)
                        .background_color(RGBA::rgba(172, 53, 53, 255))
                        .width(Pixels(10.0))
                        .height(Pixels(10.0))
                        .corner_radius(Percentage(50.0))
                        // Hovering is handled on the param slider as a whole, this
                        // should not affect that
                        .hoverable(false);
                })
                    .padding_right(fill_start_delta_lens.map(|delta| {
                        Percentage((1.0 - delta) * 100.0)
                    }))
                    .alignment(Alignment::Right);
            })
                .padding_right(Pixels(-5.0))
                .padding_left(Pixels(5.0));
        }
    }
}

impl View for ParamDisplayKnob {
    fn element(&self) -> Option<&'static str> {
        Some("param-slider")
    }
}

/// Extension methods for [`ParamDisplayKnob`] handles.
#[allow(dead_code)]
pub trait ParamDisplayKnobExt {
    /// Set slider to vertical
    fn set_vertical(self, value: bool) -> Self;
}

impl ParamDisplayKnobExt for Handle<'_, ParamDisplayKnob> {
    fn set_vertical(self, value: bool) -> Self {
        self.modify(|param_slider: &mut ParamDisplayKnob| param_slider.vertical = value)
    }
}