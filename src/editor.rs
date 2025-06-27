use nih_plug::prelude::{Editor};
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering::SeqCst;
use atomic_float::AtomicF32;
use nih_plug::nih_log;
use crate::{MetreFiddlerParams};
use crate::editor::MetreFiddlerEvent::RevertPhaseReset;
use crate::gui::param_display_knob::ParamDisplayKnob;
use crate::gui::param_slider_vertical::{ParamSliderV, ParamSliderVExt};
use crate::gui::param_slider_vertical::ParamSliderStyle::{Scaled};
use crate::gui::param_label::{ParamLabel, };
use crate::gui::param_slider_knob::{ParamSliderKnob, ParamSliderKnobExt};
use crate::gui::param_ticks::ParamTicks;
use crate::metre_data::parse_input;

// TODO Click+Alt does not seem to work properly with vizia-plug? it just sometimes detects alt and
//  sometimes it doesn't. (on linux reaper, fl studio windows is perfect?)

// TODO is there a way to clean up the Data struct?

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
    pub(crate) text_input: String,
    pub(crate) last_input_is_valid: bool,
    pub(crate) max_threshold: usize,
    pub(crate) display_metre_info: bool,
    pub(crate) use_pos: bool,
    pub(crate) displayed_position: Arc<AtomicF32>,
    pub(crate) check_for_phase_reset_toggle: bool,
    pub(crate) durations: Arc<Mutex<Vec<f32>>>,
}

#[derive(Debug, Clone)]
pub enum MetreFiddlerEvent {
    UpdateString(String),
    ToggleMetreInfo,
    ToggleDurationDisplay,
    TriggerPhaseReset,
    RevertPhaseReset,
    ToggleCheckForPhaseReset,
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|my_event, _meta| match my_event {
            MetreFiddlerEvent::UpdateString(new_text) => {
                let mut metre_data = self.params.metre_data.lock().unwrap();
                if self.text_input != *new_text {
                    // update Data
                    self.text_input = new_text.clone();
                    // parse String and send to Plugin
                    match parse_input(new_text) {
                        Ok(new_metre_data) => {
                            self.durations = Arc::new(Mutex::new(new_metre_data.durations.clone()));
                            *metre_data = new_metre_data;
                            self.max_threshold = metre_data.max;
                            self.last_input_is_valid = true;
                        },
                        Err(err_string) => {
                            nih_log!("Failed to parse string: '{}': {}", self.text_input, err_string);
                            self.last_input_is_valid = false;
                        },
                    }
                }
            }
            MetreFiddlerEvent::ToggleMetreInfo => {
                self.display_metre_info = !self.display_metre_info;
            }
            MetreFiddlerEvent::ToggleDurationDisplay => {
                self.use_pos = !self.use_pos;
            }
            MetreFiddlerEvent::TriggerPhaseReset => {
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
            MetreFiddlerEvent::ToggleCheckForPhaseReset => {
                if self.params.reset_info.load(SeqCst) == false {
                    cx.emit(RevertPhaseReset);
                } else {
                    self.check_for_phase_reset_toggle = !self.check_for_phase_reset_toggle; 
                }
            }
        });
    }
}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (500, 300))
}

pub(crate) fn create(
    params: Arc<MetreFiddlerParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        // add new styling
        let _ = cx.add_stylesheet(NEW_STYLE);

        let metre_data = params.metre_data.lock().unwrap();
        
        Data {
            params: params.clone(),
            text_input: metre_data.input.clone(),
            last_input_is_valid: true,
            max_threshold: metre_data.max.clone(),
            display_metre_info: false,
            use_pos: params.use_position.value(),
            displayed_position: params.displayed_position.clone(),
            check_for_phase_reset_toggle: false,
            durations: Arc::new(Mutex::new(metre_data.durations.clone())),
        }
            .build(cx);

        // This is a kinda hacky way to get the button and BoolParm to reset itself, but keeping
        // DAW Automation possible...
        Binding::new(cx, Data::check_for_phase_reset_toggle, |cx, _was_reset| {
            cx.emit(MetreFiddlerEvent::ToggleCheckForPhaseReset);
        });
        
        VStack::new(cx, |cx| {
            ZStack::new(cx, |cx| {
                // The upper part of the Plugin
                upper_part(cx);
                
                // Information text displayed over plugin
                Binding::new(cx, Data::display_metre_info, |cx, display| {
                    if display.get(cx) {
                        Element::new(cx)
                            .background_color(RGBA::rgba(250, 250, 250, 255))
                            .opacity(1.0);
                        Label::new(cx, "")
                            .text(PLUGIN_INFO_TEXT)
                            .top(Pixels(5.0))
                            // better too small than clipping
                            .font_size(12.0);
                    }
                })
            })
                .height(Stretch(5.0));
            
            // Lower Part of the Plugin
            lower_part(cx);
        })
            // I have no clue, why I have to hardcode this? But without this, the HStacks are
            // not stretched properly
          .width(Pixels(500.0));

        //  ResizeHandle::new(cx);
    })
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
                    Label::new(cx, "skew");
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
            
            Label::new(cx, "Threshold")
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
                            cx.emit(MetreFiddlerEvent::TriggerPhaseReset);
                        })
                        .width(Pixels(100.0));
                })
                    .alignment(Alignment::Center)
                    .top(Pixels(10.0));
            })
                .alignment(Alignment::TopCenter);

            Binding::new(cx, Data::use_pos, |cx, use_pos| {
                if use_pos.get(cx) {
                    Element::new(cx)
                        .background_color(RGBA::rgba(250, 250, 250, 255))
                        .opacity(1.0);
                }
            });
        })
            .height(Stretch(0.4))
            .alignment(Alignment::Center);

        // Position
        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                // Switch between Duration and Position
                ParamButton::new(cx, Data::params, |params|
                    &params.use_position)
                    .on_press(|cx| {
                        cx.emit(MetreFiddlerEvent::ToggleDurationDisplay)
                    })
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
                VStack::new(cx, |cx| {
                    Binding::new(cx, Data::durations, |cx, durs| {
                        ParamTicks::new(
                            cx,
                            durs
                                .map(|durations| durations.lock().unwrap().clone()))
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));                        
                    });
                })
                    .alignment(Alignment::Center);
                
                VStack::new(cx, |cx| {
                    Binding::new(cx, Data::use_pos, |cx, use_pos| {
                        let display_pos = !use_pos.get(cx);

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
                    });
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
    HStack::new(cx, |cx| {
        // Info Button
        VStack::new(cx, |cx| {
            Button::new(cx,
                        |cx| Label::new(cx, "info"))
                .on_press(|cx| {
                    cx.emit(MetreFiddlerEvent::ToggleMetreInfo)
                })
                .position_type(PositionType::Absolute)
                .right(Pixels(10.0));
        });
       
        // Metre Input
        Textbox::new(cx, Data::text_input)
            .on_submit(|cx, text, _| {
                cx.emit(MetreFiddlerEvent::UpdateString(text));
            })
            .width(Stretch(3.0));
        
        // is valid
        VStack::new(cx, |cx| {
            Binding::new(cx, Data::last_input_is_valid, |cx, is_valid|{
                let is_valid = is_valid.get(cx);
                Label::new(cx, if is_valid { "✔️" } else { "❌" })
                    .position_type(PositionType::Absolute)
                    .top(Pixels(5.0))
                    .left(Pixels(10.0));
            }); 
        });
    });
}