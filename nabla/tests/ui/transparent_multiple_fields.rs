#[derive(nabla::Display)]
enum Enum {
    #[display("ok")]
    Ok,
    #[nabla(display(transparent))]
    Multiple { a: u32, b: u32 },
}

fn main() {}
