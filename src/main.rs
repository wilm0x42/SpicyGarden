use std::fs;
use std::thread;
use std::sync::mpsc;
use std::time::Duration;
use std::convert::TryFrom;
use std::process::Command;

extern crate fs_extra;

mod serverproperties;

static JAVA_TIMEOUT_DURATION: Duration = Duration::from_secs(60);

struct Seed {
    seed: String,
    claimed_runner_index: Option<u32>,
    result: Option<String>,
}

fn run_server(mut target_seed: Seed) -> Seed {
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
        .args(["-Xms32M", "-Xmx512M", "-jar", &format!("{}/server.jar", runner_dir.clone()), ])
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
            }

        }

        match timeout_rx.try_recv() {
            Ok(_timeout) => {
                println!("TIMEOUT: Runner {} exceeded timeout, giving up", runner_index);
                return target_seed;
            },
            Err(_e) => (),
        }
    }
 
    println!("Ran server {} with seed {}", target_seed.claimed_runner_index.unwrap(), target_seed.seed);

    target_seed.result = Some(format!("Test result for seed: {}", target_seed.seed.clone()).to_string());

    return target_seed;
}

fn main() {
    println!("SpicyGarden by wilm0x42 commit {}", env!("GIT_HASH"));

    let gather_server_address = "127.0.0.1:8080";
    let client_key = "test_key";

    let target_runner_count: u32 = 1;
    let mut halted_runners: Vec<u32> = (0..target_runner_count).collect();

    let mut seed_pool: Vec<Seed> = vec![];
    let mut completed_seeds: Vec<Seed> = vec![];

    let (tx, rx) = mpsc::channel::<Seed>();

    let http_client = reqwest::blocking::Client::new();

    //fs::create_dir_all("runners").unwrap();

    loop {

        // Make sure we've got seeds from the gather server in the pool

        let seed_pool_count: u32 = u32::try_from(seed_pool.len()).unwrap();

        if seed_pool_count < target_runner_count {
            let requested_seed_count = target_runner_count - seed_pool_count;
            let request_uri = format!("http://{}/assign_seeds/{}/{}",
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

            let request_uri = format!("http://{}/submit_result/{}",
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
        
        println!("Looping");
    }
}
