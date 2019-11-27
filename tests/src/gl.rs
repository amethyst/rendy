pub use rendy::core::{gl::Backend, EnabledBackend};

fn main() {
    println!("{:#?}", EnabledBackend::which::<Backend>());
}
