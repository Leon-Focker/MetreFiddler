use vizia_plug::vizia::prelude::*;
use crate::editor::MetreFiddlerEvent;
use crate::metre::metre_slot::MetreSlot;

pub struct MetreInput {}

impl MetreInput {
    pub fn new(
        cx: &mut Context,
        text_data: Signal<String>,
        slot: MetreSlot,
    ) -> Handle<'_, Self>
    {
        Self {}
            .build(cx,|cx| {
                Textbox::new_multiline(cx, text_data, false)
                    .on_double_click(|cx, _ |{
                        cx.emit(MetreFiddlerEvent::ExpandTextBox(true));
                    })
                    .on_edit(|cx, _| {
                        cx.emit(MetreFiddlerEvent::DisplayValidity(false))
                    })
                    .on_submit(move |cx, text, _| {
                        cx.emit(MetreFiddlerEvent::ExpandTextBox(false));
                        cx.emit(MetreFiddlerEvent::DisplayValidity(true));
                        cx.emit(MetreFiddlerEvent::UpdateString(text, slot));
                    })
                    .placeholder("Input Metre Definition")
                    .background_color(RGBA::rgba(250, 250, 250, 255))
                    .border_color(RGBA::rgb(196, 196, 196))
                    .caret_color(Color::black())
                    .height(Stretch(1.0))
                    .width(Stretch(3.0));
            })
    }
}

impl View for MetreInput {
    fn element(&self) -> Option<&'static str> {
        Some("metre_input")
    }
}