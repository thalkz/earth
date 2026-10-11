use crate::app::run;

mod app;

fn main() {
    println!("Hello, world!");
    run().expect("failed to run");
}
