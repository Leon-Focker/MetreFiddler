use nice_plug::params::Param;
use nice_plug::prelude::ParamPtr;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::param_base::ParamWidgetBase;
use vizia_plug::widgets::util::{self, ModifiersExt};
// This is a modified copy of nih-plugs param_slider.rs
// ! A slider that integrates with NIH-plug's [`Param`] types.


/// When shift+dragging a parameter, one pixel dragged corresponds to this much change in the
/// normalized parameter.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.1;

/// A slider that integrates with NIH-plug's [`Param`] types. Use the
/// [`set_style()`][ParamSliderExt::set_style()] method to change how the value gets displayed.
pub struct ParamSliderV {
    param_base: ParamWidgetBase,

    /// Set to `true` when the field gets Alt+Click'ed — replaces the label with a text box.
    text_input_active: SyncSignal<bool>,
    /// What style to use for the slider.
    style: SyncSignal<ParamSliderStyle>,
    /// A specific label to use instead of displaying the parameter's value.
    label_override: SyncSignal<Option<String>>,

    /// Set to `true` while we're dragging the parameter. Resetting the parameter or entering a
    /// text value should not initiate a drag.
    drag_active: bool,
    /// Start coordinate and normalized value when holding down Shift while dragging for higher
    /// precision dragging. `None` when granular dragging is not active.
    granular_drag_status: Option<GranularDragStatus>,

    // These fields are set through modifiers:
    /// Whether to listen to scroll events for changing the parameter's value in steps.
    use_scroll_wheel: bool,
    /// Fractional scrolled lines not yet turned into parameter change events. Needed for
    /// trackpads with smooth scrolling.
    scrolled_lines: f32,
}

/// How the [`ParamSliderV`] should display its values. Set this using
/// [`ParamSliderExt::set_style()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ParamSliderStyle {
    /// Visualize the offset from the default value for continuous parameters with a default value
    /// at around half of its range, fill the bar from the left for discrete parameters and
    /// continuous parameters without centered default values.
    Centered,
    /// Always fill the bar starting from the left.
    FromLeft,
    /// Fill the bar from the mid point, regardless of where the default value lies
    FromMidPoint,
    /// Show the current step instead of filling a portion of the bar, useful for discrete
    /// parameters. Set `even` to `true` to distribute the ticks evenly instead of following the
    /// parameter's distribution. This can be desirable because discrete parameters have smaller
    /// ranges near the edges (they'll span only half the range, which can make the display look
    /// odd).
    CurrentStep { even: bool },
    /// The same as `CurrentStep`, but overlay the labels over the steps instead of showing the
    /// active value. Only useful for discrete parameters with two, maybe three possible values.
    CurrentStepLabeled { even: bool },
    /// Like FromLeft but scale and round the displayed value. This is needed because the Range
    /// of an "IntParam" cannot easily be changed. Instead, a "FloatParam" can be used  and scaled
    /// with different factors.
    Scaled { factor: usize },
}

enum ParamSliderEvent {
    /// Text input has been canceled without submitting a new value.
    CancelTextInput,
    /// A new value has been sent by the text input dialog after pressing Enter.
    TextInput(String),
}

// TODO: Vizia's lens derive macro requires this to be marked as pub
#[derive(Debug, Clone, Copy)]
pub struct GranularDragStatus {
    /// The mouse's Y-coordinate when the granular drag was started.
    pub starting_y_coordinate: f32,
    /// The normalized value when the granular drag was started.
    pub starting_value: f32,
}

impl ParamSliderV {
    /// Creates a new [`ParamSliderV`] for the given parameter. Pass a reference to the
    /// parameter directly — e.g. `ParamSliderV::new(cx, &params.my_toggle)`.
    pub fn new<'c, 'p, P>(cx: &'c mut Context, param: &'p P) -> Handle<'c, Self>
    where
        'p: 'c,
        P: Param + 'static,
    {
        let param_base = ParamWidgetBase::new(cx, param);
        let text_input_active = SyncSignal::new(false);
        let style = SyncSignal::new(ParamSliderStyle::Centered);
        let label_override: SyncSignal<Option<String>> = SyncSignal::new(None);

        let unmodulated_signal = param_base.unmodulated_signal(cx);
        let modulated_signal = param_base.modulated_signal(cx);

        Self {
            param_base,
            text_input_active,
            style,
            label_override,
            drag_active: false,
            granular_drag_status: None,
            use_scroll_wheel: true,
            scrolled_lines: 0.0,
        }
            .build(
                cx,
                ParamWidgetBase::build_view(param, move |cx, param_data| {
                    Binding::new(cx, style, move |cx| {
                        let style = style.get();
                        let param_ptr = param_base.param_ptr();

                        // Derived display string. Single reactive input: the unmodulated value.
                        // SAFETY for the `ParamPtr` read: resolved from a valid `&impl Param` at
                        // widget construction; the pointer stays valid for the plugin's lifetime.
                        let display_value: Memo<String> = Memo::new(move |_| {
                            match style {
                                ParamSliderStyle::Scaled { factor: x } => {
                                    // We need this to circumvent the Range of the Param
                                    let current = unmodulated_signal.get();
                                    (current * x as f32).round().to_string()
                                },
                                _ => {
                                    let current = unmodulated_signal.get();
                                    unsafe { param_ptr.normalized_value_to_string(2.0 * current, true) }
                                },
                            }
                        });

                        // `(start_t, delta)` for the filled portion of the bar. `start_t ∈ [0, 1]`,
                        // `delta ∈ [-1, 1]`. Reactive input: the unmodulated value. The helper also
                        // reads static parameter metadata (default value, step count, step
                        // distribution) via the `ParamPtr`; those are invariant for the plugin's
                        // lifetime so they don't need to be tracked as reactive dependencies.
                        let fill_start_delta: Memo<(f32, f32)> = Memo::new(move |_| {
                            let current = unmodulated_signal.get();
                            Self::compute_fill_start_delta(style, param_ptr, current)
                        });

                        // Modulation offset bar. Reactive inputs: both unmodulated and modulated
                        // values — if either moves, the delta must be recomputed. Reading both
                        // via `.get()` inside the memo closure subscribes to both signals.
                        let modulation_start_delta: Memo<(f32, f32)> = Memo::new(move |_| {
                            let unmod = unmodulated_signal.get();
                            let modulated = modulated_signal.get();
                            Self::compute_modulation_fill_start_delta(style, unmod, modulated)
                        });

                        // Only draw the text input widget when it gets focussed. Otherwise, overlay the
                        // label with the slider. Creating the textbox based on
                        // `ParamSliderInternal::text_input_active` lets us focus the textbox when it gets
                        // created.
                        Binding::new(cx, text_input_active, move |cx| {
                            if text_input_active.get() {
                                Self::text_input_view(cx, display_value);
                            } else {
                                ZStack::new(cx, |cx| {
                                    Self::slider_fill_view(
                                        cx,
                                        fill_start_delta,
                                        modulation_start_delta,
                                    );
                                    Self::slider_label_view(
                                        cx,
                                        param_base,
                                        style,
                                        display_value,
                                        label_override,
                                    );
                                    // Re-Draw borders over fill Element
                                    Element::new(cx)
                                        .width(Stretch(1.0))
                                        .height(Stretch(1.0))
                                        .border_color(Color::black())
                                        .border_width(Pixels(1.0));
                                })
                                    .hoverable(false);
                            }
                        });
                    });
                }),
            )
            // To override the css styling:
            .width(Pixels(30.0))
            .height(Pixels(180.0))
    }

    /// Create a text input that's shown in place of the slider.
    fn text_input_view(cx: &mut Context, display_value: Memo<String>) {
        Textbox::new(cx, display_value)
            .class("value-entry")
            .on_submit(|cx, string, success| {
                if success {
                    cx.emit(ParamSliderEvent::TextInput(string))
                } else {
                    cx.emit(ParamSliderEvent::CancelTextInput);
                }
            })
            .on_cancel(|cx| {
                cx.emit(ParamSliderEvent::CancelTextInput);
            })
            .on_build(|cx| {
                cx.emit(TextEvent::StartEdit);
                cx.emit(TextEvent::SelectAll);
            })
            // `.child_space(Stretch(1.0))` no longer works
            .class("align_center")
            .alignment(Alignment::Left)
            .height(Stretch(1.0))
            .width(Stretch(1.0));
    }

    /// Create the fill part of the slider.
    fn slider_fill_view(
        cx: &mut Context,
        fill_start_delta: Memo<(f32, f32)>,
        modulation_start_delta: Memo<(f32, f32)>,
    ) {
        // The filled bar portion. This can be visualized in a couple different ways depending on
        // the current style property. See [`ParamSliderStyle`].
        Element::new(cx)
            .class("fill")
            .position_type(PositionType::Absolute)
            .background_color(RGBA::rgb(196, 196, 196))
            .width(Stretch(1.0))
            .bottom(fill_start_delta.map(|(start_t, _)| Percentage(start_t* 100.0)))
            .height(fill_start_delta.map(|(_, delta)| Percentage(delta * 100.0)))
            // Hovering is handled on the param slider as a whole, this
            // should not affect that
            .hoverable(false);

        // If the parameter is being modulated, then we'll display another
        // filled bar showing the current modulation delta
        // VIZIA's bindings make this a bit, uh, difficult to read
        // TODO didn't change anything here yet
        Element::new(cx)
            .class("fill")
            .class("fill--modulation")
            .width(Stretch(1.0))
            .visibility(modulation_start_delta.map(|(_, delta)| *delta != 0.0))
            // Widths cannot be negative, so we need to compensate the start
            // position if the width does happen to be negative
            .height(modulation_start_delta.map(|(_, delta)| Percentage(delta.abs() * 100.0)))
            .top(modulation_start_delta.map(|(start_t, delta)| {
                if *delta < 0.0 {
                    Percentage((start_t + delta) * 100.0)
                } else {
                    Percentage(start_t * 100.0)
                }
            }))
            .hoverable(false);
    }

    /// Create the text part of the slider. Shown on top of the fill using a `ZStack`.
    fn slider_label_view(
        cx: &mut Context,
        param_base: ParamWidgetBase,
        style: ParamSliderStyle,
        display_value: Memo<String>,
        label_override: SyncSignal<Option<String>>,
    ) {
        let step_count = param_base.step_count();

        // Either display the current value, or display all values over the
        // parameter's steps
        // TODO: Do the same thing as in the iced widget where we draw the
        //       text overlapping the fill area slightly differently. We can
        //       set the cip region directly in vizia.
        match (style, step_count) {
            (ParamSliderStyle::CurrentStepLabeled { .. }, Some(step_count)) => {
                HStack::new(cx, |cx| {
                    // There are step_count + 1 possible values for a
                    // discrete parameter
                    for value in 0..step_count + 1 {
                        let normalized_value = value as f32 / step_count as f32;
                        let preview = param_base.normalized_value_to_string(normalized_value, true);

                        Label::new(cx, preview)
                            .class("value")
                            .class("value--multiple")
                            .alignment(Alignment::Center)
                            .size(Stretch(1.0))
                            .hoverable(false);
                    }
                })
                    .height(Stretch(1.0))
                    .width(Stretch(1.0))
                    .hoverable(false);
            }
            _ => {
                // Derived label text: either the `.with_label(...)` override when set, or the
                // parameter's own formatted display value (before modulation). Built as a
                // `Memo<String>` so the Label updates its text in place when either input
                // changes — cheaper than rebuilding the view subtree via `Binding::new`.
                let text: Memo<String> =
                    Memo::new(move |_| label_override.get().unwrap_or_else(|| display_value.get()));
                Label::new(cx, text)
                    .class("value")
                    .class("value--single")
                    .alignment(Alignment::Center)
                    .size(Stretch(1.0))
                    .hoverable(false);
            }
        };
    }

    /// Calculate the start position and width of the slider's fill region based on the selected
    /// style, the parameter's current value, and the parameter's step sizes. The resulting tuple
    /// `(start_t, delta)` corresponds to the start and the signed width of the bar. `start_t` is in
    /// `[0, 1]`, and `delta` is in `[-1, 1]`.
    fn compute_fill_start_delta(
        style: ParamSliderStyle,
        param_ptr: ParamPtr,
        current_value: f32,
    ) -> (f32, f32) {
        // SAFETY: `param_ptr` was resolved from a valid `&impl Param` at widget construction;
        // it stays valid for the plugin's lifetime.
        let default_value = unsafe { param_ptr.default_normalized_value() };
        let step_count = unsafe { param_ptr.step_count() };
        let draw_fill_from_default = matches!(style, ParamSliderStyle::Centered)
            && step_count.is_none()
            && (0.45..=0.55).contains(&default_value);

        match style {
            ParamSliderStyle::Centered if draw_fill_from_default => {
                let delta = (default_value - current_value).abs();

                // Don't draw the filled portion at all if it could have been a
                // rounding error since those slivers just look weird
                (
                    1.0 - default_value.min(current_value),
                    if delta >= 1e-3 { delta } else { 0.0 },
                )
            }
            ParamSliderStyle::FromMidPoint => {
                let delta = (0.5 - current_value).abs();

                // Don't draw the filled portion at all if it could have been a
                // rounding error since those slivers just look weird
                (
                    0.5_f32.min(current_value),
                    if delta >= 1e-3 { delta } else { 0.0 },
                )
            }
            ParamSliderStyle::Centered | ParamSliderStyle::FromLeft
            | ParamSliderStyle::Scaled { .. } => (0.0, current_value),
            ParamSliderStyle::CurrentStep { even: true }
            | ParamSliderStyle::CurrentStepLabeled { even: true }
            if step_count.is_some() =>
                {
                    // Assume the normalized value is distributed evenly
                    // across the range.
                    let step_count = step_count.unwrap() as f32;
                    let discrete_values = step_count + 1.0;
                    let previous_step = (current_value * step_count) / discrete_values;

                    (previous_step, discrete_values.recip())
                }
            ParamSliderStyle::CurrentStep { .. } | ParamSliderStyle::CurrentStepLabeled { .. } => {
                let previous_step =
                    unsafe { param_ptr.previous_normalized_step(current_value, false) };
                let next_step = unsafe { param_ptr.next_normalized_step(current_value, false) };

                (
                    (previous_step + current_value) / 2.0,
                    ((next_step - current_value) + (current_value - previous_step)) / 2.0,
                )
            }
        }
    }

    /// The same as `compute_fill_start_delta`, but just showing the modulation offset.
    fn compute_modulation_fill_start_delta(
        style: ParamSliderStyle,
        unmodulated_normalized: f32,
        modulated_normalized: f32,
    ) -> (f32, f32) {
        match style {
            // Don't show modulation for stepped parameters — visually meaningless.
            ParamSliderStyle::CurrentStep { .. } | ParamSliderStyle::CurrentStepLabeled { .. } => {
                (0.0, 0.0)
            }
            ParamSliderStyle::Centered | ParamSliderStyle::FromLeft => (
                unmodulated_normalized,
                modulated_normalized - unmodulated_normalized,
            ),
            _ => (0.0, 0.0)
        }
    }

    /// `self.param_base.set_normalized_value()`, but resulting from a mouse drag. When using the
    /// 'even' stepped slider styles from [`ParamSliderStyle`] this will remap the normalized range
    /// to match up with the fill value display. This still needs to be wrapped in a parameter
    /// automation gesture.
    fn set_normalized_value_drag(&self, cx: &mut EventContext, normalized_value: f32) {
        let normalized_value = match (self.style.get(), self.param_base.step_count()) {
            (
                ParamSliderStyle::CurrentStep { even: true }
                | ParamSliderStyle::CurrentStepLabeled { even: true },
                Some(step_count),
            ) => {
                // We'll remap the value range to be the same as the displayed range, e.g. with each
                // value occupying an equal area on the slider instead of the centers of those
                // ranges being distributed over the entire `[0, 1]` range.
                let discrete_values = step_count as f32 + 1.0;
                let rounded_value = ((normalized_value * discrete_values) - 0.5).round();
                rounded_value / step_count as f32
            }
            _ => normalized_value,
        };

        self.param_base.set_normalized_value(cx, normalized_value);
    }
}

impl View for ParamSliderV {
    fn element(&self) -> Option<&'static str> {
        Some("param-slider")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|param_slider_event, meta| match param_slider_event {
            ParamSliderEvent::CancelTextInput => {
                self.text_input_active.set(false);
                cx.set_active(false);

                meta.consume();
            }
            ParamSliderEvent::TextInput(string) => {
                let normalized_value = match self.style.get() {
                    ParamSliderStyle::Scaled { factor } => {
                        string.parse::<f32>().ok()
                            .map(|val| (val / factor as f32).clamp(0.0, 1.0))
                    }
                    _ => self.param_base.string_to_normalized_value(string),
                };

                if let Some(normalized_value) = normalized_value {
                    self.param_base.begin_set_parameter(cx);
                    self.param_base.set_normalized_value(cx, normalized_value);
                    self.param_base.end_set_parameter(cx);
                }

                self.text_input_active.set(false);

                meta.consume();
            }
        });

        event.map(|window_event, meta| match window_event {
            // Vizia always captures the third mouse click as a triple click. Treating that triple
            // click as a regular mouse button makes double click followed by another drag work as
            // expected, instead of requiring a delay or an additional click. Double double click
            // still won't work.
            WindowEvent::MouseDown(MouseButton::Left)
            | WindowEvent::MouseTripleClick(MouseButton::Left) => {
                if cx.modifiers().alt() {
                    // ALt+Click brings up a text entry dialog
                    self.text_input_active.set(true);
                    cx.set_active(true);
                } else if cx.modifiers().command() {
                    // Ctrl+Click, double click, and right clicks should reset the parameter instead
                    // of initiating a drag operation
                    self.param_base.begin_set_parameter(cx);
                    self.param_base
                        .set_normalized_value(cx, self.param_base.default_normalized_value());
                    self.param_base.end_set_parameter(cx);
                } else if !self.text_input_active.get() {
                    // The `!self.text_input_active` check shouldn't be needed, but the textbox does
                    // not consume the mouse down event. So clicking on the textbox to move the
                    // cursor would also change the slider.
                    self.drag_active = true;
                    cx.capture();
                    // NOTE: Otherwise we don't get key up events
                    cx.focus();
                    cx.set_active(true);

                    // When holding down shift while clicking on a parameter we want to granuarly
                    // edit the parameter without jumping to a new value
                    self.param_base.begin_set_parameter(cx);
                    if cx.modifiers().shift() {
                        self.granular_drag_status = Some(GranularDragStatus {
                            starting_y_coordinate: cx.mouse().cursor_y,
                            starting_value: self.param_base.unmodulated_normalized_value(),
                        });
                    } else {
                        self.granular_drag_status = None;
                        self.set_normalized_value_drag(
                            cx,
                            // TODO set to 1.0 - ... to invert slider:
                            1.0 - util::remap_current_entity_y_coordinate(cx, cx.mouse().cursor_y),
                        );
                    }
                }

                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left)
            | WindowEvent::MouseDown(MouseButton::Right)
            | WindowEvent::MouseDoubleClick(MouseButton::Right)
            | WindowEvent::MouseTripleClick(MouseButton::Right) => {
                // Ctrl+Click, double click, and right clicks should reset the parameter instead of
                // initiating a drag operation
                self.param_base.begin_set_parameter(cx);
                self.param_base
                    .set_normalized_value(cx, self.param_base.default_normalized_value());
                self.param_base.end_set_parameter(cx);

                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_active {
                    self.drag_active = false;
                    cx.release();
                    cx.set_active(false);

                    self.param_base.end_set_parameter(cx);

                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_x, y) => {
                if self.drag_active {
                    // If shift is being held then the drag should be more granular instead of
                    // absolute
                    if cx.modifiers().shift() {
                        let granular_drag_status =
                            *self
                                .granular_drag_status
                                .get_or_insert_with(|| GranularDragStatus {
                                    starting_y_coordinate: *y,
                                    starting_value: self.param_base.unmodulated_normalized_value(),
                                });

                        // These positions should be compensated for the DPI scale so it remains
                        // consistent
                        let start_y =
                            util::remap_current_entity_y_t(cx, granular_drag_status.starting_value);
                        let delta_y = ((*y - granular_drag_status.starting_y_coordinate)
                            * GRANULAR_DRAG_MULTIPLIER)
                            * cx.scale_factor();

                        // TODO set to 1.0 - ... to invert slider:
                        self.set_normalized_value_drag(
                            cx,
                            1.0 - util::remap_current_entity_y_coordinate(cx, start_y + delta_y),
                        );
                    } else {
                        self.granular_drag_status = None;

                        self.set_normalized_value_drag(
                            cx,
                            1.0 - util::remap_current_entity_y_coordinate(cx, *y),
                        );
                    }
                }
            }
            WindowEvent::KeyUp(_, Some(Key::Shift)) => {
                // If this happens while dragging, snap back to reality uh I mean the current screen
                // position
                if self.drag_active && self.granular_drag_status.is_some() {
                    self.granular_drag_status = None;
                    self.param_base.set_normalized_value(
                        cx,
                        util::remap_current_entity_y_coordinate(cx, cx.mouse().cursor_y),
                    );
                }
            }
            WindowEvent::MouseScroll(_scroll_x, scroll_y) if self.use_scroll_wheel => {
                // With a regular scroll wheel `scroll_y` will only ever be -1 or 1, but with smooth
                // scrolling trackpads being a thing `scroll_y` could be anything.
                self.scrolled_lines += scroll_y;

                if self.scrolled_lines.abs() >= 1.0 {
                    let use_finer_steps = cx.modifiers().shift();

                    // Scrolling while dragging needs to be taken into account here
                    if !self.drag_active {
                        self.param_base.begin_set_parameter(cx);
                    }

                    let mut current_value = self.param_base.unmodulated_normalized_value();

                    while self.scrolled_lines >= 1.0 {
                        current_value = self
                            .param_base
                            .next_normalized_step(current_value, use_finer_steps);
                        self.param_base.set_normalized_value(cx, current_value);
                        self.scrolled_lines -= 1.0;
                    }

                    while self.scrolled_lines <= -1.0 {
                        current_value = self
                            .param_base
                            .previous_normalized_step(current_value, use_finer_steps);
                        self.param_base.set_normalized_value(cx, current_value);
                        self.scrolled_lines += 1.0;
                    }

                    if !self.drag_active {
                        self.param_base.end_set_parameter(cx);
                    }
                }

                meta.consume();
            }
            _ => {}
        });
    }
}

/// Extension methods for [`ParamSliderV`] handles.
pub trait ParamSliderVExt {
    /// Don't respond to scroll wheel events. Useful when this slider is used as part of a scrolling
    /// view.
    fn _disable_scroll_wheel(self) -> Self;

    /// Change how the [`ParamSliderV`] visualizes the current value.
    fn set_style(self, style: ParamSliderStyle) -> Self;

    /// Manually set a fixed label for the slider instead of displaying the current value. This is
    /// currently not reactive.
    fn _with_label(self, value: impl Into<String>) -> Self;
}

impl ParamSliderVExt for Handle<'_, ParamSliderV> {
    fn _disable_scroll_wheel(self) -> Self {
        self.modify(|param_slider: &mut ParamSliderV| param_slider.use_scroll_wheel = false)
    }

    fn set_style(self, style: ParamSliderStyle) -> Self {
        self.modify(|param_slider: &mut ParamSliderV| param_slider.style.set(style))
    }

    fn _with_label(self, value: impl Into<String>) -> Self {
        self.modify(|param_slider: &mut ParamSliderV| {
            param_slider.label_override.set(Some(value.into()))
        })
    }
}