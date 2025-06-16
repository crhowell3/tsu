use data::appearance;
pub use theme::Theme;

pub mod theme;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Mode {
    Dark,
    Light,
}

impl From<dark_light::Mode> for Mode {
    fn from(mode: dark_light::Mode) -> Self {
        match mode {
            dark_light::Mode::Dark => Mode::Dark,
            dark_light::Mode::Light => Mode::Light,
            dark_light::Mode::Unspecified => Mode::Light,
        }
    }
}

pub fn detect() -> Option<Mode> {
    let Ok(mode) = dark_light::detect() else {
        return None;
    };

    Some(Mode::from(mode))
}

pub fn theme(selected: &data::appearance::Selected) -> data::appearance::Theme {
    match &selected {
        appearance::Selected::Static(theme) => theme.clone(),
        appearance::Selected::Dynamic { light, dark } => match detect() {
            Some(mode) => match mode {
                Mode::Dark => dark.clone(),
                Mode::Light => light.clone(),
            },
            None => appearance::Theme::default(),
        },
    }
}
