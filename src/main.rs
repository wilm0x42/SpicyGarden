use std::fs;
use std::sync::atomic;
use std::thread;
use std::time::{Duration, Instant};

use iced::{
    executor, Application, Button, Column, Element, Padding, Row, Settings, Subscription, Text,
    TextInput,
};
extern crate iced_native;

extern crate fs_extra;

use serde::Deserialize;

mod runner;

#[derive(Debug, Clone, PartialEq)]
enum RunningState {
    Waiting,
    Running,
    Quitting,
    Quit,
}

struct SpicyGarden {
    start_button: iced::button::State,

    server_address_input: iced::text_input::State,
    server_address: String,

    client_key_input: iced::text_input::State,
    client_key: String,

    runner_count_input: iced::text_input::State,
    runner_count: String,

    status_message: String,
    running_state: RunningState,

    searched_seed_count: u32,
    started_running_at: Option<Instant>,
}

#[derive(Debug, Clone)]
enum Message {
    StartSeedSearch,
    StoppedSeedSearch,
    SearchedSeedCountUpdated(u32),
    ServerAddressChanged(String),
    ClientKeyChanged(String),
    RunnerCountChanged(String),
    Quit,
    IgnorableEvent,
}

#[derive(Deserialize)]
#[derive(Default)]
struct SpicyGardenFlags {
    server_address: String,
    client_key: String,
    runner_count: u32,
}

impl Application for SpicyGarden {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = SpicyGardenFlags;

    fn new(flags: SpicyGardenFlags) -> (SpicyGarden, iced::Command<Self::Message>) {
        (
            SpicyGarden {
                start_button: iced::button::State::new(),

                server_address_input: iced::text_input::State::new(),
                server_address: flags.server_address,

                client_key_input: iced::text_input::State::new(),
                client_key: flags.client_key,

                runner_count_input: iced::text_input::State::new(),
                runner_count: flags.runner_count.to_string(),

                status_message: "".to_string(),
                running_state: RunningState::Waiting,

                searched_seed_count: 0,
                started_running_at: None,
            },
            iced::Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("SpicyGarden - Minecraft Seed Data Collector")
    }

    fn subscription(&self) -> Subscription<Message> {
        let subscriptions: Vec<Subscription<Message>> = vec![
            iced_native::subscription::events().map(|event| match event {
                iced_native::Event::Window(window_event) => match window_event {
                    iced_native::window::Event::CloseRequested => Message::Quit,
                    _ => Message::IgnorableEvent,
                },
                _ => Message::IgnorableEvent,
            }),
            iced_native::subscription::unfold((), self.searched_seed_count.clone(), |known_seed_count| async move {
                // NOTE: Because rust standard library doesn't have unbounded channels
                // that would let me bring a reference of a mpsc Receiver into a
                // closure like this, and I don't want to use Tokio, we're doing
                // this funny caricature of a busy loop to watch for when the
                // searched seed count updates.
                // Ain't I a stinker?

                thread::sleep(Duration::from_millis(100));

                let current_seed_count: u32 = runner::JAVA_SEED_SEARCH_COUNT.load(atomic::Ordering::Relaxed);

                if current_seed_count != known_seed_count {
                    return (Some(Message::SearchedSeedCountUpdated(current_seed_count)), current_seed_count);
                };

                (Some(Message::IgnorableEvent), known_seed_count)
            }),
        ];

        Subscription::batch(subscriptions.into_iter())
    }

    fn view(&mut self) -> Element<Self::Message> {
        let mut column =
            Column::new().push(Row::new().push(Text::new("SpicyGarden by wilm0x42").size(32)));

        if self.running_state == RunningState::Waiting {
            column = column
                .push(
                    Row::new()
                        .push(Text::new("Server address:"))
                        .push(
                            TextInput::new(
                                &mut self.server_address_input,
                                "example.com",
                                &self.server_address,
                                Message::ServerAddressChanged,
                            )
                            .padding(Padding::from(8)),
                        )
                        .align_items(iced::Alignment::Center)
                        .spacing(8),
                )
                .push(
                    Row::new()
                        .push(Text::new("Client key:"))
                        .push(
                            TextInput::new(
                                &mut self.client_key_input,
                                "super_secret_key",
                                &self.client_key,
                                Message::ClientKeyChanged,
                            )
                            .padding(Padding::from(8)),
                        )
                        .align_items(iced::Alignment::Center)
                        .spacing(8),
                )
                .push(
                    Row::new()
                        .push(Text::new("Concurrent server count:"))
                        .push(
                            TextInput::new(
                                &mut self.runner_count_input,
                                "4",
                                &self.runner_count,
                                Message::RunnerCountChanged,
                            )
                            .padding(Padding::from(8)),
                        )
                        .align_items(iced::Alignment::Center)
                        .spacing(8),
                )
                .push(
                    Button::new(&mut self.start_button, Text::new("Start gathering data"))
                        .on_press(Message::StartSeedSearch)
                        .padding(Padding::from(16)),
                )
                .push(Text::new(self.status_message.clone()))
        };

        if self.running_state == RunningState::Running {
            column = column
                .push(Text::new(self.status_message.clone()))
                .push(Text::new(format!("Seeds searched so far: {}", self.searched_seed_count)));
            
            if let Some(started_at) = self.started_running_at {
                let running_duration: f32 = started_at.elapsed().as_secs() as f32;
                let duration_hours: f32 = running_duration / (60.0 * 60.0);
                let seeds_per_minute: f32 = (self.searched_seed_count as f32) / (running_duration / 60.0);

                column = column
                    .push(Text::new(format!("Seeds per minute: {:.2}", seeds_per_minute)))
                    .push(Text::new(format!("Uptime: {:.2} hours", duration_hours)));
            };
        };

        if self.running_state == RunningState::Quitting {
            column = column.push(Text::new("Shutting down..."))
        };

        column = column
            .padding(Padding::from(8))
            .spacing(8)
            .width(iced_native::Length::Fill)
            .align_items(iced::Alignment::Center);

        Element::from(column)
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::StartSeedSearch => {
                let server_address = self.server_address.clone();
                let client_key = self.client_key.clone();

                let runner_count = match self.runner_count.parse::<u32>() {
                    Ok(value) => value,
                    Err(_e) => {
                        self.status_message = "ERROR: Invalid target runner count".to_string();
                        return iced::Command::none();
                    }
                };

                self.status_message = format!("Collecting data with {} runners...", runner_count)
                    .to_string();
                self.running_state = RunningState::Running;
                self.started_running_at = Some(Instant::now());

                return iced::Command::perform(
                    runner::seed_search_async_wrapper(server_address, client_key, runner_count),
                    |_| Message::StoppedSeedSearch,
                );
            }
            Message::StoppedSeedSearch => {
                self.running_state = RunningState::Quit;
            }
            Message::SearchedSeedCountUpdated(value) => {
                self.searched_seed_count = value;
            }
            Message::ServerAddressChanged(value) => {
                self.server_address = value;
            }
            Message::ClientKeyChanged(value) => {
                self.client_key = value;
            }
            Message::RunnerCountChanged(value) => {
                self.runner_count = value;
            }
            Message::Quit => {
                if self.running_state == RunningState::Running {
                    self.running_state = RunningState::Quitting;
                    runner::JAVA_THREADS_SHUTDOWN.store(true, atomic::Ordering::Relaxed);
                } else {
                    self.running_state = RunningState::Quit;
                };
            }
            Message::IgnorableEvent => {}
        }
        iced::Command::none()
    }

    fn should_exit(&self) -> bool {
        self.running_state == RunningState::Quit
    }
}

fn main() {
    println!("SpicyGarden by wilm0x42 commit {}", env!("GIT_HASH"));
    
    // Define default initial values

    let mut flags = SpicyGardenFlags {
        server_address: "".to_string(),
        client_key: "".to_string(),
        runner_count: 1,
    };

    // Load values from config.toml, if possible

    match fs::read("config.toml") {
        Ok(toml_slice) => {
            match toml::from_slice::<SpicyGardenFlags>(&toml_slice) {
                Ok(config) => {
                    flags = config;
                },
                Err(e) => {
                    println!("ERROR: Failed to parse config.toml: {:?}", e);
                }
            }
        },
        Err(_) => {
            println!("Couldn't read config.toml, using default values.");
        }
    };

    // Start the GUI

    let mut settings: Settings<SpicyGardenFlags> = Settings::default();
    settings.flags = flags;
    settings.window.size = (400, 300);
    settings.exit_on_close_request = false;
    SpicyGarden::run(settings).unwrap();
}
