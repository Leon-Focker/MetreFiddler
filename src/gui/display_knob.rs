use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;

/// Repaint interval for knobs backed by a Getter instead of a signal.
const REPAINT_INTERVAL: Duration = Duration::from_millis(20);

/// A slider that integrates with NIH-plug's [`Param`] types. Use the
/// [`set_style()`][ParamSliderExt::set_style()] method to change how the value gets displayed.
#[allow(dead_code)]
pub struct DisplayKnob {
    /// A specific label to use instead of displaying the parameter's value.
    label_override: Option<String>,
    /// Whether the widget is drawn vertical or horizontal.
    vertical: Signal<bool>,
    /// Either a Signal or Getter for a Value
    value_source: ValueSource,
}

enum ValueSource {
    Signal(SyncSignal<f32>),
    Getter(Arc<dyn Fn() -> f32 + Send + Sync>),
}

impl DisplayKnob {
    pub fn new(
        cx: &mut Context,
        value: SyncSignal<f32>,
    ) -> Handle<'_, Self>
    {
        let vertical = Signal::from(false);

        Self {
            label_override: None,
            vertical,
            value_source: ValueSource::Signal(value),
        }
            .build(cx, |_| {})
            .bind(value, |mut handle| handle.needs_redraw())
            // To override the css styling:
            .width(Pixels(20.0))
            .height(Pixels(180.0))
    }

    pub fn new_with_getter(
        cx: &mut Context,
        getter: impl Fn() -> f32 + Send + Sync + 'static,
    ) -> Handle<'_, Self>
    {
        let vertical = Signal::from(false);
        let getter = Arc::new(getter);

        Self {
            label_override: None,
            vertical,
            value_source: ValueSource::Getter(getter),
        }
            .build(
                cx,
                |cx| {
                    let knob_entity = cx.current();

                    // Register a Timer to periodically redraw the view
                    let repaint_timer = cx.add_timer(REPAINT_INTERVAL, None, move |cx, action| {
                        if matches!(action, TimerAction::Tick(_)) {
                            cx.with_current(knob_entity, |cx| cx.needs_redraw());
                        }
                    });
                    cx.start_timer(repaint_timer);
                }
            )
            // To override the css styling:
            .width(Pixels(20.0))
            .height(Pixels(180.0))
    }

    fn current_value(&self) -> f32 {
        match &self.value_source {
            ValueSource::Signal(value) => value.get(),
            ValueSource::Getter(getter) => getter(),
        }
    }
}

impl View for DisplayKnob {
    fn element(&self) -> Option<&'static str> {
        Some("display-knob")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let value = self.current_value().clamp(0.0, 1.0);
        let vertical = self.vertical.get();
        let bounds = cx.bounds();

        let bar_start: (f32, f32) = if vertical {
            (bounds.x + bounds.w / 2.0, bounds.y)
        } else {
            (bounds.x, bounds.y + bounds.h / 2.0)
        };
        let bar_end = if vertical {
            (bounds.x + bounds.w / 2.0, bounds.y + bounds.h)
        } else {
            (bounds.x + bounds.w, bounds.y + bounds.h / 2.0)
        };

        let radius = 5.0;
        let center_x = bounds.x + bounds.w / 2.0;
        let center_y = bounds.y + bounds.h / 2.0;

        let travel_x = (bounds.w - 2.0 * radius).max(0.0);
        let travel_y = (bounds.h - 2.0 * radius).max(0.0);

        let (x, y) = if vertical {
            // Value 1 is at the top, matching slider_fill_view().
            (
                center_x,
                bounds.y + radius + (1.0 - value) * travel_y,
            )
        } else {
            (
                bounds.x + radius + value * travel_x,
                center_y,
            )
        };

        // The black Slider Bar
        let mut path = vg::PathBuilder::new();
        path.move_to(bar_start);
        path.line_to(bar_end);


        let mut paint = vg::Paint::default();
        paint.set_color(Color::black());
        paint.set_stroke_width(2.0);
        paint.set_style(vg::PaintStyle::Stroke);
        canvas.draw_path(&path.snapshot(), &paint);

        // The red Slider Knob
        let mut path = vg::PathBuilder::new();
        path.add_circle((x, y), radius, None);

        let mut paint = vg::Paint::default();
        paint.set_color(RGBA::rgba(172, 53, 53, 255));
        paint.set_anti_alias(true);

        canvas.draw_path(&path.snapshot(), &paint);
    }
}

/// Extension methods for [`ParamDisplayKnob`] handles.
pub trait DisplayKnobExt {
    /// Set slider to vertical
    fn set_vertical(self, value: bool) -> Self;
}

impl DisplayKnobExt for Handle<'_, DisplayKnob> {
    fn set_vertical(self, value: bool) -> Self {
        self.modify(|param_slider: &mut DisplayKnob| param_slider.vertical.update(|b| *b = value))
    }
}