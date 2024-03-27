mod protocol;
mod keys;
mod network;
mod node;
mod rpc;
mod util;
mod app;

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(app::start());
}
