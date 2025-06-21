#![allow(clippy::large_enum_variant, clippy::too_many_arguments)]

mod appearance;
mod event;
mod font;
mod icon;
mod widget;
mod window;

use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use appearance::{Theme, theme};
use clap::Parser;
use data::config::{self, Config};
use data::environment;
use iced::keyboard;
use iced::widget::{column, container, horizontal_space, row, text, text_editor};
use iced::{Fill, Subscription, Task};
use tokio::runtime;
use tracing::{debug, error, info};

use self::event::{Event, events};
use self::widget::Element;
use self::window::Window;

#[derive(Parser, Debug)]
#[clap(name = "tsu")]
#[command(
    version,
    about,
    author = "Cameron Howell <me@crhowell.com>",
    display_name = "tsu text editor",
    help_template = "{name} {version}
{author-with-newline}{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
"
)]
struct Args {
    /// Log level
    #[arg(short, action = clap::ArgAction::Count, help="Increases logging verbosity (-v, -vv, -vvv)")]
    verbose: u8,
    /// File to open
    #[arg(default_value = "")]
    file: String,
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let user_level = match args.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filename = args.file;

    let crate_name = env!("CARGO_CRATE_NAME");
    let filter = tracing_subscriber::EnvFilter::new(format!("{crate_name}={user_level}"));

    tracing_subscriber::fmt::fmt()
        .with_env_filter(filter)
        .init();

    info!("tsu {} has started", environment::formatted_version());
    info!("tsu config dir: {:?}", environment::config_dir());
    info!("tsu data dir: {:?}", environment::data_dir());

    let (config_load, window_load) = {
        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(async {
            let config = Config::load().await;
            let window = data::Window::load().await;

            (config, window)
        })
    };

    // Font must be set using config
    font::set(config_load.as_ref().ok());

    let settings = settings(&config_load);

    iced::daemon(
        move || Tsu::new(filename.clone(), window_load.clone()),
        Tsu::update,
        Tsu::view,
    )
    .title(Tsu::title)
    .theme(Tsu::theme)
    .subscription(Tsu::subscription)
    .settings(settings)
    .run()
    .inspect_err(|err| error!("{err}"))?;

    Ok(())
}

fn settings(config_load: &Result<Config, config::Error>) -> iced::Settings {
    let default_text_size = config_load
        .as_ref()
        .ok()
        .and_then(|config| config.font.size)
        .map_or(theme::TEXT_SIZE, f32::from);

    iced::Settings {
        default_font: font::MONO.clone().into(),
        default_text_size: default_text_size.into(),
        id: None,
        antialiasing: false,
        fonts: font::load(),
    }
}

struct Tsu {
    file: Option<PathBuf>,
    content: text_editor::Content,
    theme: Theme,
    word_wrap: bool,
    is_loading: bool,
    is_dirty: bool,
    main_window: Window,
}

#[derive(Debug, Clone)]
pub enum Message {
    ActionPerformed(text_editor::Action),
    ThemeSelected(Theme),
    Event(window::Id, Event),
    Window(window::Id, window::Event),
    NewFile,
    OpenFile,
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
    SaveFile,
    FileSaved(Result<PathBuf, Error>),
}

impl Tsu {
    fn new(
        filename: String,
        window_load: Result<data::Window, window::Error>,
    ) -> (Self, Task<Message>) {
        let data::Window { size, position } = window_load.unwrap_or_default();
        let position = position.map(window::Position::Specific).unwrap_or_default();

        let (main_window, open_main_window) = window::open(window::Settings {
            size,
            position,
            min_size: Some(window::MIN_SIZE),
            exit_on_close_request: false,
            ..window::settings()
        });

        let main_window = Window::new(main_window);

        let commands = vec![
            open_main_window.then(|_| Task::none()),
            Task::perform(load_file(filename), Message::FileOpened),
            iced::widget::focus_next(),
        ];

        (
            Self {
                file: None,
                content: text_editor::Content::new(),
                theme: appearance::Theme::default(),
                word_wrap: true,
                is_loading: true,
                is_dirty: false,
                main_window,
            },
            Task::batch(commands),
        )
    }

    fn title(&self, _window_id: window::Id) -> String {
        String::from("tsu")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ActionPerformed(action) => {
                self.is_dirty = self.is_dirty || action.is_edit();

                self.content.perform(action);

                Task::none()
            }
            Message::ThemeSelected(theme) => {
                self.theme = theme;

                Task::none()
            }
            Message::Event(_window, _event) => Task::none(),
            Message::Window(id, event) => {
                if id == self.main_window.id {
                    match event {
                        window::Event::Moved(position) => {
                            self.main_window.position = Some(position);
                        }
                        window::Event::Resized(size) => {
                            self.main_window.size = size;
                        }
                        window::Event::Focused => {
                            self.main_window.focused = true;
                        }
                        window::Event::Unfocused => {
                            self.main_window.focused = false;
                        }
                        window::Event::Opened { position, size } => {
                            self.main_window.opened(position, size);
                        }
                        window::Event::CloseRequested => {
                            return iced::exit();
                        }
                    }
                    Task::none()
                } else {
                    Task::none()
                }
            }
            Message::NewFile => {
                if !self.is_loading {
                    self.file = None;
                    self.content = text_editor::Content::new();
                }

                Task::none()
            }
            Message::OpenFile => {
                if self.is_loading {
                    Task::none()
                } else {
                    self.is_loading = true;

                    Task::perform(open_file(), Message::FileOpened)
                }
            }
            Message::FileOpened(result) => {
                self.is_loading = false;
                self.is_dirty = false;

                if let Ok((path, contents)) = result {
                    self.file = Some(path);
                    self.content = text_editor::Content::with_text(&contents);
                }

                Task::none()
            }
            Message::SaveFile => {
                if self.is_loading {
                    Task::none()
                } else {
                    self.is_loading = true;

                    let mut text = self.content.text();

                    if let Some(ending) = self.content.line_ending() {
                        if !text.ends_with(ending.as_str()) {
                            text.push_str(ending.as_str());
                        }
                    }

                    Task::perform(save_file(self.file.clone(), text), Message::FileSaved)
                }
            }
            Message::FileSaved(result) => {
                self.is_loading = false;

                if let Ok(path) = result {
                    self.file = Some(path);
                    self.is_dirty = false;
                }

                Task::none()
            }
        }
    }

    fn view(&self, id: window::Id) -> Element<Message> {
        if id == self.main_window.id {
            let status = row![
                text(if let Some(path) = &self.file {
                    let path = path.display().to_string();

                    if path.len() > 60 {
                        format!("...{}", &path[path.len() - 40..])
                    } else {
                        path
                    }
                } else {
                    String::from("New file")
                }),
                horizontal_space(),
                text({
                    let (line, column) = self.content.cursor_position();

                    format!("{}:{}", line + 1, column + 1)
                })
            ]
            .spacing(10);

            let base = container(
                column![
                    text_editor(&self.content)
                        .height(Fill)
                        .on_action(Message::ActionPerformed)
                        .wrapping(if self.word_wrap {
                            text::Wrapping::Word
                        } else {
                            text::Wrapping::None
                        })
                        .key_binding(|key_press| {
                            match key_press.key.as_ref() {
                                keyboard::Key::Character("s") if key_press.modifiers.control() => {
                                    debug!("CTRL + S pressed");
                                    Some(text_editor::Binding::Custom(Message::SaveFile))
                                }
                                keyboard::Key::Named(keyboard::key::Named::Escape) => {
                                    debug!("ESC pressed");
                                    Some(text_editor::Binding::Unfocus)
                                }
                                _ => text_editor::Binding::from_key_press(key_press),
                            }
                        }),
                    status,
                ]
                .spacing(10)
                .padding(10),
            );

            base.into()
        } else {
            column![].into()
        }
    }

    fn theme(&self, _window_id: window::Id) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        let subscriptions = vec![
            events().map(|(window, event)| Message::Event(window, event)),
            window::events().map(|(window, event)| Message::Window(window, event)),
        ];

        Subscription::batch(subscriptions)
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    DialogClosed,
    IoError(io::ErrorKind),
}

async fn open_file() -> Result<(PathBuf, Arc<String>), Error> {
    let picked_file = rfd::AsyncFileDialog::new()
        .set_title("Open a text file")
        .pick_file()
        .await
        .ok_or(Error::DialogClosed)?;

    load_file(picked_file).await
}

async fn load_file(path: impl Into<PathBuf>) -> Result<(PathBuf, Arc<String>), Error> {
    let path = path.into();

    let contents = tokio::fs::read_to_string(&path)
        .await
        .map(Arc::new)
        .map_err(|error| Error::IoError(error.kind()))?;

    Ok((path, contents))
}

async fn save_file(path: Option<PathBuf>, contents: String) -> Result<PathBuf, Error> {
    let path = if let Some(path) = path {
        path
    } else {
        rfd::AsyncFileDialog::new()
            .save_file()
            .await
            .as_ref()
            .map(rfd::FileHandle::path)
            .map(Path::to_owned)
            .ok_or(Error::DialogClosed)?
    };

    tokio::fs::write(&path, contents)
        .await
        .map_err(|error| Error::IoError(error.kind()))?;

    Ok(path)
}
