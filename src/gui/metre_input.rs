use nih_plug::nih_log;
use vizia_plug::vizia::prelude::*;
use crate::editor::MetreFiddlerEvent;

#[derive(Lens)]
pub struct MetreInput {}

#[derive(Clone, Copy, Debug)]
pub enum MetreAorB {
    MetreA,
    MetreB,
}

impl MetreInput {
    pub fn new<L>(
        cx: &mut Context,
        text_data: L,
        which: MetreAorB,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = String>,
    {
        Self {}
            .build(cx,|cx| {
                Textbox::new_multiline(cx, text_data, false)
                    .on_edit(|cx, _| {
                        cx.emit(MetreFiddlerEvent::ShowValidity(false))
                    })
                    .on_submit(move |cx, text, _| {
                        cx.emit(MetreFiddlerEvent::ExpandTextBox(false));
                        cx.emit(MetreFiddlerEvent::ShowValidity(true));
                        cx.emit(MetreFiddlerEvent::UpdateString(text, which));
                    })
                    .height(Stretch(1.0))
                    .width(Stretch(3.0));
            })
    }
}

impl View for MetreInput {
    fn element(&self) -> Option<&'static str> {
        Some("param-ticks")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _meta| match window_event {
            // Return
            WindowEvent::KeyDown(_, Some(Key::Escape)) | WindowEvent::FocusOut => {
                cx.emit(MetreFiddlerEvent::ExpandTextBox(false));
            },
            // Focus
            WindowEvent::MouseDown(MouseButton::Left) |
            WindowEvent::KeyDown(_, _) => {
                cx.emit(MetreFiddlerEvent::ExpandTextBox(true));
            }
            // TODO scrolling should work out of the box but even this doesn't work?
            WindowEvent::MouseScroll(x, y) => {
                nih_log!("scrolling!");
                cx.emit(TextEvent::Scroll(*x, *y))
            },
            _ => (),
        })
    }
}