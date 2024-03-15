use crate::keys::Seed;

mod blocks;
mod keys;
mod network;
mod node;
mod rpc;

fn main() {
    let seed = Seed::random();
    println!("seed {}", seed);
    let private = seed.derive(0);
    println!("private (index 0) {}", private);
    let public = private.to_public();
    println!("public {}", public);
    let address = public.to_address();
    println!("address {}", address);
}
