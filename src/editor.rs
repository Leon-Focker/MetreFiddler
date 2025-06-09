use nih_plug::prelude::{Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;

use crate::{MetreFiddlerParams};

#[derive(Lens)]
struct Data {
    params: Arc<MetreFiddlerParams>,
}

impl Model for Data {}

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

            Label::new(cx, "BPM Toggle")
                .top(Pixels(10.0));
            ParamSlider::new(cx, Data::params, |params| &params.bpm_toggle);

            Label::new(cx, "Duration")
                .top(Pixels(10.0));
            ParamSlider::new(cx, Data::params, |params| &params.outer_dur_selector);
        })
            .row_between(Pixels(0.0)) // Space between elements in column
            .child_left(Stretch(1.0))
            .child_right(Stretch(1.0));

        ResizeHandle::new(cx);
    })
}