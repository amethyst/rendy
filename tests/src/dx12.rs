pub use rendy::core::{dx12::Backend, EnabledBackend};

fn main() {
    println!("{:#?}", EnabledBackend::which::<Backend>());
}
