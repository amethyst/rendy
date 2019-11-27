pub use rendy::core::{EnabledBackend, dx12::Backend};

fn main() {
    println!("{:#?}", EnabledBackend::which::<Backend>());
}
