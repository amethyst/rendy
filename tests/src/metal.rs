pub use rendy::core::{metal::Backend, EnabledBackend};

fn main() {
    println!("{:#?}", EnabledBackend::which::<Backend>());
}
