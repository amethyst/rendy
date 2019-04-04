use {
    crate::{
        factory::{Factory, ImageState},
        memory::Data,
        pixel::AsPixel,
        resource::{Escape, Handle, Image, ImageInfo, ImageView, ImageViewInfo, Sampler},
        util::cast_cow,
    },
    gfx_hal::Backend,
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
    kind: gfx_hal::image::Kind,
    view_kind: gfx_hal::image::ViewKind,
    format: gfx_hal::format::Format,
    data: std::borrow::Cow<'a, [u8]>,
    data_width: u32,
    data_height: u32,
    filter: gfx_hal::image::Filter,
    swizzle: gfx_hal::format::Swizzle,
}

impl<'a> TextureBuilder<'a> {
    /// New empty builder.
    pub fn new() -> Self {
        TextureBuilder {
            kind: gfx_hal::image::Kind::D1(0, 0),
            view_kind: gfx_hal::image::ViewKind::D1,
            format: gfx_hal::format::Format::Rgba8Unorm,
            data: std::borrow::Cow::Borrowed(&[]),
            data_width: 0,
            data_height: 0,
            filter: gfx_hal::image::Filter::Linear,
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
    pub fn with_kind(mut self, kind: gfx_hal::image::Kind) -> Self {
        self.set_kind(kind);
        self
    }

    /// Set image kind.
    pub fn set_kind(&mut self, kind: gfx_hal::image::Kind) -> &mut Self {
        self.kind = kind;
        self
    }

    /// With image view kind.
    pub fn with_view_kind(mut self, view_kind: gfx_hal::image::ViewKind) -> Self {
        self.set_view_kind(view_kind);
        self
    }

    /// Set image view kind.
    pub fn set_view_kind(&mut self, view_kind: gfx_hal::image::ViewKind) -> &mut Self {
        self.view_kind = view_kind;
        self
    }

    /// With image filter.
    pub fn with_filter(mut self, filter: gfx_hal::image::Filter) -> Self {
        self.set_filter(filter);
        self
    }

    /// Set image filter.
    pub fn set_filter(&mut self, filter: gfx_hal::image::Filter) -> &mut Self {
        self.filter = filter;
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
                    tiling: gfx_hal::image::Tiling::Optimal,
                    view_caps: gfx_hal::image::ViewCapabilities::empty(),
                    usage: gfx_hal::image::Usage::SAMPLED,
                },
                Data,
            )?
            .into();

        unsafe {
            factory.upload_image(
                &image,
                self.data_width,
                self.data_height,
                gfx_hal::image::SubresourceLayers {
                    aspects: self.format.surface_desc().aspects,
                    level: 0,
                    layers: 0..self.kind.num_layers(),
                },
                gfx_hal::image::Offset::ZERO,
                self.kind.extent(),
                &self.data,
                gfx_hal::image::Layout::Undefined,
                next_state,
            )?;
        }

        let view = unsafe {
            factory.create_image_view(
                image.clone(),
                ImageViewInfo {
                    view_kind: self.view_kind,
                    format: self.format,
                    swizzle: self.swizzle,
                    range: gfx_hal::image::SubresourceRange {
                        aspects: self.format.surface_desc().aspects,
                        levels: 0..1,
                        layers: 0..self.kind.num_layers(),
                    },
                },
            )
        }?;

        let sampler = factory.get_sampler(gfx_hal::image::SamplerInfo::new(
            self.filter,
            gfx_hal::image::WrapMode::Clamp,
        ))?;

        Ok(Texture {
            image,
            view,
            sampler,
        })
    }
}
