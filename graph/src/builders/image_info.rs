use rendy_core::hal;
use crate::scheduler::resources::{ImageMode, ImageInfo};

pub struct ImageInfoBuilder {
    kind: Option<hal::image::Kind>,
    format: Option<hal::format::Format>,
    mode: ImageMode,
}

impl Default for ImageInfoBuilder {
    fn default() -> Self {
        ImageInfoBuilder {
            kind: None,
            format: None,
            mode: ImageMode::Clear {
                clear: hal::command::ClearValue::default(),
            },
        }
    }
}

impl ImageInfoBuilder {

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_clear(mut self, clear: hal::command::ClearValue) -> Self {
        self.mode = ImageMode::Clear {
            clear,
        };
        self
    }

    pub fn with_clear_black(self) -> Self {
        self.with_clear(hal::command::ClearValue::default())
    }

    pub fn with_format(mut self, format: hal::format::Format) -> Self {
        self.format = Some(format);
        self
    }

    pub fn with_kind(mut self, kind: hal::image::Kind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn infer_kind(mut self) -> Self {
        self.kind = None;
        self
    }

    pub fn build(self) -> ImageInfo {
        ImageInfo {
            kind: self.kind,
            levels: 1,
            format: self.format.expect("need format"),
            mode: self.mode,
        }
    }

}
