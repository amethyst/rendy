//! Exports the image and palette modules if the features
//! are enabled

#[cfg(feature = "image")]
pub mod image;
#[cfg(feature = "palette")]
pub mod palette;
