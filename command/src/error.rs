//! Error module docs.

/// Error that can be returned by some functions
/// indicating that logical device is lost.
/// Those methods on objects created from the device will likely result in this error again.
/// When device is lost user should free all objects created from it and destroy the device.
/// After that user can create new device to continue.
/// Note: physical device may be lost as well.
#[derive(Clone, Copy, Debug, Fail)]
#[fail(display = "Device lost. Re-initialization required")]
pub struct DeviceLost;
