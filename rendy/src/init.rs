use config::Config;
use device::Device;
use factory::Factory;
use queue::QueuesPicker;

/// Initalize rendy
#[cfg(feature = "hal")]
pub use impls::hal::init;

/// Initialize rendy
#[cfg(not(feature = "hal"))]
pub fn init<D, Q>(_config: Config, queue_picker: Q) -> Result<(Factory<D>), ()>
where
    D: Device,
    Q: QueuesPicker,
{
    unimplemented!()
}
