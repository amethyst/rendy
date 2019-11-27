pub use rendy::core::{EnabledBackend, gl::Backend};

fn main() {
    println!("{:#?}", EnabledBackend::which::<Backend>());
}
