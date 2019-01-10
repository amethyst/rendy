
use crate::{
    pixel::AsPixel,
    command::QueueId,
    resource::image::{Image, Texture as TextureUsage},
    factory::{Factory, ImageState},
    util::cast_cow,
};

/// Static image.
/// Can be loaded from various of formats.
#[derive(Debug)]
pub struct Texture<B: gfx_hal::Backend> {
    image: Image<B>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextureBuilder<'a> {
    kind: gfx_hal::image::Kind,
    format: gfx_hal::format::Format,
    data: std::borrow::Cow<'a, [u8]>,
    data_width: u32,
    data_height: u32,
}

impl<'a> TextureBuilder<'a> {
    /// New empty builder.
    pub fn new() -> Self {
        TextureBuilder {
            kind: gfx_hal::image::Kind::D1(0, 0),
            format: gfx_hal::format::Format::Rgba8Unorm,
            data: std::borrow::Cow::Borrowed(&[]),
            data_width: 0,
            data_height: 0,
        }
    }

    /// Set pixel data.
    pub fn with_data<P: AsPixel>(mut self, data: impl Into<std::borrow::Cow<'a, [P]>>) -> Self {
        self.set_data(data);
        self
    }

    /// Set pixel data.
    pub fn set_data<P: AsPixel>(&mut self, data: impl Into<std::borrow::Cow<'a, [P]>>) -> &mut Self {
        self.data = cast_cow(data.into());
        self.format = P::FORMAT;
        self
    }

    /// Set pixel data width.
    pub fn with_data_width(mut self, data_width: u32) -> Self {
        self.set_data_width(data_width);
        self
    }

    /// Set pixel data width.
    pub fn set_data_width(&mut self, data_width: u32) -> &mut Self {
        self.data_width = data_width;
        self
    }

    /// Set pixel data height.
    pub fn with_data_height(mut self, data_height: u32) -> Self {
        self.set_data_height(data_height);
        self
    }

    /// Set pixel data height.
    pub fn set_data_height(&mut self, data_height: u32) -> &mut Self {
        self.data_height = data_height;
        self
    }

    /// Set image extent.
    pub fn with_kind(mut self, kind: gfx_hal::image::Kind) -> Self {
        self.set_kind(kind);
        self
    }

    /// Set image kind.
    pub fn set_kind(&mut self, kind: gfx_hal::image::Kind) -> &mut Self {
        self.kind = kind;
        self
    }

    /// Build texture.
    pub fn build<B>(
        &self,
        queue: QueueId,
        access: gfx_hal::image::Access,
        layout: gfx_hal::image::Layout,
        factory: &mut Factory<B>,
    ) -> Result<Texture<B>, failure::Error>
    where
        B: gfx_hal::Backend,
    {
        let mut image = factory.create_image(
            256,
            self.kind,
            1,
            self.format,
            gfx_hal::image::Tiling::Optimal,
            gfx_hal::image::ViewCapabilities::empty(),
            TextureUsage,
        )?;

        unsafe {
            factory.upload_image(
                &mut image,
                self.data_width,
                self.data_height,
                gfx_hal::image::SubresourceLayers {
                    aspects: self.format.surface_desc().aspects,
                    level: 0,
                    layers: 0 .. 1,
                },
                gfx_hal::image::Offset::ZERO,
                self.kind.extent(),
                &self.data,
                gfx_hal::image::Layout::Undefined,
                ImageState::new(queue, layout)
                    .with_access(access)
            )?;
        }

        Ok(Texture {
            image,
        })
    }
}
