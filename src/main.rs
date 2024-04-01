mod app;
mod keys;
mod network;
mod node;
mod protocol;
mod rpc;
mod util;

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(app::start());
}
