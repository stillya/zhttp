mod runner;

use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    runner::run(&args);
}
