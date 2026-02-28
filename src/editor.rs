use std::ops::DerefMut;
use nih_plug::prelude::{Editor};
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};
use vizia_plug::vizia::icons::ICON_SETTINGS;
use std::sync::{Arc};
use std::sync::atomic::Ordering::SeqCst;
use atomic_float::AtomicF32;
use nih_plug::{nih_log};
use crate::{MetreFiddlerParams};
use crate::editor::MetreFiddlerEvent::*;
use crate::gui::metre_input::{MetreAorB, MetreInput};
use crate::gui::metre_input::MetreAorB::{MetreA, MetreB};
use crate::gui::param_binding::ParamBinding;
use crate::gui::param_display_knob::ParamDisplayKnob;
use crate::gui::param_slider_vertical::{ParamSliderV, ParamSliderVExt};
use crate::gui::param_slider_vertical::ParamSliderStyle::{Scaled};
use crate::gui::param_label::{ParamLabel};
use crate::gui::param_slider_knob::{ParamSliderKnob, ParamSliderKnobExt};
use crate::gui::param_ticks::ParamTicks;
use crate::metre::interpolation::interpolation_data::InterpolationData;
use crate::metre::metre_data::MetreData;

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

#[derive(Lens, Clone)]
pub(crate) struct Data {
    pub(crate) params: Arc<MetreFiddlerParams>,
    pub(crate) screen: MetreFiddlerScreen,
    pub(crate) settings: Settings,
    pub(crate) interpolation_data_snapshot: InterpolationData,
    pub(crate) textbox_expanded: bool,
    pub(crate) text_input_a: String,
    pub(crate) text_input_b: String,
    pub(crate) display_b: bool,
    pub(crate) last_input_is_valid: bool,
    pub(crate) max_threshold: usize,
    pub(crate) display_metre_validity: bool,
    pub(crate) displayed_position: Arc<AtomicF32>,
    pub(crate) check_for_phase_reset_toggle: bool,   // this is toggled for every frame until the phase_reset button has been reset
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Settings {
    pub(crate) interpolate_durations: bool,
    pub(crate) interpolate_indisp: bool,
    pub(crate) many_velocities: bool,
    pub(crate) midi_out_one_note: bool,
}

impl vizia_plug::vizia::prelude::Data for Settings {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MetreFiddlerScreen {
    Main,
    Settings,
    Info,
}

impl vizia_plug::vizia::prelude::Data for MetreFiddlerScreen {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Debug, Clone)]
pub(crate) enum MetreFiddlerEvent {
    UpdateString(String, MetreAorB),
    SetScreen(MetreFiddlerScreen),
    ToggleInterpolateDurs,
    ToggleInterpolateIndisp,
    ToggleManyVelocities,
    ToggleMidiOutput,
    TriggerPhaseReset,
    RevertPhaseReset,
    ToggleCheckForPhaseReset,
    ToggleAB,
    ShowValidity(bool),
    ExpandTextBox(bool),
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|my_event, _meta| match my_event {
            UpdateString(new_text, which) => {
                match MetreData::try_from(new_text.as_str()) {
                    Ok(new_metre_data) => {
                        let mut metric_data = self.params.combined_metre_data.lock().unwrap();

                        match which {
                            MetreA => {
                                if self.text_input_a != *new_text {
                                    self.text_input_a = new_text.clone();
                                }
                                metric_data.deref_mut().set_metre_a(new_metre_data);
                            },
                            MetreB => {
                                if self.text_input_b != *new_text {
                                    self.text_input_b = new_text.clone();
                                }
                                metric_data.deref_mut().set_metre_b(new_metre_data);
                            },
                        }

                        self.max_threshold = metric_data.metre_a().max.max(metric_data.metre_b().max);
                        self.interpolation_data_snapshot = metric_data.interpolation_data().clone();
                        self.last_input_is_valid = true;
                        if self.settings.interpolate_durations {
                            self.params.current_nr_of_beats.store(metric_data.get_interpolated_durations(self.params.interpolate_a_b.value()).count(), SeqCst);
                        } else {
                            self.params.current_nr_of_beats.store(metric_data.get_interleaved_durations(self.params.interpolate_a_b.value()).count(), SeqCst);
                        }
                    },
                    Err(err_string) => {
                        nih_log!("Failed to parse string: '{}': {}", new_text, err_string);
                        self.last_input_is_valid = false;
                    },
                }
            }
            SetScreen(screen) => {
                self.screen = *screen;
            }
            ToggleInterpolateDurs => {
                self.params.interpolate_durations.store(!self.params.interpolate_durations.load(SeqCst), SeqCst);
                self.settings.interpolate_durations = !self.settings.interpolate_durations;
            }
            ToggleInterpolateIndisp => {
                self.params.interpolate_indisp.store(!self.params.interpolate_indisp.load(SeqCst), SeqCst);
                self.settings.interpolate_indisp = !self.settings.interpolate_indisp;
            }
            ToggleManyVelocities => {
                self.params.many_velocities.store(!self.params.many_velocities.load(SeqCst), SeqCst);
                self.settings.many_velocities = !self.settings.many_velocities;
            }
            ToggleMidiOutput => {
                self.params.midi_out_one_note.store(!self.params.midi_out_one_note.load(SeqCst), SeqCst);
                self.settings.midi_out_one_note = !self.settings.midi_out_one_note;
            }
            ToggleAB => {
                self.display_b = !self.display_b;
            }
            TriggerPhaseReset => {
                self.params.reset_info.store(true, SeqCst);
                self.check_for_phase_reset_toggle = !self.check_for_phase_reset_toggle;
                
                let param_ref = &self.params.reset_phase;

                cx.emit(ParamEvent::BeginSetParameter(param_ref).upcast());
                cx.emit(ParamEvent::SetParameter(param_ref, true).upcast());
                cx.emit(ParamEvent::EndSetParameter(param_ref).upcast());
            }
            RevertPhaseReset => {
                let param_ref = &self.params.reset_phase;

                cx.emit(ParamEvent::BeginSetParameter(param_ref).upcast());
                cx.emit(ParamEvent::SetParameter(param_ref, false).upcast());
                cx.emit(ParamEvent::EndSetParameter(param_ref).upcast());
            }
            ToggleCheckForPhaseReset => {
                if !self.params.reset_info.load(SeqCst) {
                    cx.emit(RevertPhaseReset);
                } else {
                    self.check_for_phase_reset_toggle = !self.check_for_phase_reset_toggle; 
                }
            }
            ShowValidity(show) => {
                self.display_metre_validity = *show;
            }
            ExpandTextBox(expand) => {
                self.textbox_expanded = *expand;
            }
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
        let settings = Settings {
            interpolate_durations: params.interpolate_durations.load(SeqCst),
            interpolate_indisp: params.interpolate_indisp.load(SeqCst),
            many_velocities: params.many_velocities.load(SeqCst),
            midi_out_one_note: params.midi_out_one_note.load(SeqCst),
        };
        
        Data {
            params: params.clone(),
            screen: MetreFiddlerScreen::Main,
            settings,
            interpolation_data_snapshot: metric_data.interpolation_data().clone(),
            max_threshold: metric_data.metre_a().max.max(metric_data.metre_b().max),
            text_input_a: metric_data.metre_a().string.clone(),
            text_input_b: metric_data.metre_b().string.clone(),
            last_input_is_valid: true,
            display_b: false,
            display_metre_validity: true,
            displayed_position: params.displayed_position.clone(),
            check_for_phase_reset_toggle: false,
            textbox_expanded: false,
        }
            .build(cx);

        // This is a kinda hacky way to get the button and BoolParm to reset itself, but keeping
        // DAW Automation possible...
        Binding::new(cx, Data::check_for_phase_reset_toggle, |cx, _was_reset| {
            cx.emit(ToggleCheckForPhaseReset);
        });

        VStack::new(cx, |cx| {

            Binding::new(cx, Data::screen, |cx, visible_screen| {
                match visible_screen.get(cx) {
                    MetreFiddlerScreen::Settings => {
                        settings_window(cx);
                    },
                    MetreFiddlerScreen::Main => {
                        // Upper Part of the Plugin
                        VStack::new(cx, |cx| {
                            upper_part(cx);
                        })
                            .height(Stretch(3.0));
                        // Lower Part of the Plugin
                        lower_part(cx);
                    }
                    MetreFiddlerScreen::Info => {
                        // Upper Part of the Plugin
                        VStack::new(cx, |cx| {
                            metre_info_screen(cx);
                        })
                            .height(Stretch(3.0));
                        // Lower Part of the Plugin
                        lower_part(cx);
                    }
                };
            });
        })
            // I have no clue, why I have to hardcode this? But without this, the HStacks are
            // not stretched properly
          .width(Pixels(500.0));

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

// Upper Part of the Plugin
fn upper_part(cx: &mut Context) {
    HStack::new(cx, |cx| {
        // The Velocity Sliders
        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Element::new(cx)
                    .width(Pixels(10.0));
                // min vel
                VStack::new(cx, |cx| {
                    ParamSliderV::new(cx, Data::params, |params|
                        &params.velocity_min)
                        .set_style(Scaled {factor: 1});
                    Label::new(cx, "min");
                })
                    .padding_top(Pixels(20.0))
                    .alignment(Alignment::Center);
                // max vel
                VStack::new(cx, |cx| {
                    ParamSliderV::new(cx, Data::params, |params|
                        &params.velocity_max)
                        .set_style(Scaled {factor: 1});
                    Label::new(cx, "max");
                })
                    .padding_top(Pixels(20.0))
                    .alignment(Alignment::Center);
                // Skew
                VStack::new(cx, |cx| {
                    ParamSliderKnob::new(cx, Data::params, |params|
                        &params.velocity_skew)
                        .set_vertical(true);
                    Binding::new(cx, Data::settings, |cx, settings | {
                        if settings.get(cx).many_velocities {
                            Label::new(cx, "skew");
                        } else {
                            // many ugly bindings because I can't directly bind to params.current_nr_of_beats.
                            Binding::new(cx, Data::text_input_a, |cx, _| {
                                Binding::new(cx, Data::text_input_b, |cx, _| {
                                    ParamBinding::new(cx, Data::params, |params| &params.interpolate_a_b,
                                                      |cx, _| {
                                                          let nr_beats = Data::params.get(cx).current_nr_of_beats.load(SeqCst) as f32;
                                                          ParamLabel::new(cx, Data::params, |params| &params.velocity_skew,
                                                                          move |skew| {
                                                                              ((skew * nr_beats).round() as usize).to_string()
                                                                          })
                                                              .alignment(Alignment::Center);
                                    });
                                });
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
        VStack::new(cx, |cx| {
            Element::new(cx)
                .height(Pixels(25.0));
            Label::new(cx, "MetreFiddler")
                .font_family(vec![FamilyOwned::Named(String::from(NOTO_SANS))])
                .font_weight(FontWeightKeyword::Thin)
                .font_size(40.0)
                .height(Pixels(50.0));

            duration_position(cx);

            Element::new(cx)
                .height(Pixels(10.0));
        })
            .alignment(Alignment::Center)
            .width(Stretch(2.0));

        // The Threshold Sliders
        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Binding::new(cx, Data::max_threshold, |cx, max| {
                    let max_val = max.get(cx);

                    Element::new(cx)
                        .width(Pixels(10.0));
                    // min thresh
                    VStack::new(cx, |cx| {
                        ParamSliderV::new(cx, Data::params, |params|
                            &params.lower_threshold)
                            .set_style(Scaled {factor: max_val});
                        Label::new(cx, "min");
                    })
                        .padding_top(Pixels(20.0))
                        .alignment(Alignment::Center);
                    // max thresh
                    VStack::new(cx, |cx| {
                        ParamSliderV::new(cx, Data::params, |params|
                            &params.upper_threshold)
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

fn duration_position(cx: &mut Context) {
    VStack::new(cx, |cx| {

        // Duration
        ParamBinding::new(
            cx,
            Data::params,
            |params| &params.use_position,
            |cx, use_pos| {

                ZStack::new(cx, |cx| {
                    // Label that changes according to Parameter
                    VStack::new(cx, |cx| {
                        ParamLabel::new(
                            cx,
                            Data::params,
                            |params| &params.use_bpm,
                            |param| {
                                if param < 0.5 {
                                    String::from("Duration in Seconds")
                                } else {
                                    String::from("Duration in Quarter Notes")
                                }
                            },
                        )
                            .alignment(Alignment::BottomCenter)
                            .font_weight(FontWeightKeyword::Bold);

                        ParamSlider::new(cx, Data::params, |params|
                            &params.metric_dur_selector)
                            .width(Pixels(200.0));

                        HStack::new(cx, |cx| {
                            // BPM Toggle
                            ParamButton::new(cx, Data::params, |params|
                                &params.use_bpm)
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
        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                // Switch between Duration and Position
                ParamButton::new(cx, Data::params, |params|
                    &params.use_position)
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
            
            ZStack::new(cx, |cx| {
                // The ticks on the position bar
                VStack::new(cx, |cx| {
                    Binding::new(cx, Data::settings,|cx, settings| {
                        ParamBinding::new(
                            cx,
                            Data::params,
                            |params| &params.interpolate_a_b,
                            move |cx, interpolate| {
                                ParamTicks::new(
                                    cx,
                                    200.0,
                                    Data::interpolation_data_snapshot,
                                    interpolate,
                                    settings.get(cx).interpolate_durations)
                                    .height(Pixels(20.0));
                            }).alignment(Alignment::Center);
                    });
                })
                    .alignment(Alignment::Center);
                
                VStack::new(cx, |cx| {
                    ParamBinding::new(
                        cx,
                        Data::params, 
                        |params| &params.use_position,
                        |cx, use_pos| {
                        let display_pos = use_pos < 0.5;

                        if display_pos {
                            ParamDisplayKnob::new(
                                cx,
                                Data::displayed_position
                                    .map(|position| position.load(SeqCst)))
                                .height(Pixels(20.0))
                                .width(Pixels(200.0));
                        } else {
                            ParamSliderKnob::new(cx, Data::params, |params|
                                &params.bar_position)
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

// Lower Part of the Plugin, containing the Metre Definition
fn lower_part(cx: &mut Context) {
    // The entire lower part
    VStack::new(cx, |cx| {

        // First Row: Textfield, info and feedback:
        HStack::new(cx, |cx| {
            // Info Button
            VStack::new(cx, |cx| {
                Button::new(cx,
                            |cx| Label::new(cx, "info"))
                    .on_press(|cx| {
                        match Data::screen.get(cx) {
                            MetreFiddlerScreen::Info => cx.emit(SetScreen(MetreFiddlerScreen::Main)),
                            _ => cx.emit(SetScreen(MetreFiddlerScreen::Info)),
                        }
                    })
                    .position_type(PositionType::Absolute)
                    .right(Pixels(10.0));
            });

            // Metre Input for A or B
            VStack::new(cx, |cx| {
                Binding::new(cx, Data::display_b, |cx, display| {
                    Binding::new(cx, Data::textbox_expanded,  move |cx, expanded| {
                        if expanded.get(cx) {
                            Popup::new(cx, |cx| {
                                if display.get(cx) {
                                    MetreInput::new(cx, Data::text_input_b, MetreB);
                                } else {
                                    MetreInput::new(cx, Data::text_input_a, MetreA);
                                }
                            })
                                .lock_focus_to_within() // automatically move into popup textbox
                                .placement(Placement::Over)
                                .background_color(Color::yellowgreen())
                                .height(Pixels(75.0)); // TODO adjust size or add scrollable view in future?
                        } else if display.get(cx) {
                            MetreInput::new(cx, Data::text_input_b, MetreB);
                        } else {
                                MetreInput::new(cx, Data::text_input_a, MetreA);
                        }
                    });
                });
            })
                .width(Stretch(3.0));

            // is valid
            VStack::new(cx, |cx| {
                Binding::new(cx, Data::display_metre_validity, |cx, display| {
                    if display.get(cx) {
                        Binding::new(cx, Data::last_input_is_valid, |cx, is_valid|{
                            let is_valid = is_valid.get(cx);
                            Label::new(cx, if is_valid { "✔️" } else { "❌" })
                                .position_type(PositionType::Absolute)
                                .top(Pixels(5.0))
                                .left(Pixels(10.0));
                        });
                    }
                })
            });
        })
            .height(Pixels(32.0));

        // Second Row: Send Midi, Interpolation, Settings
        HStack::new(cx, |cx| {
            // Extra HStack with height 50p for alignment
            HStack::new(cx, |cx| {
                // Send Midi Events?
                VStack::new(cx, |cx| {
                    ParamButton::new(cx, Data::params, |params| &params.send_midi)
                        .alignment(Alignment::Center)
                        .with_label("Send Midi")
                        .class("red_button")
                        .width(Pixels(80.0));
                })
                    .alignment(Alignment::Center);

                // Switching A & B
                HStack::new(cx, |cx| {
                    // Switch between A and B
                    Binding::new(cx, Data::display_b, |cx, display| {
                        Button::new(cx,
                                    |cx|
                                        if display.get(cx) {
                                            Label::new(cx, "Switch to A")
                                        } else {
                                            Label::new(cx, "Switch to B")
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

                        ParamSliderKnob::new(cx, Data::params, |params|
                            &params.interpolate_a_b)
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
                        Svg::new(cx, ICON_SETTINGS).width(Stretch(1.0)).height(Stretch(1.0));
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

fn settings_window(cx: &mut Context) {
    // Settings
    ScrollView::new(cx, |cx| {
        Binding::new(cx, Data::settings, |cx, settings| {
            Button::new(cx, |cx|
                if settings.get(cx).interpolate_durations {
                    Label::new(cx, "Interpolate Durations")
                } else {
                    Label::new(cx, "Don't Interpolate Durations")
                })
                .on_press(|cx| {cx.emit(ToggleInterpolateDurs)});
            Button::new(cx, |cx|
                if settings.get(cx).interpolate_indisp {
                    Label::new(cx, "Interpolate Indispensability Values")
                } else {
                    Label::new(cx, "Don't Interpolate Indispensability Values")
                })
                .on_press(|cx| {cx.emit(ToggleInterpolateIndisp)});
            Button::new(cx, |cx|
                if settings.get(cx).many_velocities {
                    Label::new(cx, "Many Velocities")
                } else {
                    Label::new(cx, "Not Many Velocities")
                })
                .on_press(|cx| {cx.emit(ToggleManyVelocities)});
            Button::new(cx, |cx|
                if settings.get(cx).midi_out_one_note {
                    Label::new(cx, "Output just one Note")
                } else {
                    Label::new(cx, "Output many notes")
                })
                .on_press(|cx| {cx.emit(ToggleMidiOutput)});
        })
    })
        .width(Stretch(1.0))
        .height(Stretch(1.0));

    HStack::new(cx, |cx| {
        ZStack::new(cx, |cx| {
            Svg::new(cx, ICON_SETTINGS).width(Stretch(1.0)).height(Stretch(1.0)).cursor(CursorIcon::Hand);
        })
            .hoverable(true)
            .on_press(|cx|cx.emit(SetScreen(MetreFiddlerScreen::Main)))
            .width(Pixels(24.0))
            .height(Pixels(24.0));
        Element::new(cx)
            .width(Pixels(24.0));
    });
}