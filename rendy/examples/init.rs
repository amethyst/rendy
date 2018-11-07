extern crate rendy;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;


type Factory = rendy::factory::Factory<Backend>;


fn main() -> Result<(), failure::Error> {

    env_logger::Builder::from_default_env()
        .filter_module("init", log::LevelFilter::Trace)
        .init();

    log::info!("Running 'init' example");

    let config: rendy::factory::Config = Default::default();

    let factory: Factory = Factory::new(config)?;

    factory.dispose();
    Ok(())
}
