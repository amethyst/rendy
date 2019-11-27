pub use rendy::core::{vulkan::Backend, EnabledBackend};

fn main() {
    println!("{:#?}", EnabledBackend::which::<Backend>());
}
