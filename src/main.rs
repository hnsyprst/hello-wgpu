use hello_wgpu::run;

fn main() {
    println!("Hello, window!");
    pollster::block_on(run());
}
