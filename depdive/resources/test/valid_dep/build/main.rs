mod custom_build;

fn main() {
    println!("I am an unnecessary build script");
    custom_build::Hello::hello();
}
