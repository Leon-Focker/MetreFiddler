use nih_plug::prelude::{Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;

use crate::{BitFiddlerParams};

#[derive(Lens)]
struct Data {
    params: Arc<BitFiddlerParams>,
}

impl Model for Data {}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (250, 150))
}

pub(crate) fn create(
    params: Arc<BitFiddlerParams>,
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
            Label::new(cx, "BitFiddler")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Thin)
                .font_size(40.0)
                .height(Pixels(50.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0))
                .top(Pixels(10.0));

            Label::new(cx, "Bit Index")
                .top(Pixels(10.0));
            ParamSlider::new(cx, Data::params, |params| &params.bit_selector);
        })
            .row_between(Pixels(0.0)) // Space between elements in column
            .child_left(Stretch(1.0))
            .child_right(Stretch(1.0));

        ResizeHandle::new(cx);
    })
}

/*fn view(&mut self) -> Element<'_, Self::Message> {
    Column::new()
        .width(Length::FillPortion(1)) // Make this column take 1 part of the row
        .align_items(Alignment::Center)
        .push(
            Text::new("BitFiddler")
                .font(assets::NOTO_SANS_LIGHT)
                .size(40)
                .height(50.into())
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center)
                .vertical_alignment(alignment::Vertical::Bottom),
        )
        .push(Space::with_height(10.into()))
        .push(
            Text::new("Bit Index")
                .height(20.into())
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center)
                .vertical_alignment(alignment::Vertical::Center),
        )
        .push(
            ParamSlider::new(&mut self.bit_selector_slider_state, &self.params.bit_selector)
                .map(Message::ParamUpdate),
        )
        .into()
}

fn background_color(&self) -> nih_plug_iced::Color {
    nih_plug_iced::Color {
        r: 0.98,
        g: 0.98,
        b: 0.98,
        a: 1.0,
    }
}*/