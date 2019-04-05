use {
    crate::{
        factory::{Factory, ImageState},
        memory::Data,
        pixel::AsPixel,
        resource::{Escape, Handle, Image, ImageInfo, ImageView, ImageViewInfo, Sampler},
        util::cast_cow,
    },
    gfx_hal::{image, Backend},
};

/// Static image.
/// Can be loaded from various of formats.
#[derive(Debug)]
pub struct Texture<B: Backend> {
    image: Handle<Image<B>>,
    view: Escape<ImageView<B>>,
    sampler: Handle<Sampler<B>>,
}

impl<B> Texture<B>
where
    B: Backend,
{
    /// Get image handle.
    pub fn image(&self) -> &Handle<Image<B>> {
        &self.image
    }

    /// Get sampler handle.
    pub fn sampler(&self) -> &Handle<Sampler<B>> {
        &self.sampler
    }

    /// Get reference to image view.
    pub fn view(&self) -> &ImageView<B> {
        &self.view
    }

    /// Get mutable reference to image view.
    pub fn view_mut(&mut self) -> &mut ImageView<B> {
        &mut self.view
    }
}

/// Generics-free texture builder.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextureBuilder<'a> {
    kind: image::Kind,
    view_kind: image::ViewKind,
    format: gfx_hal::format::Format,
    data: std::borrow::Cow<'a, [u8]>,
    data_width: u32,
    data_height: u32,
    sampler_info: gfx_hal::image::SamplerInfo,
    swizzle: gfx_hal::format::Swizzle,
}

impl<'a> TextureBuilder<'a> {
    /// New empty builder.
    pub fn new() -> Self {
        TextureBuilder {
            kind: image::Kind::D1(0, 0),
            view_kind: image::ViewKind::D1,
            format: gfx_hal::format::Format::Rgba8Unorm,
            data: std::borrow::Cow::Borrowed(&[]),
            data_width: 0,
            data_height: 0,
            sampler_info: gfx_hal::image::SamplerInfo::new(
                gfx_hal::image::Filter::Linear,
                gfx_hal::image::WrapMode::Clamp,
            ),
            swizzle: gfx_hal::format::Swizzle::NO,
        }
    }

    /// Set pixel data.
    pub fn with_data<P: AsPixel>(mut self, data: impl Into<std::borrow::Cow<'a, [P]>>) -> Self {
        self.set_data(data);
        self
    }

    /// Set pixel data.
    pub fn set_data<P: AsPixel>(
        &mut self,
        data: impl Into<std::borrow::Cow<'a, [P]>>,
    ) -> &mut Self {
        self.data = cast_cow(data.into());
        self.format = P::FORMAT;
        self
    }

    /// Set pixel data with manual format definition.
    pub fn with_raw_data(
        mut self,
        data: impl Into<std::borrow::Cow<'a, [u8]>>,
        format: gfx_hal::format::Format,
    ) -> Self {
        self.set_raw_data(data, format);
        self
    }

    /// Set pixel data with manual format definition.
    pub fn set_raw_data(
        &mut self,
        data: impl Into<std::borrow::Cow<'a, [u8]>>,
        format: gfx_hal::format::Format,
    ) -> &mut Self {
        self.data = data.into();
        self.format = format;
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
    pub fn with_kind(mut self, kind: image::Kind) -> Self {
        self.set_kind(kind);
        self
    }

    /// Set image kind.
    pub fn set_kind(&mut self, kind: image::Kind) -> &mut Self {
        self.kind = kind;
        self
    }

    /// With image view kind.
    pub fn with_view_kind(mut self, view_kind: image::ViewKind) -> Self {
        self.set_view_kind(view_kind);
        self
    }

    /// Set image view kind.
    pub fn set_view_kind(&mut self, view_kind: image::ViewKind) -> &mut Self {
        self.view_kind = view_kind;
        self
    }

    /// With image sampler info.
    pub fn with_sampler_info(mut self, sampler_info: gfx_hal::image::SamplerInfo) -> Self {
        self.set_sampler_info(sampler_info);
        self
    }

    /// Set image sampler info.
    pub fn set_sampler_info(&mut self, sampler_info: gfx_hal::image::SamplerInfo) -> &mut Self {
        self.sampler_info = sampler_info;
        self
    }

    /// With swizzle.
    pub fn with_swizzle(mut self, swizzle: gfx_hal::format::Swizzle) -> Self {
        self.set_swizzle(swizzle);
        self
    }

    /// Set swizzle.
    pub fn set_swizzle(&mut self, swizzle: gfx_hal::format::Swizzle) -> &mut Self {
        self.swizzle = swizzle;
        self
    }

    /// Build texture.
    ///
    /// ## Parameters
    /// * `next_state`: The next state that this texture will be used in.
    ///     It will get transitioned to this state after uploading.
    /// * `factory`: Factory to use to build the texture
    pub fn build<B>(
        &self,
        next_state: ImageState,
        factory: &'a mut Factory<B>,
    ) -> Result<Texture<B>, failure::Error>
    where
        B: Backend,
    {
        let image: Handle<Image<B>> = factory
            .create_image(
                ImageInfo {
                    kind: self.kind,
                    levels: 1,
                    format: self.format,
                    tiling: image::Tiling::Optimal,
                    view_caps: image::ViewCapabilities::empty(),
                    usage: image::Usage::SAMPLED | image::Usage::TRANSFER_DST,
                },
                Data,
            )?
            .into();

        unsafe {
            factory.upload_image(
                &image,
                self.data_width,
                self.data_height,
                image::SubresourceLayers {
                    aspects: self.format.surface_desc().aspects,
                    level: 0,
                    layers: 0..self.kind.num_layers(),
                },
                image::Offset::ZERO,
                self.kind.extent(),
                &self.data,
                image::Layout::Undefined,
                next_state,
            )?;
        }

        let view = factory.create_image_view(
            image.clone(),
            ImageViewInfo {
                view_kind: self.view_kind,
                format: self.format,
                swizzle: self.swizzle,
                range: image::SubresourceRange {
                    aspects: self.format.surface_desc().aspects,
                    levels: 0..1,
                    layers: 0..self.kind.num_layers(),
                },
            },
        )?;

        let sampler = factory.get_sampler(self.sampler_info.clone())?;

        Ok(Texture {
            image,
            view,
            sampler,
        })
    }
}
