use vizia_plug::vizia::prelude::*;

pub struct SettingsButton {
    pub on_press: Option<Box<dyn Fn(&mut EventContext)>>,
}

pub enum SettingsButtonEvent {
    Press
}

impl SettingsButton {
    pub fn new(cx: &mut Context, is_on: SyncSignal<bool>, label: String) -> Handle<Self> {
        Self {
            on_press: None,
        }.build(cx, |cx|{

            HStack::new(cx, move |cx| {
                Element::new(cx).width(Pixels(48.0));

                let button_text = is_on.map(|on| if *on { "On" } else { "Off" });

                Button::new(cx, |cx| Label::new(cx, button_text))
                    .class("red_button")
                    .checked(is_on)
                    .width(Pixels(50.0))
                    .on_press(|cx| cx.emit(SettingsButtonEvent::Press));

                Element::new(cx).width(Pixels(24.0));
                Label::new(cx, label);
            })
                .alignment(Alignment::Left);
        })
    }
}

impl View for SettingsButton {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|counter_event, _meta| match counter_event{
            SettingsButtonEvent::Press => {
                if let Some(callback) = &self.on_press {
                    callback(cx);
                }
            }
        });
    }
}

pub trait SettingsButtonModifiers {
    fn on_button_press<F: Fn(&mut EventContext) + 'static>(self, callback: F) -> Self;
}

impl<'a> SettingsButtonModifiers for Handle<'a, SettingsButton> {
    fn on_button_press<F: Fn(&mut EventContext) + 'static>(self, callback: F) -> Self {
        self.modify(|counter| counter.on_press = Some(Box::new(callback)))
    }
}