pub use rendy::core::{EnabledBackend, metal::Backend};

fn main() {
    println!("{:#?}", EnabledBackend::which::<Backend>());
}
