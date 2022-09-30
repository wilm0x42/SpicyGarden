use std::fs;
use std::thread;
use std::sync::mpsc;

fn run_server(tx: mpsc::Sender<&str>) {
    tx.send("test");
    println!("Ran a server!");
    return;
}

fn main() {
    println!("SpicyGarden by wilm0x42 commit {}", env!("GIT_HASH"));

    let runner_count: u32 = 4;

    fs::create_dir_all("runners");

    let (tx, rx) = mpsc::channel::<&str>();

    for runner_index in 1..runner_count {
        let runner_tx = tx.clone();
        thread::spawn(move || {
            run_server(runner_tx);
        });
    }

    for received in rx {
        println!("Got: {}", received);
    }
}
