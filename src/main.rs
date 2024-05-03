mod app;
mod bench;
mod keys;
mod network;
mod state;
mod protocol;
mod rpc;
mod util;
mod storage;
mod process;
mod waiting;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|x| x.as_str()) {
        Some("bench") => bench::start(),
        _ => app::start()
    };
}
