use nice_plug::prelude::{util, Editor};
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};
use vizia_plug::vizia::icons::ICON_SETTINGS;
use std::sync::{Arc};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use atomic_float::AtomicF32;
use nice_plug::{nice_log};
use crate::{MetreFiddlerParams};
use crate::editor::MetreFiddlerEvent::*;
use crate::gui::param_label::ParamLabel;
use crate::gui::param_slider_knob::ParamSliderKnob;
use crate::gui::param_slider_vertical::{ParamSliderV, ParamSliderVExt};
use crate::gui::param_slider_vertical::ParamSliderStyle::Scaled;
use crate::gui::settings_button::{SettingsButton, SettingsButtonModifiers};
use crate::gui::metre_input::{MetreAorB, MetreInput};
use crate::gui::metre_input::MetreAorB::{MetreA, MetreB};
// use crate::gui::param_binding::ParamBinding;
// use crate::gui::param_display_knob::ParamDisplayKnob;
// use crate::gui::param_slider_vertical::ParamSliderStyle::{Scaled};
// use crate::gui::param_label::{ParamLabel};
// use crate::gui::param_slider_knob::{ParamSliderKnob, ParamSliderKnobExt};
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
    pub(crate) last_input_is_valid: Signal<String>,
    pub(crate) display_which_metre: Signal<MetreAorB>,
    pub(crate) display_validity: Signal<bool>,
    pub(crate) textbox_expanded: Signal<bool>,
    pub(crate) text_input_a: Signal<String>,
    pub(crate) text_input_b: Signal<String>,
    // pub(crate) interpolation_data_snapshot: Signal<InterpolationData>,
    // pub(crate) textbox_expanded: bool,
    // pub(crate) text_input_a: String,
    // pub(crate) text_input_b: String,
    // pub(crate) max_threshold: Signal<usize>,
    // pub(crate) displayed_position: Signal<f32>,
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
    UpdateString(String, MetreAorB),
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
    ShowValidity(bool),
    ExpandTextBox(bool),
}

impl Model for AppData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|my_event, _meta| match my_event {
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
            _ => ()
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
        let interpolate_durations = SyncSignal::from(params.interpolate_durations.load(Relaxed));
        let many_velocities = SyncSignal::from(params.many_velocities.load(Relaxed));
        let midi_out_one_note = SyncSignal::from(params.midi_out_one_note.load(Relaxed));
        let interpolate_indisp = SyncSignal::from(params.interpolate_indisp.load(Relaxed));
        let retain_metric_phase= SyncSignal::from(params.retain_metric_phase.load(Relaxed));
        let last_input_is_valid = Signal::new("✔️".to_string());
        let display_which_metre = Signal::from(MetreA);
        let display_validity = Signal::from(true);
        let text_input_a = Signal::from(metric_data.metre_a().string.clone());
        let text_input_b = Signal::from(metric_data.metre_b().string.clone());
        let textbox_expanded = Signal::from(false);

        // interpolation_data_snapshot: metric_data.interpolation_data().clone(),
        // max_threshold: metric_data.metre_a().max.max(metric_data.metre_b().max),
        // displayed_position: params.displayed_position.clone(),
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
            // max_threshold: metric_data.metre_a().max.max(metric_data.metre_b().max),
            // displayed_position: Signal::from(params.displayed_position.load(Relaxed)),
            // check_for_phase_reset_toggle: false,
            last_input_is_valid,
            display_which_metre,
            display_validity,
            textbox_expanded,
            text_input_a,
            text_input_b,
        }
            .build(cx);

        // This is a kinda hacky way to get the button and BoolParm to reset itself, but keeping
        // DAW Automation possible...
        // Binding::new(cx, Data::check_for_phase_reset_toggle, |cx, _was_reset| {
        //     cx.emit(ToggleCheckForPhaseReset);
        // });

        let binding_params = params.clone();

       Binding::new(cx, screen, move |cx| {
           let binding_params = binding_params.clone();
           match screen.get() {
               MetreFiddlerScreen::Settings => {
                   settings_window(cx);
               },
               MetreFiddlerScreen::Main => {
                   // Upper Part of the Plugin
                   VStack::new(cx,  move |cx| {
                       upper_part(cx);
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
            Svg::new(cx, ICON_SETTINGS).width(Stretch(1.0)).height(Stretch(1.0)).cursor(CursorIcon::Hand).background_color(Color::black());
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
fn upper_part(cx: &mut Context) {
    // HStack::new(cx, |cx| {
    //     // The Velocity Sliders
    //     VStack::new(cx, |cx| {
    //         HStack::new(cx, |cx| {
    //             Element::new(cx)
    //                 .width(Pixels(10.0));
    //             // min vel
    //             VStack::new(cx, |cx| {
    //                 ParamSliderV::new(cx, &params.velocity_min)
    //                     .set_style(Scaled {factor: 1});
    //                 Label::new(cx, "min");
    //             })
    //                 .padding_top(Pixels(20.0))
    //                 .alignment(Alignment::Center);
    //             // max vel
    //             VStack::new(cx, |cx| {
    //                 ParamSliderV::new(cx, &params.velocity_max)
    //                     .set_style(Scaled {factor: 1});
    //                 Label::new(cx, "max");
    //             })
    //                 .padding_top(Pixels(20.0))
    //                 .alignment(Alignment::Center);
    //             // Skew
    //             VStack::new(cx, |cx| {
    //                 ParamSliderKnob::new(cx, &params.velocity_skew)
    //                     .set_vertical(true);
    //                 Binding::new(cx, Data::settings, |cx, settings | {
    //                     if settings.get(cx).many_velocities {
    //                         Label::new(cx, "skew");
    //                     } else {
    //                         // many ugly bindings because I can't directly bind to params.current_nr_of_beats.
    //                         Binding::new(cx, Data::text_input_a, |cx, _| {
    //                             Binding::new(cx, Data::text_input_b, |cx, _| {
    //                                 ParamBinding::new(cx, Data::params, |params| &params.interpolate_a_b,
    //                                                   |cx, _| {
    //                                                       let nr_beats = &params.get(cx).current_nr_of_beats.load(Acquire) as f32;
    //                                                       ParamLabel::new(cx, &params.velocity_skew, move |skew| {
    //                                                           ((skew * nr_beats).round() as usize).to_string()
    //                                                       })
    //                                                           .alignment(Alignment::Center);
    //                                                   });
    //                             });
    //                         });
    //                     }
    //                 });
    //             })
    //                 .padding_top(Pixels(20.0))
    //                 .alignment(Alignment::Center);
    //
    //             Element::new(cx)
    //                 .width(Pixels(10.0));
    //         });
    //
    //         Label::new(cx, "Velocity")
    //             .font_weight(FontWeightKeyword::Bold)
    //             .padding_bottom(Pixels(20.0));
    //     })
    //         .alignment(Alignment::Center)
    //         .width(Stretch(1.0));
    //
    //     // Middle Part (Name, Duration, Buttons)
    //     VStack::new(cx, |cx| {
    //         Element::new(cx)
    //             .height(Pixels(25.0));
    //         Label::new(cx, "MetreFiddler")
    //             .font_family(vec![FamilyOwned::Named(String::from(NOTO_SANS))])
    //             .font_weight(FontWeightKeyword::Thin)
    //             .font_size(40.0)
    //             .height(Pixels(50.0));
    //
    //         duration_position(cx);
    //
    //         Element::new(cx)
    //             .height(Pixels(10.0));
    //     })
    //         .alignment(Alignment::Center)
    //         .width(Stretch(2.0));
    //
    //     // The Threshold Sliders
    //     VStack::new(cx, |cx| {
    //         HStack::new(cx, |cx| {
    //             Binding::new(cx, Data::max_threshold, |cx, max| {
    //                 let max_val = max.get(cx);
    //
    //                 Element::new(cx)
    //                     .width(Pixels(10.0));
    //                 // min thresh
    //                 VStack::new(cx, |cx| {
    //                     ParamSliderV::new(cx, Data::params, |params|
    //                         &params.lower_threshold)
    //                         .set_style(Scaled {factor: max_val});
    //                     Label::new(cx, "min");
    //                 })
    //                     .padding_top(Pixels(20.0))
    //                     .alignment(Alignment::Center);
    //                 // max thresh
    //                 VStack::new(cx, |cx| {
    //                     ParamSliderV::new(cx, Data::params, |params|
    //                         &params.upper_threshold)
    //                         .set_style(Scaled { factor: max_val });
    //                     Label::new(cx, "max");
    //                 })
    //                     .padding_top(Pixels(20.0))
    //                     .alignment(Alignment::Center);
    //                 Element::new(cx)
    //                     .width(Pixels(10.0));
    //             });
    //         });
    //
    //         Label::new(cx, "Thresholds")
    //             .font_weight(FontWeightKeyword::Bold)
    //             .padding_bottom(Pixels(20.0));
    //     })
    //         .alignment(Alignment::Center)
    //         .width(Stretch(1.0));
    // });
}

// Lower Part of the Plugin, containing the Metre Definition
fn lower_part(cx: &mut Context,
              text_input_a: Signal<String>,
              text_input_b: Signal<String>,
              screen: Signal<MetreFiddlerScreen>,
              params: Arc<MetreFiddlerParams>,
              display_which_metre: Signal<MetreAorB>,
              display_validity: Signal<bool>,
              is_valid: Signal<String>,
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
                                    MetreB =>  MetreInput::new(cx, text_input_b, MetreB),
                                };
                            })
                                .lock_focus_to_within() // automatically move into popup textbox
                                .placement(Placement::Over)
                                .background_color(Color::yellowgreen())
                                .height(Pixels(75.0)); // TODO adjust size or add scrollable view in future?
                        } else {
                            match display_which_metre.get() {
                                MetreA =>  MetreInput::new(cx, text_input_a, MetreA),
                                MetreB =>  MetreInput::new(cx, text_input_b, MetreB),
                            };
                        }
                    });
                });
            })
                .width(Stretch(3.0));

            // is valid
            VStack::new(cx, move |cx| {
                Binding::new(cx, display_validity, move |cx| {
                    if display_validity.get() {
                        Label::new(cx, is_valid)
                            .position_type(PositionType::Absolute)
                            .top(Pixels(5.0))
                            .left(Pixels(10.0));
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
                                            MetreB => Label::new(cx, "Switch to A"),
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
