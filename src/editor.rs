use nih_plug::prelude::{Editor, GuiContext};
use nih_plug_iced::widgets as nih_widgets;
use nih_plug_iced::*;
use std::sync::Arc;
use nih_plug_iced::widgets::ParamSlider;

use crate::{BitFiddlerParams};

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<IcedState> {
    IcedState::from_size(250, 150)
}

pub(crate) fn create(
    params: Arc<BitFiddlerParams>,
    editor_state: Arc<IcedState>,
) -> Option<Box<dyn Editor>> {
    create_iced_editor::<BitFiddlerEditor>(editor_state, params)
}

struct BitFiddlerEditor {
    params: Arc<BitFiddlerParams>,
    context: Arc<dyn GuiContext>,

    bit_selector_slider_state: nih_widgets::param_slider::State,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    /// Update a parameter's value.
    ParamUpdate(nih_widgets::ParamMessage),
}

impl IcedEditor for BitFiddlerEditor {
    type Executor = executor::Default;
    type Message = Message;
    type InitializationFlags = Arc<BitFiddlerParams>;

    fn new(
        params: Self::InitializationFlags,
        context: Arc<dyn GuiContext>,
    ) -> (Self, Command<Self::Message>) {
        let editor = BitFiddlerEditor {
            params,
            context,

            bit_selector_slider_state: Default::default(),
        };

        (editor, Command::none())
    }

    fn context(&self) -> &dyn GuiContext {
        self.context.as_ref()
    }

    fn update(
        &mut self,
        _window: &mut WindowQueue,
        message: Self::Message,
    ) -> Command<Self::Message> {
        match message {
            Message::ParamUpdate(message) => self.handle_param_message(message),
        }
        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
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
    }
}
