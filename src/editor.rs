use nih_plug::prelude::{Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;
use nih_plug::nih_log;
use crate::{MetreFiddlerParams};
use crate::gui::param_slider_vertical::ParamSliderV;
use crate::metre_data::parse_input;

#[derive(Lens)]
struct Data {
    params: Arc<MetreFiddlerParams>,
    text_input: String,
    last_input_is_valid: bool,
    display_metre_info: bool,
    reset_time: Option<Instant>,
    reset_timer: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum MetreFiddlerEvent {
    UpdateString(String),
    ToggleMetreInfo,
    TriggerPhaseReset,
    RevertPhaseReset,
    PhaseResetCounter,
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
                        Ok(parsed_string) => {
                            metre_data.value = parsed_string;
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
                println!("hey! you clicked a button!")
            }
            MetreFiddlerEvent::TriggerPhaseReset => {
                let param_ref = &self.params.reset_phase;

                cx.emit(ParamEvent::BeginSetParameter(param_ref).upcast());
                cx.emit(ParamEvent::SetParameter(param_ref, true).upcast());
                cx.emit(ParamEvent::EndSetParameter(param_ref).upcast());

                self.reset_time = Some(Instant::now());
                self.reset_timer = Some(Duration::from_millis(0));
            }
            MetreFiddlerEvent::RevertPhaseReset => {
                let param_ref = &self.params.reset_phase;

                cx.emit(ParamEvent::BeginSetParameter(param_ref).upcast());
                cx.emit(ParamEvent::SetParameter(param_ref, false).upcast());
                cx.emit(ParamEvent::EndSetParameter(param_ref).upcast());

                self.reset_time = None;
                self.reset_timer = None;
            }
            MetreFiddlerEvent::PhaseResetCounter => {
                self.reset_timer = Some(Instant::now().duration_since(self.reset_time.unwrap()));
            }
        });
    }
}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (500, 500))
}

pub(crate) fn create(
    params: Arc<MetreFiddlerParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data {
            params: params.clone(),
            text_input: params.metre_data.lock().unwrap().input.clone(),
            last_input_is_valid: true,
            display_metre_info: false,
            reset_time: None,
            reset_timer: None,
        }
            .build(cx);

        // This is a kinda hacky way to get the button and BoolParm to reset itself, but keeping
        // DAW Automation possible...
        Binding::new(cx, Data::reset_timer, |cx, timer| {
            if let Some(time) = timer.get(cx) {
                if time >= Duration::from_millis(100) {
                    cx.emit(MetreFiddlerEvent::RevertPhaseReset);
                } else {
                    cx.emit(MetreFiddlerEvent::PhaseResetCounter);
                }
            }
        });

        // Overlays its elements
        ZStack::new(cx, |cx| {
            // A Column
            VStack::new(cx, |cx| {
                
                // Upper Part of the Plugin
                HStack::new(cx, |cx| {
                    // min vel
                    ParamSliderV::new(cx, Data::params, |params|
                        &params.velocity_min);
                        // .width(Pixels(50.0)).
                        // height(Pixels(200.0));
                    // max vel
                    ParamSliderV::new(cx, Data::params, |params|
                        &params.velocity_max);
                        // .width(Pixels(50.0)).
                        // height(Pixels(200.0));
                        
                    // Middle Part (Name, Duration, Buttons)
                    // VStack::new(cx, |cx| {
                    //     Label::new(cx, "MetreFiddler")
                    //         .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                    //         .font_weight(FontWeightKeyword::Thin)
                    //         .font_size(40.0)
                    //         .height(Pixels(50.0))
                    //         .child_top(Stretch(1.0))
                    //         .child_bottom(Pixels(0.0))
                    //         .top(Pixels(10.0));
                    //     
                    //     HStack::new(cx, |cx| {
                    //         // BPM Toggle
                    //         ParamButton::new(cx, Data::params, |params| 
                    //             &params.bpm_toggle);
                    //         // Reset Phase
                    //         Button::new(
                    //             cx,
                    //             |cx| {
                    //                 cx.emit(MetreFiddlerEvent::TriggerPhaseReset);
                    //             },
                    //             |cx| Label::new(cx, "reset phase"));
                    //     });
                    // 
                    //     Label::new(cx, "Duration")
                    //         .top(Pixels(10.0));
                    //     ParamSlider::new(cx, Data::params, |params| 
                    //         &params.metric_dur_selector);
                    // 
                    // });
                    
                    // lower threshold
                    ParamSlider::new(cx, Data::params, |params|
                        &params.lower_threshold);
                    // upper threshold
                    ParamSlider::new(cx, Data::params, |params|
                        &params.upper_threshold);
                })
                    .child_left(Pixels(0.0))
                    .child_right(Pixels(0.0));
                
                // Lower Part, containing the Metre Definition
                HStack::new(cx, |cx| {
                    // Info Button
                    Button::new(cx,
                                |cx| { cx.emit(MetreFiddlerEvent::ToggleMetreInfo); },
                                |cx| Label::new(cx, "info"));
                    // Metre Input
                    Textbox::new(cx, Data::text_input)
                        .on_submit(|cx, text, _| {
                            cx.emit(MetreFiddlerEvent::UpdateString(text));
                        })
                        .width(Stretch(1.0))
                        .top(Pixels(10.0));
                    // is valid
                    Binding::new(cx, Data::last_input_is_valid, |cx, is_valid|{
                        let is_valid = is_valid.get(cx);
                        Label::new(cx, if is_valid { "✔️" } else { "❌" })
                            .width(Stretch(1.0))
                            .top(Pixels(15.0));
                    });                        
                })
                    .background_color(Color::red());
            })
                .row_between(Pixels(0.0)) ;// Space between elements in column
               // .child_left(Stretch(1.0))
                //.child_right(Stretch(1.0))
                //.width(Stretch(1.0));

            //Information text displayed over plugin
            Binding::new(cx, Data::display_metre_info, |cx, display| {
                if display.get(cx) {
                    Element::new(cx)
                        .text("This is a lot of text")
                        .width(Pixels(100.0))
                        .height(Pixels(100.0))
                        .background_color(Color::black());
                }
            })
        })
            .width(Stretch(1.0));


        ResizeHandle::new(cx);
    })
}