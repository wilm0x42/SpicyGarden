use std::fs;
use std::thread;
use std::sync::{mpsc, atomic};
use std::time::Duration;
use std::convert::TryFrom;
use std::process::{Command, Stdio};

use iced::{executor, Application, Button, Column, Row, Element,
    Settings, Text, TextInput, Padding, Subscription};
extern crate iced_native;

extern crate fs_extra;

mod serverproperties;

// If a server takes this long to complete, then its process will be killed and its seed will be skipped
const JAVA_TIMEOUT_DURATION: Duration = Duration::from_secs(60);

// All runners will shutdown gracefully when they see this set to true
static JAVA_THREADS_SHUTDOWN: atomic::AtomicBool = atomic::AtomicBool::new(false);

struct Seed {
    seed: String,
    claimed_runner_index: Option<u32>,
    result: Option<String>,
}

fn run_server(mut target_seed: Seed) -> Seed {
    println!("Running server {} with seed {}", target_seed.claimed_runner_index.unwrap(), target_seed.seed);

    let runner_index = target_seed.claimed_runner_index.unwrap();
    let runner_dir = format!("runners/runner_{}", runner_index);

    // Clean up previous runner's server
    _ = fs::remove_dir_all(runner_dir.clone());
    match fs::create_dir_all(runner_dir.clone()) {
        Ok(_) => (),
        Err(e) => {
            println!("ERROR: Unable to create directory {} - {:?}", runner_dir.clone(), e);
            return target_seed;
        },
    };

    // Copy the template into our runner directory
    let mut copy_options = fs_extra::dir::CopyOptions::new();
    copy_options.copy_inside = true;
    copy_options.content_only = true;
    match fs_extra::dir::copy("server_template", runner_dir.clone(), &copy_options) {
        Ok(_) => (),
        Err(e) => {
            println!("ERROR: Unable to copy server_template into {} - {:?}", runner_dir, e);
            return target_seed;
        },
    };

    // Write a seed-specific (and runner-specific) server.properties
    let server_properties: String = serverproperties::get_server_properties(
        runner_index,
        &target_seed.seed.clone()
    );
    match fs::write(format!("{}/server.properties", runner_dir.clone()), server_properties) {
        Ok(_) => (),
        Err(e) => {
            println!("ERROR: Unable to write to server.properties - {:?}", e);
            return target_seed;
        },
    };

    // Start the java server in a child process
    let mut server_process: std::process::Child = match Command::new("java")
        .current_dir(runner_dir.clone())
        .args(["-Xms32M", "-Xmx512M", "-jar", "server.jar", "nogui"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn() {
        Ok(process) => process,
        Err(e) => {
            println!("ERROR: Unable to start minecraft server: {:?}", e);
            return target_seed;
        },
    };

    // Elaborate busy loop because rust doesn't help you timeout child processes
    let (timeout_tx, timeout_rx) = mpsc::channel();

    thread::spawn(move || {
        thread::sleep(JAVA_TIMEOUT_DURATION);
        let _ = timeout_tx.send("timeout");
    });

    loop {
        if JAVA_THREADS_SHUTDOWN.load(atomic::Ordering::Relaxed) {
            match server_process.kill() {
                Ok(()) => {
                    server_process.wait().unwrap(); // Wait to ensure resources are released
                }
                Err(_) => () // Process already died
            };
            return target_seed;
        }

        match server_process.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    break;
                } else {
                    println!("ERROR: Minecraft server exited with failure: {}", status);
                    return target_seed;
                }
            },
            Ok(None) => {
                thread::sleep(Duration::from_millis(10));
            },
            Err(e) => {
                println!("ERROR: Failed while waiting on java process: {:?}", e);
                return target_seed;
            },
        }

        match timeout_rx.try_recv() {
            Ok(_timeout) => {
                println!("TIMEOUT: Runner {} exceeded timeout, giving up", runner_index);
                match server_process.kill() {
                    Ok(()) => {
                        server_process.wait().unwrap(); // Wait to ensure resources are released
                    }
                    Err(_) => () // Process already died
                };
                return target_seed;
            },
            Err(_e) => (),
        }
    }
 
    // Check for results
    
    let server_result = match fs::read(format!("{}/SpicyGardenData.txt", runner_dir)) {
        Ok(result_txt) => result_txt,
        Err(e) => {
            println!("ERROR: Unable to read SpicyGardenData.txt on runner {}: {:?}", runner_index, e);
            return target_seed;
        },
    };

    let decoded_server_result = match String::from_utf8(server_result) {
        Ok(decoded) => decoded,
        Err(e) => {
            println!("ERROR: Failed to decode SpicyGardenData.txt on runner {}: {:?}", runner_index, e);
            return target_seed;
        }
    };

    target_seed.result = Some(decoded_server_result);

    return target_seed;
}

fn seed_search_loop(gather_server_address: String, client_key: String, target_runner_count: u32) {
    println!("SpicyGarden by wilm0x42 commit {}", env!("GIT_HASH"));

    let mut halted_runners: Vec<u32> = (0..target_runner_count).collect();

    let mut seed_pool: Vec<Seed> = vec![];
    let mut completed_seeds: Vec<Seed> = vec![];

    let (tx, rx) = mpsc::channel::<Seed>();

    let http_client = reqwest::blocking::Client::new();

    loop {
        // If shutdown has been signaled, wait for all runners to complete and then break

        if JAVA_THREADS_SHUTDOWN.load(atomic::Ordering::Relaxed) {
            while halted_runners.len() < target_runner_count as usize {
                let received = rx.recv().unwrap_or_else(|error| {
                    panic!("Thread communication error: {:?}", error);
                });
        
                halted_runners.push(received.claimed_runner_index.unwrap());
            }
            break;
        }
        
        // Make sure we've got seeds from the gather server in the pool

        let seed_pool_count: u32 = u32::try_from(seed_pool.len()).unwrap();

        if seed_pool_count < target_runner_count {
            let requested_seed_count = target_runner_count - seed_pool_count;
            let request_uri = format!("{}/assign_seeds/{}/{}",
                gather_server_address,
                client_key,
                requested_seed_count,
            );
            
            let response = match http_client.get(request_uri).send() {
                Ok(r) => r,
                Err(e) => {
                    println!("Unable to contact seed server: {:?} Retrying in 3s.", e);
                    thread::sleep(Duration::from_secs(3));
                    continue;
                },
            };

            if response.status() != 200 {
                println!("Bad response from seed server: {:?} Retrying in 3s.", response.status());
                thread::sleep(Duration::from_secs(3));
                continue;
            }
                
            let body_text = response.text().unwrap();
            let assigned_seeds: Vec<&str> = body_text.split("\n").collect();
            
            for seed in assigned_seeds {
                let new_seed = Seed {
                    seed: seed.to_string(),
                    claimed_runner_index: None,
                    result: None,
                };
                seed_pool.push(new_seed);
            }
        }

        // Spawn new runners if one or more has halted

        while halted_runners.len() > 0 {
            let runner_index = halted_runners.pop().unwrap();

            let mut seed = seed_pool.pop().unwrap();
            seed.claimed_runner_index = Some(runner_index);

            let runner_tx = tx.clone();

            thread::spawn(move || {
                runner_tx.send(run_server(seed)).unwrap();
            });
        }

        // Submit completed seeds to the gather server if we have any

        while completed_seeds.len() > 0 {
            let seed = completed_seeds.pop().unwrap();

            let request_uri = format!("{}/submit_result/{}",
                gather_server_address,
                client_key,
            );

            let _response = match http_client.post(request_uri)
                .header("SpicyGarden-Seed", seed.seed.clone())
                .body(seed.result.clone().unwrap())
                .send() {
                    Ok(r) => {
                        if r.status() != 204 {
                            println!("Error submitting to seed server: {:?}", r.status());
                            completed_seeds.push(seed);
                            break;
                        }
                        println!("Sent result for seed: {:?}", seed.seed.clone());
                    },
                    Err(e) => {
                        println!("Unable to submit to seed server: {:?}", e);
                        completed_seeds.push(seed);
                        break;
                    },
                };
        }
        
        // Check for completed runners

        let received = rx.recv().unwrap_or_else(|error| {
            panic!("Thread communication error: {:?}", error);
        });

        halted_runners.push(received.claimed_runner_index.unwrap());

        match &received.result {
            Some(_result) => {
                println!("Seed {:?} completed successfully.", received.seed);
            },
            None => {
                println!("Seed {:?} failed to capture data. Skipping.", received.seed);
                continue;
            }
        };

        completed_seeds.push(received);
    }
}

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

    seed_search_thread: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
enum Message {
    StartSeedSearch,
    ServerAddressChanged(String),
    ClientKeyChanged(String),
    RunnerCountChanged(String),
    Quit,
    IgnorableEvent,
}

impl Application for SpicyGarden {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (SpicyGarden, iced::Command<Self::Message>) {
        (
            SpicyGarden {
                start_button: iced::button::State::new(),

                server_address_input: iced::text_input::State::new(),
                server_address: "http://localhost:8080".to_string(),

                client_key_input: iced::text_input::State::new(),
                client_key: "test_key".to_string(),

                runner_count_input: iced::text_input::State::new(),
                runner_count: "4".to_string(),

                status_message: "".to_string(),
                running_state: RunningState::Waiting,

                seed_search_thread: None,
            },
            iced::Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("SpicyGarden - Minecraft Seed Data Collector")
    }

    fn subscription(&self) -> Subscription<Message> {
        iced_native::subscription::events().map(|event| {
            match event {
                iced_native::Event::Window(window_event) => {
                    match window_event {
                        iced_native::window::Event::CloseRequested => Message::Quit,
                        _ => Message::IgnorableEvent
                    }
                }
                _ => Message::IgnorableEvent
            }
        })
    }

    fn view(&mut self) -> Element<Self::Message> {
        let mut column = Column::new()
            .push(Row::new()
                .push(Text::new("SpicyGarden by wilm0x42").size(32))
            );
        
        if self.running_state == RunningState::Waiting {
            column = column.push(Row::new()
                .push(Text::new("Server address:"))
                .push(TextInput::new(
                    &mut self.server_address_input,
                    "example.com",
                    &self.server_address,
                    Message::ServerAddressChanged,
                    ).padding(Padding::from(8))
                )
                .align_items(iced::Alignment::Center)
                .spacing(8)
            )
            .push(Row::new()
                .push(Text::new("Client key:"))
                .push(TextInput::new(
                    &mut self.client_key_input,
                    "super_secret_key",
                    &self.client_key,
                    Message::ClientKeyChanged,
                    ).padding(Padding::from(8))
                )
                .align_items(iced::Alignment::Center)
                .spacing(8)
            )
            .push(Row::new()
                .push(Text::new("Concurrent server count:"))
                .push(TextInput::new(
                    &mut self.runner_count_input,
                    "4",
                    &self.runner_count,
                    Message::RunnerCountChanged,
                    ).padding(Padding::from(8))
                )
                .align_items(iced::Alignment::Center)
                .spacing(8)
            )
            .push(Button::new(&mut self.start_button, Text::new("Start gathering data"))
                .on_press(Message::StartSeedSearch)
                .padding(Padding::from(16))
            )
            .push(Text::new(self.status_message.clone()))
        };

        if self.running_state == RunningState::Running {
            column = column.push(Text::new(self.status_message.clone()))
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

                self.seed_search_thread = Some(thread::spawn(move || {
                    seed_search_loop(server_address, client_key, runner_count);
                }));

                self.status_message = "Collecting data...".to_string();
                self.running_state = RunningState::Running;
            },
            Message::ServerAddressChanged(value) => {
                self.server_address = value;
            },
            Message::ClientKeyChanged(value) => {
                self.client_key = value;
            },
            Message::RunnerCountChanged(value) => {
                self.runner_count = value;
            },
            Message::Quit => {
                if self.seed_search_thread.is_some() {
                    let join_handle = self.seed_search_thread.take().unwrap();
                    self.running_state = RunningState::Quitting;
                    JAVA_THREADS_SHUTDOWN.store(true, atomic::Ordering::Relaxed);
                    //TODO: Join this thread in a non-blocking way, such that iced is informed when it's done
                    join_handle.join().expect("FATAL: Failed to join seed search thread");
                };
                self.running_state = RunningState::Quit;
            },
            Message::IgnorableEvent => {},
        }
        iced::Command::none()
    }

    fn should_exit(&self) -> bool {
        self.running_state == RunningState::Quit
    }
}

fn main() {
    let mut settings: Settings<()> = Settings::default();
    settings.window.size = (400, 300);
    settings.exit_on_close_request = false;
    SpicyGarden::run(settings).unwrap();
}