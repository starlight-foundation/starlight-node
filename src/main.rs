use keys::seed::Seed;

mod keys;
mod error;
mod node;
mod blocks;
mod pow;

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
