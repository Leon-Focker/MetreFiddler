use nih_plug::prelude::{Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use nih_plug::nih_log;
use crate::{MetreFiddlerParams};
use crate::editor::MetreFiddlerEvent::RevertPhaseReset;
use crate::gui::param_slider_vertical::{ParamSliderExt, ParamSliderV};
use crate::gui::param_slider_vertical::ParamSliderStyle::{Scaled};
use crate::gui::param_label::{ParamLabel, };
use crate::metre_data::parse_input;

const PLUGIN_INFO_TEXT: &str = "
     Below you can define a metric structure using RQQ notation, i.e. hierarchical 
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


#[derive(Lens)]
struct Data {
    params: Arc<MetreFiddlerParams>,
    text_input: String,
    last_input_is_valid: bool,
    max_threshold: usize,
    display_metre_info: bool,
    check_for_phase_reset_toggle: bool,
}

#[derive(Debug, Clone)]
pub enum MetreFiddlerEvent {
    UpdateString(String),
    ToggleMetreInfo,
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
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        let metre_data = params.metre_data.lock().unwrap();

        Data {
            params: params.clone(),
            text_input: metre_data.input.clone(),
            last_input_is_valid: true,
            max_threshold: metre_data.max.clone(),
            display_metre_info: false,
            check_for_phase_reset_toggle: false,
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
                
                //Information text displayed over plugin
                Binding::new(cx, Data::display_metre_info, |cx, display| {
                    if display.get(cx) {
                        Element::new(cx)
                            .text(PLUGIN_INFO_TEXT)
                            .font_size(13.0)
                            .background_color(RGBA::rgba(250, 250, 250, 255))
                            .opacity(1.0);
                    }
                })                
            })
                .height(Stretch(5.0));
            
            // Lower Part of the Plugin
            lower_part(cx);
        })
            .row_between(Pixels(0.0)) ;// Space between elements in column

        ResizeHandle::new(cx);
    })
}

// Upper Part of the Plugin
fn upper_part(cx: &mut Context) {
    HStack::new(cx, |cx| {
        // The Velocity Sliders
        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                // min vel
                VStack::new(cx, |cx| {
                    ParamSliderV::new(cx, Data::params, |params|
                        &params.velocity_min)
                        .set_style(Scaled {factor: 1});
                    Label::new(cx, "min");
                });
                // max vel
                VStack::new(cx, |cx| {
                    ParamSliderV::new(cx, Data::params, |params|
                        &params.velocity_max)
                        .set_style(Scaled {factor: 1});
                    Label::new(cx, "max");
                })
                    .left(Pixels(15.0));
            })
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(0.1));

            Label::new(cx, "Velocity")
                .font_weight(FontWeightKeyword::Bold)
                .left(Stretch(1.0))
                .right(Stretch(1.0))
                .child_bottom(Pixels(10.0));
        })
            .width(Stretch(1.0));

        // Middle Part (Name, Duration, Buttons)
        VStack::new(cx, |cx| {
            Label::new(cx, "MetreFiddler")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Thin)
                .font_size(40.0)
                .height(Pixels(50.0))
                .child_bottom(Pixels(0.0))
                .top(Stretch(0.1));

            // Label that changes according to Parameter
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
                .font_weight(FontWeightKeyword::Bold)
                .top(Stretch(1.5));
            ParamSlider::new(cx, Data::params, |params|
                &params.metric_dur_selector)
                .width(Pixels(200.0))
                .bottom(Pixels(0.0));

            HStack::new(cx, |cx| {
                // BPM Toggle
                ParamButton::new(cx, Data::params, |params|
                    &params.use_bpm)
                    .with_label("  Use BPM")
                    .width(Pixels(100.0));
                // Reset Phase
                Button::new(
                    cx,
                    |cx| {
                        cx.emit(MetreFiddlerEvent::TriggerPhaseReset);
                    },
                    |cx| Label::new(cx, "reset phase"))
                    .width(Pixels(100.0));
            })
                .top(Pixels(10.0))
                .child_space(Stretch(1.0));
        })
            .top(Stretch(0.2))
            .width(Stretch(2.0))
            .child_space(Stretch(1.0));

        // The Threshold Sliders
        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Binding::new(cx, Data::max_threshold, |cx, max| {
                    let max_val = max.get(cx);

                    VStack::new(cx, |cx| {
                        ParamSliderV::new(cx, Data::params, |params|
                            &params.lower_threshold)
                            .set_style(Scaled {factor: max_val});
                        Label::new(cx, "min");
                    });

                    VStack::new(cx, |cx| {
                        ParamSliderV::new(cx, Data::params, |params|
                            &params.upper_threshold)
                            .set_style(Scaled { factor: max_val });
                        Label::new(cx, "max");
                    })
                        .left(Pixels(15.0));
                }); 
            })
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0))
                .child_top(Stretch(0.1));
            
            Label::new(cx, "Threshold")
                .font_weight(FontWeightKeyword::Bold)
                .left(Stretch(1.0))
                .right(Stretch(1.0))
                .child_bottom(Pixels(10.0));
        })
            .width(Stretch(1.0));
    })
        .child_left(Pixels(0.0))
        .child_right(Pixels(0.0));
}

// Lower Part of the Plugin, containing the Metre Definition
fn lower_part(cx: &mut Context) {
    HStack::new(cx, |cx| {
        // Info Button
        VStack::new(cx, |cx| {
            Button::new(cx,
                        |cx| { cx.emit(MetreFiddlerEvent::ToggleMetreInfo); },
                        |cx| Label::new(cx, "info"))
                .right(Pixels(10.0));     
        })
            .top(Pixels(0.0))
            .bottom(Pixels(0.0))
            .left(Pixels(0.0))
            .right(Pixels(0.0))
            .child_space(Stretch(1.0));
       
        // Metre Input
        // TODO Make this work in FL studio
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
                    .left(Pixels(10.0));
            }); 
        })
            .top(Pixels(0.0))
            .bottom(Pixels(0.0))
            .left(Pixels(0.0))
            .right(Pixels(0.0))
            .child_space(Stretch(1.0));
    })
        //.height(Stretch(1.0))
        .child_space(Stretch(1.0));
}