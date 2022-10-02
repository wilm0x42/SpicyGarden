use std::fs;
use std::thread;
use std::sync::mpsc;
use std::time::Duration;
use std::convert::TryFrom;

struct Seed {
    seed: String,
    claimed_runner_index: Option<u32>,
    result: Option<String>,
}

fn run_server(mut target_seed: Seed) -> Seed {
    thread::sleep(Duration::from_secs(2));
 
    println!("Ran server {} with seed {}", target_seed.claimed_runner_index.unwrap(), target_seed.seed);

    target_seed.result = Some(format!("Test result for seed: {}", target_seed.seed.clone()).to_string());

    return target_seed;
}

fn main() {
    println!("SpicyGarden by wilm0x42 commit {}", env!("GIT_HASH"));

    let gather_server_address = "127.0.0.1:8080";
    let client_key = "test_key";

    let target_runner_count: u32 = 4;
    let mut halted_runners: Vec<u32> = (0..target_runner_count).collect();

    let mut seed_pool: Vec<Seed> = vec![];
    let mut completed_seeds: Vec<Seed> = vec![];

    let (tx, rx) = mpsc::channel::<Seed>();

    let http_client = reqwest::blocking::Client::new();

    fs::create_dir_all("runners").unwrap();

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
