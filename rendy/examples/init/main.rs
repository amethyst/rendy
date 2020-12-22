//!
//! Basic example initializes core type of the rendy - `Factory` and exits.
//!

use rendy::{factory::Config, init::AnyRendy};

fn main() {
    env_logger::Builder::from_default_env()
        .filter_module("init", log::LevelFilter::Trace)
        .init();

    let config: Config = Default::default();
    let _rendy = AnyRendy::init_auto(&config).unwrap();

    rendy::with_any_rendy!((rendy) (_, _) => {});
}
