use nice_plug::prelude::{Editor};
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};
use vizia_plug::vizia::icons::{ICON_SETTINGS, ICON_CHECK, ICON_X};
use std::sync::{Arc};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use nice_plug::nice_log;
use vizia_plug::vizia::input::Key::AltGraph;
use crate::{MetreFiddlerParams};
use crate::editor::MetreFiddlerEvent::*;
use crate::gui::param_label::ParamLabel;
use crate::gui::param_slider_knob::{ParamSliderKnob, ParamSliderKnobExt};
use crate::gui::param_slider_vertical::{ParamSliderV, ParamSliderVExt};
use crate::gui::param_slider_vertical::ParamSliderStyle::Scaled;
use crate::gui::settings_button::{SettingsButton, SettingsButtonModifiers};
use crate::gui::param_binding::ParamBinding;
use crate::gui::display_knob::DisplayKnob;
use crate::gui::metre_input::MetreInput;
use crate::gui::param_ticks::ParamTicks;
use crate::metre::interpolation::interpolation_data::InterpolationData;
use crate::metre::metre_data::MetreData;
use crate::metre::metre_slot::MetreSlot;
use crate::metre::metre_slot::MetreSlot::*;
// TODO Click+Alt does not seem to work properly with vizia-plug? it just sometimes detects alt and
//  sometimes it doesn't. (only on linux)

pub const NOTO_SANS: &str = "Noto Sans";

const PLUGIN_INFO_TEXT: &str = "     Below you can define a metric structure using RQQ notation, i.e. hierarchical
     lists of proportions. Each list begins with a total duration, followed by a 
     sub-list of relative durations. These define the relative length of each beat 
     in a bar. Each relative duration can be replaced by another RQQ list.

     The calculation of each beats weight is inspired by Clarance Barlows 
     indispensability function.
   
     The subdivision into of these nested lists defines the metric hierarchy 
     (metric groupings). Instead of a Space, you could also use ',' to separate
     elements. The following examples describe a bar in 6/8 compared to 3/4:

     (6  ((3 (1 1 1))  (3 (1 1 1))))
     (6  ((2 (1 1))  (2 (1 1))  (2 (1 1))))
 ";

const NEW_STYLE: &str = r#"
    .red_button:checked {
        background-color: #ac3535;
    }
"#;

pub(crate) struct AppData {
    pub(crate) params: Arc<MetreFiddlerParams>,
    // settings
    pub interpolate_durations: SyncSignal<bool>,
    pub many_velocities: SyncSignal<bool>,
    pub midi_out_one_note: SyncSignal<bool>,
    pub interpolate_indisp: SyncSignal<bool>,
    pub retain_metric_phase: SyncSignal<bool>,
    // others
    pub(crate) screen: Signal<MetreFiddlerScreen>,
    pub(crate) displayed_position: SyncSignal<f32>,
    pub(crate) interpolation_data_snapshot: SyncSignal<InterpolationData>,
    pub(crate) last_input_is_valid: Signal<bool>,
    pub(crate) display_which_metre: Signal<MetreSlot>,
    pub(crate) display_validity: Signal<bool>,
    pub(crate) textbox_expanded: Signal<bool>,
    pub(crate) text_input_a: Signal<String>,
    pub(crate) text_input_b: Signal<String>,
    pub(crate) max_threshold: Signal<usize>,
    pub(crate) current_nr_beats: Signal<usize>,
    // pub(crate) check_for_phase_reset_toggle: Signal<bool>,   // this is toggled for every frame until the phase_reset button has been reset
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MetreFiddlerScreen {
    Main,
    Settings,
    Info,
}

#[derive(Debug, Clone)]
pub(crate) enum MetreFiddlerEvent {
    UpdateString(String, MetreSlot),
    SetScreen(MetreFiddlerScreen),
    ToggleInterpolateDurs,
    ToggleInterpolateIndisp,
    ToggleManyVelocities,
    ToggleMidiOutput,
    ToggleRetainPhase,
    TriggerPhaseReset,
    RevertPhaseReset,
    ToggleCheckForPhaseReset,
    ToggleAB,
    DisplayValidity(bool),
    ExpandTextBox(bool),
}

impl Model for AppData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|my_event, _meta| match my_event {
            UpdateString(new_text, slot) => {
                match MetreData::try_from(new_text.as_str()) {
                    // TODO its seems like currently edited non-parameter persistentfields like
                    // metre_data don't mark plugin state as dirty and thus are not automatically
                    // saved until a parameter is changed...
                    Ok(new_metre_data) => {
                        let mut metric_data = self.params.combined_metre_data.lock().unwrap();

                        match slot {
                            MetreA => {
                                if self.text_input_a.get() != *new_text {
                                    self.text_input_a.update(|a| *a = new_text.clone());
                                }
                                metric_data.set_metre_a(new_metre_data);
                            },
                            _ => {
                                if self.text_input_b.get() != *new_text {
                                    self.text_input_b.update(|a| *a = new_text.clone());
                                }
                                metric_data.set_metre_b(new_metre_data);
                            },
                        }

                        self.max_threshold.update(|a| *a = metric_data.metre_a().max.max(metric_data.metre_b().max));
                        self.interpolation_data_snapshot.update(|a| *a = metric_data.interpolation_data().clone());
                        self.last_input_is_valid.update(|a| *a = true);
                        if self.interpolate_durations.get() {
                            self.params.current_nr_of_beats.store(metric_data.get_interpolated_durations(self.params.interpolate_a_b.value()).count(), Release);
                        } else {
                            self.params.current_nr_of_beats.store(metric_data.get_interleaved_durations(self.params.interpolate_a_b.value()).count(), Release);
                        }
                    },
                    Err(err_string) => {
                        nice_log!("Failed to parse string: '{}': {}", new_text, err_string);
                        self.last_input_is_valid.update(|a| *a = false);
                    },
                }
            }
            ToggleInterpolateDurs => {
                self.params.interpolate_durations.store(!self.params.interpolate_durations.load(Relaxed), Relaxed);
                self.interpolate_durations.update(|s| *s = !*s);
            }
            ToggleInterpolateIndisp => {
                self.params.interpolate_indisp.store(!self.params.interpolate_indisp.load(Relaxed), Relaxed);
                self.interpolate_indisp.update(|s| *s = !*s);
            }
            ToggleManyVelocities => {
                self.params.many_velocities.store(!self.params.many_velocities.load(Relaxed), Relaxed);
                self.many_velocities.update(|s| *s = !*s);
            }
            ToggleMidiOutput => {
                self.params.midi_out_one_note.store(!self.params.midi_out_one_note.load(Relaxed), Relaxed);
                self.midi_out_one_note.update(|s| *s = !*s);
            }
            ToggleRetainPhase => {
                self.params.retain_metric_phase.store(!self.params.retain_metric_phase.load(Relaxed), Relaxed);
                self.retain_metric_phase.update(|s| *s = !*s);
            }
            SetScreen(screen) => {
                self.screen.set(*screen);
            }
            ToggleAB => {
                self.display_which_metre.update(|m| *m = !*m)
            }
            TriggerPhaseReset => {
                // TODO
                // self.params.reset_info.store(true, Release);
                // self.check_for_phase_reset_toggle = !self.check_for_phase_reset_toggle;
                //
                // let param_ref = &self.params.reset_phase;
                //
                // cx.emit(ParamEvent::BeginSetParameter(param_ref).upcast());
                // cx.emit(ParamEvent::SetParameter(param_ref, true).upcast());
                // cx.emit(ParamEvent::EndSetParameter(param_ref).upcast());
            }
            RevertPhaseReset => {
                // TODO
                let param_ref = &self.params.reset_phase;

                cx.emit(ParamEvent::BeginSetParameter(param_ref).upcast());
                cx.emit(ParamEvent::SetParameter(param_ref, false).upcast());
                cx.emit(ParamEvent::EndSetParameter(param_ref).upcast());
            }
            ToggleCheckForPhaseReset => {
                // TODO
                // if !self.params.reset_info.load(Acquire) {
                //     cx.emit(RevertPhaseReset);
                // } else {
                //     self.check_for_phase_reset_toggle = !self.check_for_phase_reset_toggle;
                // }
            }
            DisplayValidity(show) => {
                self.display_validity.update(|s| *s = *show);
            }
            ExpandTextBox(expand) => {
                self.textbox_expanded.update(|e| *e = *expand);
            },
        });
    }
}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (500, 350))
}

pub(crate) fn create(
    params: Arc<MetreFiddlerParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        // add new styling
        let _ = cx.add_stylesheet(NEW_STYLE);
        let metric_data = params.combined_metre_data.lock().unwrap();

        // App Parameters
        let screen = Signal::from(MetreFiddlerScreen::Main);
        let interpolation_data_snapshot = SyncSignal::from(metric_data.interpolation_data().clone());
        let displayed_position = SyncSignal::from(params.displayed_position.load(Relaxed));
        let interpolate_durations = SyncSignal::from(params.interpolate_durations.load(Relaxed));
        let many_velocities = SyncSignal::from(params.many_velocities.load(Relaxed));
        let midi_out_one_note = SyncSignal::from(params.midi_out_one_note.load(Relaxed));
        let interpolate_indisp = SyncSignal::from(params.interpolate_indisp.load(Relaxed));
        let retain_metric_phase= SyncSignal::from(params.retain_metric_phase.load(Relaxed));
        let last_input_is_valid = Signal::new(true);
        let display_which_metre = Signal::from(MetreA);
        let display_validity = Signal::from(true);
        let text_input_a = Signal::from(metric_data.metre_a().string.clone());
        let text_input_b = Signal::from(metric_data.metre_b().string.clone());
        let textbox_expanded = Signal::from(false);
        let max_threshold = Signal::from(metric_data.metre_a().max.max(metric_data.metre_b().max));
        let current_nr_beats = Signal::from(0); // TODO
        let displayed_position = SyncSignal::from(params.displayed_position.load(Relaxed));

        // check_for_phase_reset_toggle: false,

        AppData {
            params: params.clone(),
            screen,
            interpolate_durations,
            many_velocities,
            midi_out_one_note,
            interpolate_indisp,
            retain_metric_phase,
            // interpolation_data_snapshot: Signal::from(metric_data.interpolation_data().clone()),
            displayed_position,
            // check_for_phase_reset_toggle: false,
            interpolation_data_snapshot,
            last_input_is_valid,
            display_which_metre,
            display_validity,
            textbox_expanded,
            text_input_a,
            text_input_b,
            max_threshold,
            current_nr_beats,
        }
            .build(cx);

        // This is a kinda hacky way to get the button and BoolParm to reset itself, but keeping
        // DAW Automation possible...
        // Binding::new(cx, Data::check_for_phase_reset_toggle, |cx, _was_reset| {
        //     cx.emit(ToggleCheckForPhaseReset);
        // });

        let binding_params = params.clone();

       Binding::new(cx, screen, move |cx| {
           let binding_params1 = binding_params.clone();
           let binding_params2 = binding_params.clone();

           match screen.get() {
               MetreFiddlerScreen::Settings => {
                   settings_window(cx);
               },
               MetreFiddlerScreen::Main => {
                   // Upper Part of the Plugin
                   VStack::new(cx,  move |cx| {
                       upper_part(cx,
                                  binding_params1.clone(),
                                  many_velocities,
                                  max_threshold,
                                  current_nr_beats,
                                  displayed_position);
                   })
                       .height(Stretch(3.0));
                   // Lower Part of the Plugin
                   lower_part(cx,
                              text_input_a,
                              text_input_b,
                              screen,
                              binding_params2.clone(),
                              display_which_metre,
                              display_validity,
                              last_input_is_valid,
                              textbox_expanded);
               }
               MetreFiddlerScreen::Info => {
                   // Upper Part of the Plugin
                   VStack::new(cx, |cx| {
                       metre_info_screen(cx);
                   })
                       .height(Stretch(3.0));
                   // Lower Part of the Plugin
                   lower_part(cx,
                              text_input_a,
                              text_input_b,
                              screen,
                              binding_params.clone(),
                              display_which_metre,
                              display_validity,
                              last_input_is_valid,
                              textbox_expanded);
               }
           };
       });

        // this doesn't work?
        // ResizeHandle::new(cx).background_color(Color::red());
    })
}

fn metre_info_screen(cx: &mut Context) {
    Element::new(cx)
        .background_color(RGBA::rgba(250, 250, 250, 255))
        .opacity(1.0);
    Label::new(cx, "")
        .text(PLUGIN_INFO_TEXT)
        .top(Pixels(5.0))
        // better too small than clipping
        .font_size(12.0);
}

fn settings_window(cx: &mut Context) {
    Element::new(cx).height(Pixels(25.0));
    Label::new(cx, "Settings")
        .alignment(Alignment::Center)
        .width(Stretch(1.0))
        .font_family(vec![FamilyOwned::Named(String::from(NOTO_SANS))])
        .font_weight(FontWeightKeyword::Thin)
        .font_size(40.0)
        .height(Pixels(50.0));

    Element::new(cx).height(Pixels(5.0));

    // Settings
    ScrollView::new(cx, move |cx| {
        VStack::new(cx, |cx| {
            SettingsButton::new(cx, cx.data::<AppData>().interpolate_durations, "Interpolate Durations".to_string())
                .on_button_press(|cx| cx.emit(ToggleInterpolateDurs));
            settings_divider(cx);
            SettingsButton::new(cx, cx.data::<AppData>().interpolate_indisp, "Interpolate Indispensability Values".to_string())
                .on_button_press(|cx| cx.emit(ToggleInterpolateIndisp));
            settings_divider(cx);
            SettingsButton::new(cx, cx.data::<AppData>().many_velocities, "Accent-Mode: Only two distinct Velocities".to_string())
                .on_button_press(|cx| cx.emit(ToggleManyVelocities));
            settings_divider(cx);
            SettingsButton::new(cx, cx.data::<AppData>().midi_out_one_note, "Send different Pitches According to Indispensability".to_string())
                .on_button_press(|cx| cx.emit(ToggleMidiOutput));
            settings_divider(cx);
            SettingsButton::new(cx, cx.data::<AppData>().retain_metric_phase, "Retain Metric Phase when changing \nMetric Duration during Playback".to_string())
                .on_button_press(|cx| cx.emit(ToggleRetainPhase));
        });
    })
        .width(Stretch(1.0))
        .height(Stretch(1.0));

    // Settings Icon
    HStack::new(cx, |cx| {
        ZStack::new(cx, |cx| {
            Svg::new(cx, ICON_SETTINGS).width(Stretch(1.0)).height(Stretch(1.0)).cursor(CursorIcon::Hand).fill(Color::black());
        })
            .hoverable(true)
            .on_press(|cx|cx.emit(SetScreen(MetreFiddlerScreen::Main)))
            .width(Pixels(24.0))
            .height(Pixels(24.0));
        Element::new(cx)
            .width(Pixels(24.0));
    })
        .width(Stretch(1.0))
        .alignment(Alignment::Right)
        .height(Pixels(60.0));
}

fn settings_divider(cx: &mut Context) {
    HStack::new(cx, |cx| {
        Element::new(cx).width(Pixels(150.0));
        Element::new(cx).width(Pixels(150.0)).background_color(Color::lightgray()).height(Pixels(1.0));
    })
        .alignment(Alignment::Left)
        .height(Pixels(1.0));
}

// Upper Part of the Plugin
fn upper_part(cx: &mut Context,
              params: Arc<MetreFiddlerParams>,
              many_velocities: SyncSignal<bool>,
              max_threshold: Signal<usize>,
              current_nr_beats: Signal<usize>,
              displayed_position: SyncSignal<f32>) {
    let velocity_params = Arc::clone(&params);
    let threshold_params = Arc::clone(&params);
    let duration_params = Arc::clone(&params);

    HStack::new(cx, move |cx| {
        // The Velocity Sliders
        VStack::new(cx, move |cx| {
            HStack::new(cx, |cx| {
                Element::new(cx)
                    .width(Pixels(10.0));
                // min vel
                VStack::new(cx, |cx| {
                    ParamSliderV::new(cx, &velocity_params.velocity_min)
                        .set_style(Scaled {factor: 1});
                    Label::new(cx, "min");
                })
                    .padding_top(Pixels(20.0))
                    .alignment(Alignment::Center);
                // max vel
                VStack::new(cx, |cx| {
                    ParamSliderV::new(cx, &velocity_params.velocity_max)
                        .set_style(Scaled {factor: 1});
                    Label::new(cx, "max");
                })
                    .padding_top(Pixels(20.0))
                    .alignment(Alignment::Center);
                // Skew
                VStack::new(cx, |cx| {
                    ParamSliderKnob::new(cx, &velocity_params.velocity_skew)
                        .set_vertical(true);
                    let skew_label_params = Arc::clone(&velocity_params);
                    Binding::new(cx, many_velocities, move |cx | {
                        if many_velocities.get() {
                            Label::new(cx, "skew");
                        } else {
                            // This callback is also `Fn`, so clone here rather
                            // than moving its captured `Arc` into the child.
                            let params_for_beat_label = Arc::clone(&skew_label_params);
                            Binding::new(cx, current_nr_beats, move |cx| {
                                // TODO does this need a parambinding on interpolate_a_b?
                                let nr_beats = params_for_beat_label.current_nr_of_beats.load(Acquire) as f32;

                                ParamLabel::new(cx, &params_for_beat_label.velocity_skew, move |skew: f32| {
                                    ((skew * nr_beats).round() as usize).to_string()
                                })
                                    .alignment(Alignment::Center);
                            });
                        }
                    });
                })
                    .padding_top(Pixels(20.0))
                    .alignment(Alignment::Center);

                Element::new(cx)
                    .width(Pixels(10.0));
            });

            Label::new(cx, "Velocity")
                .font_weight(FontWeightKeyword::Bold)
                .padding_bottom(Pixels(20.0));
        })
            .alignment(Alignment::Center)
            .width(Stretch(1.0));

        // Middle Part (Name, Duration, Buttons)
        VStack::new(cx, move |cx| {
            Element::new(cx)
                .height(Pixels(25.0));
            Label::new(cx, "MetreFiddler")
                .font_family(vec![FamilyOwned::Named(String::from(NOTO_SANS))])
                .font_weight(FontWeightKeyword::Thin)
                .font_size(40.0)
                .height(Pixels(50.0));

            duration_position(cx, Arc::clone(&duration_params), displayed_position);

            Element::new(cx)
                .height(Pixels(10.0));
        })
            .alignment(Alignment::Center)
            .width(Stretch(2.0));

        // The Threshold Sliders
        VStack::new(cx, move |cx| {
            HStack::new(cx, |cx| {
                Binding::new(cx, max_threshold, move |cx| {
                    let max_val = max_threshold.get();

                    Element::new(cx)
                        .width(Pixels(10.0));
                    // min thresh
                    VStack::new(cx, |cx| {
                        ParamSliderV::new(cx, &threshold_params.lower_threshold)
                            .set_style(Scaled {factor: max_val});
                        Label::new(cx, "min");
                    })
                        .padding_top(Pixels(20.0))
                        .alignment(Alignment::Center);
                    // max thresh
                    VStack::new(cx, |cx| {
                        ParamSliderV::new(cx, &threshold_params.upper_threshold)
                            .set_style(Scaled { factor: max_val });
                        Label::new(cx, "max");
                    })
                        .padding_top(Pixels(20.0))
                        .alignment(Alignment::Center);
                    Element::new(cx)
                        .width(Pixels(10.0));
                });
            });

            Label::new(cx, "Thresholds")
                .font_weight(FontWeightKeyword::Bold)
                .padding_bottom(Pixels(20.0));
        })
            .alignment(Alignment::Center)
            .width(Stretch(1.0));
    });
}

// Lower Part of the Plugin, containing the Metre Definition
fn lower_part(cx: &mut Context,
              text_input_a: Signal<String>,
              text_input_b: Signal<String>,
              screen: Signal<MetreFiddlerScreen>,
              params: Arc<MetreFiddlerParams>,
              display_which_metre: Signal<MetreSlot>,
              display_validity: Signal<bool>,
              is_valid: Signal<bool>,
              textbox_expanded: Signal<bool>) {
    // The entire lower part
    VStack::new(cx, move |cx| {

        // First Row: Textfield, info and feedback:
        HStack::new(cx, |cx| {
            // Info Button
            VStack::new(cx, |cx| {
                Button::new(cx,
                            |cx| Label::new(cx, "info"))
                    .on_press(move |cx| {
                        match screen.get() {
                            MetreFiddlerScreen::Info => cx.emit(SetScreen(MetreFiddlerScreen::Main)),
                            _ => cx.emit(SetScreen(MetreFiddlerScreen::Info)),
                        }
                    })
                    .position_type(PositionType::Absolute)
                    .right(Pixels(10.0));
            });

            // Metre Input for A or B
            VStack::new(cx, |cx| {
                Binding::new(cx, display_which_metre, move |cx| {
                    Binding::new(cx, textbox_expanded,  move |cx| {
                        if textbox_expanded.get() {
                            Popover::new(cx, |cx| {
                                match display_which_metre.get() {
                                    MetreA =>  MetreInput::new(cx, text_input_a, MetreA),
                                    _ =>  MetreInput::new(cx, text_input_b, MetreB),
                                };
                            })
                                .lock_focus_to_within() // automatically move into popup textbox
                                .placement(Placement::Over)
                                .background_color(Color::yellowgreen())
                                .height(Pixels(75.0)); // TODO adjust size or add scrollable view in future?
                        } else {
                            match display_which_metre.get() {
                                MetreA =>  MetreInput::new(cx, text_input_a, MetreA),
                                _ =>  MetreInput::new(cx, text_input_b, MetreB),
                            };
                        }
                    });
                });
            })
                .width(Stretch(3.0));

            // is valid
            VStack::new(cx, move |cx| {
                Binding::new(cx, display_validity, move |cx| {
                    Binding::new(cx, is_valid, move |cx| {
                        if display_validity.get() {
                            if is_valid.get() {
                                Svg::new(cx, ICON_CHECK).fill(Color::black()).alignment(Alignment::BottomLeft);
                            } else {
                                Svg::new(cx, ICON_X).fill(RGBA::rgba(172, 53, 53, 255));
                            }
                        }
                    });
                })
            })
                .alignment(Alignment::Left);
        })
            .height(Pixels(32.0));

        // Second Row: Send Midi, Interpolation, Settings
        HStack::new(cx, |cx| {
            // Extra HStack with height 50p for alignment
            HStack::new(cx, |cx| {
                // Send Midi Events?
                VStack::new(cx, |cx| {
                    ParamButton::new(cx, &params.send_midi)
                        .alignment(Alignment::Center)
                        .with_label("Send Midi")
                        .class("red_button")
                        .width(Pixels(80.0));
                })
                    .alignment(Alignment::Center);

                // Switching A & B
                HStack::new(cx, move |cx| {
                    // Switch between A and B
                    Binding::new(cx, display_which_metre, move |cx| { // todo binding needed?
                        Button::new(cx,
                                    |cx|
                                        match display_which_metre.get() {
                                            MetreA => Label::new(cx, "Switch to B"),
                                            _ => Label::new(cx, "Switch to A"),
                                        }
                        )
                            .on_press(|cx| {
                                cx.emit(ToggleAB)
                            })
                            .alignment(Alignment::Center);
                    });

                    Element::new(cx).width(Pixels(10.0));

                    // Interpolation
                    HStack::new(cx, |cx| {
                        Label::new(cx, "A");

                        Element::new(cx).width(Pixels(10.0));

                        ParamSliderKnob::new(cx, &params.interpolate_a_b)
                            .height(Pixels(20.0))
                            .width(Pixels(100.0));

                        Element::new(cx).width(Pixels(10.0));

                        Label::new(cx, "B");
                    })
                        .alignment(Alignment::Center);
                })
                    .alignment(Alignment::Center)
                    .width(Stretch(3.0));

                // Settings
                HStack::new(cx, |cx| {
                    ZStack::new(cx, |cx| {
                        Svg::new(cx, ICON_SETTINGS).width(Stretch(1.0)).height(Stretch(1.0)).fill(Color::black());
                    })
                        .hoverable(true)
                        .on_press(|cx|cx.emit(SetScreen(MetreFiddlerScreen::Settings)))
                        .width(Pixels(24.0))
                        .height(Pixels(24.0));
                    Element::new(cx)
                        .width(Pixels(24.0));
                })
                    .width(Stretch(1.0))
                    .alignment(Alignment::Right);
            })
                .height(Pixels(50.0));
        })
            .alignment(Alignment::TopCenter)
            .height(Stretch(2.0));
    });
}


fn duration_position(cx: &mut Context, params: Arc<MetreFiddlerParams>, displayed_position: SyncSignal<f32>) {
    let duration_content_params = Arc::clone(&params);
    let position_params = Arc::clone(&params);

    VStack::new(cx, move |cx| {

        // Duration
        ParamBinding::new(
            cx,
            &params.use_position,
            move |cx, use_pos| {
                let duration_params = Arc::clone(&duration_content_params);

                ZStack::new(cx, move |cx| {
                    // Label that changes according to Parameter
                    VStack::new(cx, |cx| {
                        ParamLabel::new(
                            cx,
                            &duration_params.use_bpm,
                            |param: f32| {
                                if param < 0.5 {
                                    String::from("Duration in Seconds")
                                } else {
                                    String::from("Duration in Quarter Notes")
                                }
                            },
                        )
                            .alignment(Alignment::BottomCenter)
                            .font_weight(FontWeightKeyword::Bold);

                        ParamSlider::new(cx, &duration_params.metric_dur_selector)
                            .width(Pixels(200.0));

                        HStack::new(cx, |cx| {
                            // BPM Toggle
                            ParamButton::new(cx, &duration_params.use_bpm)
                                .class("red_button")
                                .with_label("  Use BPM")
                                .width(Pixels(100.0));
                            // Reset Phase
                            Button::new(
                                cx,
                                |cx| Label::new(cx, "reset phase"))
                                .on_press(|cx| {
                                    cx.emit(TriggerPhaseReset);
                                })
                                .width(Pixels(100.0));
                        })
                            .alignment(Alignment::Center)
                            .top(Pixels(10.0));
                    })
                        .alignment(Alignment::TopCenter);

                    // Hide Duration Gui when using the position slider
                    if use_pos > 0.5 {
                        Element::new(cx)
                            .background_color(RGBA::rgba(250, 250, 250, 255))
                            .opacity(1.0);
                    }
                })
                    .alignment(Alignment::Center);
            })
            .height(Stretch(0.4))
            .alignment(Alignment::Center);

        // Position
        VStack::new(cx, move |cx| {
            let position_toggle_params = Arc::clone(&position_params);
            HStack::new(cx, move |cx| {
                // Switch between Duration and Position
                ParamButton::new(cx, &position_toggle_params.use_position)
                    .class("red_button")
                    .with_label("Use")
                    .height(Pixels(20.0))
                    .width(Pixels(40.0));

                Label::new(
                    cx,
                    "  Position within Measure"
                )
                    .font_weight(FontWeightKeyword::Bold);
            })
                .alignment(Alignment::Center);

            let position_view_params = Arc::clone(&position_params);
            ZStack::new(cx, move |cx| {
                // TODO
                // The ticks on the position bar
                // VStack::new(cx, |cx| {
                //     Binding::new(cx, interpolate_durations,|cx| {
                //         ParamBinding::new(
                //             cx,
                //             &params.interpolate_a_b,
                //             move |cx, interpolate| {
                //                 ParamTicks::new(
                //                     cx,
                //                     200.0,
                //                     interpolation_data_snapshot,
                //                     interpolate,
                //                     interpolate_durations)
                //                     .height(Pixels(20.0));
                //             }).alignment(Alignment::Center);
                //     });
                // })
                //     .alignment(Alignment::Center);

                VStack::new(cx, |cx| {
                    // TODO explore, whether parambinding can be replaced with a binding to param.unmodulated_signal or similar
                    let position_slider_params = Arc::clone(&position_view_params);
                    ParamBinding::new(
                        cx,
                        &position_view_params.use_position,
                        move |cx, use_pos| {
                            let display_pos = use_pos < 0.5;

                            if display_pos {
                                DisplayKnob::new(
                                    cx,
                                    displayed_position)
                                    .height(Pixels(20.0))
                                    .width(Pixels(200.0));
                            } else {
                                ParamSliderKnob::new(cx, &position_slider_params.bar_position)
                                    .height(Pixels(20.0))
                                    .width(Pixels(200.0));
                            }
                        })
                        .alignment(Alignment::Center);
                })
                    .alignment(Alignment::Center);
            });
        })
            .alignment(Alignment::TopCenter)
            .height(Stretch(0.2));
     })
         .alignment(Alignment::Center);
}
