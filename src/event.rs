use iced::{Subscription, event, keyboard, mouse, window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Escape,
    LeftClick,
    OpenControlPalette,
}

pub fn events() -> Subscription<(window::Id, Event)> {
    event::listen_with(filtered_events)
}

fn filtered_events(
    event: iced::Event,
    status: iced::event::Status,
    window: window::Id,
) -> Option<(window::Id, Event)> {
    let ignored = |status| matches!(status, iced::event::Status::Ignored);

    let event = match &event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            ..
        }) => Some(Event::Escape),
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if ignored(status) => {
            Some(Event::LeftClick)
        }
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character(p),
            modifiers,
            ..
        }) if p.as_str() == "p" && modifiers.command() && modifiers.shift() => {
            Some(Event::OpenControlPalette)
        }
        _ => None,
    };

    event.map(|event| (window, event))
}
