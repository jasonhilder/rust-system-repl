use iced::widget::{
    button,
    column,
    text,
    text_input, row
};
use iced::{
    Alignment,
    Element,
    Sandbox,
    Settings, Renderer
};

pub fn main() -> iced::Result {
    Counter::run(Settings::default())
}

struct Counter {
    value: i32,
    text: String
}

#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    InputChanged(String)
}

impl Sandbox for Counter {
    type Message = Message;


    fn new() -> Self {
        Self {
            value: 0,
            text: "".to_string()
        }
    }

    fn title(&self) -> String {
        String::from("Counter - Iced")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::IncrementPressed => {
                self.value += 1;
            }
            Message::DecrementPressed => {
                self.value -= 1;
            },
            Message::InputChanged(value) => {
                self.text = value

            }
        }
    }

    fn view(&self) -> Element<Message> {
        let input: text_input::TextInput<'_, Message, Renderer> = text_input(
            "What needs to be done?",
            &self.text,
            Message::InputChanged,
        ).width(iced::Length::Fill);


        row![
            column![
                button("Increment").on_press(Message::IncrementPressed),
                text(self.value).size(50),
                button("Decrement").on_press(Message::DecrementPressed),
            ]
            .padding(20)
            .align_items(Alignment::Center),

            column![
                input
            ]
            .padding(20)
            .align_items(Alignment::Center)
            .height(iced::Length::Fill)
        ].height(iced::Length::Fill).into()

    }

    fn theme(&self) -> iced::Theme {
        iced::Theme::default()
    }

    fn style(&self) -> iced::theme::Application {
        iced::theme::Application::default()
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }

    fn should_exit(&self) -> bool {
        false
    }

    fn run(settings: Settings<()>) -> Result<(), iced::Error>
    where
        Self: 'static + Sized,
    {
        <Self as iced::Application>::run(settings)
    }
}
