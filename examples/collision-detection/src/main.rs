use collision_detection::run;

fn main() {
    println!("Hello, window!");
    pollster::block_on(run());
}