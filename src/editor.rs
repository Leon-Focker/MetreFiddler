use nih_plug::prelude::{Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;
use nih_plug::nih_log;
use crate::{MetreFiddlerParams};
use crate::metre_data::parse_input;


#[derive(Lens)]
struct Data {
    params: Arc<MetreFiddlerParams>,
    text_input: String,
    last_input_is_valid: bool,
}

#[derive(Debug, Clone)]
pub enum MetreFiddlerEvent {
    UpdateString(String),
}

impl Model for Data {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|my_event, _meta| match my_event {
            MetreFiddlerEvent::UpdateString(new_text) => {
                let mut metre_data = self.params.metre_data.lock().unwrap();
                if self.text_input != *new_text {
                    // update Data
                    self.text_input = new_text.clone();
                    // parse String and send to Plugin
                    match parse_input(new_text) {
                        Ok(parsed_string) => {
                            println!("I got an update!: {:?}", &parsed_string);
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
        });
    }
}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (250, 250))
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
        }
            .build(cx);

        // A Column
        VStack::new(cx, |cx| {
            Label::new(cx, "MetreFiddler")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Thin)
                .font_size(40.0)
                .height(Pixels(50.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0))
                .top(Pixels(10.0));

            ParamButton::new(cx, Data::params, |params| &params.bpm_toggle);

            Label::new(cx, "Duration")
                .top(Pixels(10.0));
            ParamSlider::new(cx, Data::params, |params| &params.metric_dur_selector);

            HStack::new(cx, |cx| {
                Textbox::new(cx, Data::text_input)
                    .on_submit(|cx, text, _| {
                        cx.emit(MetreFiddlerEvent::UpdateString(text));
                    })
                    .width(Stretch(10.0))
                    .top(Pixels(10.0));

                Binding::new(cx, Data::last_input_is_valid, |cx, is_valid|{
                    let is_valid = is_valid.get(cx);
                    Label::new(cx, if is_valid { "✔️" } else { "❌" })
                        .width(Stretch(1.0))
                        .top(Pixels(15.0));
                });
                
            });
            
        })
            .row_between(Pixels(0.0)) // Space between elements in column
            .child_left(Stretch(1.0))
            .child_right(Stretch(1.0));

        ResizeHandle::new(cx);
    })
}