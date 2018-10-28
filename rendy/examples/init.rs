extern crate ash;
extern crate failure;
extern crate rendy;

use rendy::factory::{Config, Factory};
fn main() -> Result<(), failure::Error> {
    env_logger::init();

    let config: Config = Default::default();

    let factory: Factory = Factory::new(config)?;

    factory.dispose();
    Ok(())
}
