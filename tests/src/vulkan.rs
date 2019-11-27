pub use rendy::core::{EnabledBackend, vulkan::Backend};

fn main() {
    println!("{:#?}", EnabledBackend::which::<Backend>());
}
