use {
    crate::{
        factory::{Factory, ImageState},
        memory::Data,
        pixel::AsPixel,
        resource::{Escape, Handle, Image, ImageInfo, ImageView, ImageViewInfo, Sampler},
        util::cast_cow,
    },
    gfx_hal::{
        format::{Component, Format, Swizzle},
        image, Backend,
    },
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
    format: Format,
    data: std::borrow::Cow<'a, [u8]>,
    data_width: u32,
    data_height: u32,
    sampler_info: gfx_hal::image::SamplerInfo,
    swizzle: Swizzle,
}

impl<'a> TextureBuilder<'a> {
    /// New empty builder.
    pub fn new() -> Self {
        TextureBuilder {
            kind: image::Kind::D1(0, 0),
            view_kind: image::ViewKind::D1,
            format: Format::Rgba8Unorm,
            data: std::borrow::Cow::Borrowed(&[]),
            data_width: 0,
            data_height: 0,
            sampler_info: gfx_hal::image::SamplerInfo::new(
                gfx_hal::image::Filter::Linear,
                gfx_hal::image::WrapMode::Clamp,
            ),
            swizzle: Swizzle::NO,
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
        format: Format,
    ) -> Self {
        self.set_raw_data(data, format);
        self
    }

    /// Set pixel data with manual format definition.
    pub fn set_raw_data(
        &mut self,
        data: impl Into<std::borrow::Cow<'a, [u8]>>,
        format: Format,
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
    pub fn with_swizzle(mut self, swizzle: Swizzle) -> Self {
        self.set_swizzle(swizzle);
        self
    }

    /// Set swizzle.
    pub fn set_swizzle(&mut self, swizzle: Swizzle) -> &mut Self {
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
        let view_caps = match self.view_kind {
            gfx_hal::image::ViewKind::D2Array => gfx_hal::image::ViewCapabilities::KIND_2D_ARRAY,
            gfx_hal::image::ViewKind::Cube | gfx_hal::image::ViewKind::CubeArray => {
                gfx_hal::image::ViewCapabilities::KIND_CUBE
            }
            _ => gfx_hal::image::ViewCapabilities::empty(),
        };

        let (info, transform, transform_swizzle) = find_compatible_format(
            factory,
            ImageInfo {
                kind: self.kind,
                levels: 1,
                format: self.format,
                tiling: gfx_hal::image::Tiling::Optimal,
                view_caps,
                usage: gfx_hal::image::Usage::SAMPLED | gfx_hal::image::Usage::TRANSFER_DST,
            },
        )
        .ok_or_else(|| {
            failure::format_err!(
                "Format {:?} is not supported and no suitable conversion found.",
                self.format
            )
        })?;

        let image: Handle<Image<B>> = factory.create_image(info, Data)?.into();

        let mut transformed_vec: Vec<u8> = Vec::new();

        let buffer: &[u8] = match transform {
            BufferTransform::Intact => &self.data,
            BufferTransform::AddPadding { stride, padding } => {
                transformed_vec.reserve_exact(self.data.len() / stride * (stride + padding));
                transformed_vec.extend(self.data.chunks_exact(stride).flat_map(|chunk| {
                    chunk
                        .iter()
                        .cloned()
                        .chain(std::iter::repeat(0).take(padding))
                }));

                &transformed_vec
            }
        };

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
                buffer,
                image::Layout::Undefined,
                next_state,
            )?;
        }

        let view = factory.create_image_view(
            image.clone(),
            ImageViewInfo {
                view_kind: self.view_kind,
                format: info.format,
                swizzle: double_swizzle(self.swizzle, transform_swizzle),
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

enum BufferTransform {
    Intact,
    AddPadding { stride: usize, padding: usize },
}

fn double_swizzle(src: Swizzle, overlay: Swizzle) -> Swizzle {
    fn pick_component(src: Swizzle, component: Component) -> Component {
        let Swizzle(r, g, b, a) = src;
        match component {
            Component::R => r,
            Component::G => g,
            Component::B => b,
            Component::A => a,
            Component::Zero => Component::Zero,
            Component::One => Component::One,
        }
    }
    Swizzle(
        pick_component(src, overlay.0),
        pick_component(src, overlay.1),
        pick_component(src, overlay.2),
        pick_component(src, overlay.3),
    )
}

fn find_compatible_format<B: Backend>(
    factory: &Factory<B>,
    info: ImageInfo,
) -> Option<(ImageInfo, BufferTransform, Swizzle)> {
    if let Some(info) = image_format_supported(factory, info) {
        return Some((info, BufferTransform::Intact, Swizzle::NO));
    }
    if let Some((format, transform, swizzle)) = expand_format_channels(info.format) {
        let mut new_info = info.clone();
        new_info.format = format;
        if let Some(new_info) = image_format_supported(factory, new_info) {
            log::trace!("Converting image from {:?} to {:?}", info, new_info);
            return Some((new_info, transform, swizzle));
        }
    }

    None
}

fn expand_format_channels(format: Format) -> Option<(Format, BufferTransform, Swizzle)> {
    let t2to4_8 = BufferTransform::AddPadding {
        stride: 2,
        padding: 2,
    };

    let t2to4_16 = BufferTransform::AddPadding {
        stride: 4,
        padding: 4,
    };

    let t2to4_32 = BufferTransform::AddPadding {
        stride: 8,
        padding: 8,
    };

    let t3to4_8 = BufferTransform::AddPadding {
        stride: 3,
        padding: 1,
    };

    let t3to4_16 = BufferTransform::AddPadding {
        stride: 6,
        padding: 2,
    };

    let t3to4_32 = BufferTransform::AddPadding {
        stride: 12,
        padding: 4,
    };

    let rgzo = Swizzle(Component::R, Component::G, Component::Zero, Component::One);
    let rgbo = Swizzle(Component::R, Component::G, Component::B, Component::One);
    let bgro = Swizzle(Component::B, Component::G, Component::R, Component::One);
    let bgra = Swizzle(Component::B, Component::G, Component::R, Component::A);

    Some(match format {
        // Destination formats chosen according to this table
        // https://vulkan.gpuinfo.org/listformats.php
        Format::Rg8Unorm => (Format::Rgba8Unorm, t2to4_8, rgzo),
        Format::Rg8Inorm => (Format::Rgba8Inorm, t2to4_8, rgzo),
        Format::Rg8Uscaled => (Format::Rgba8Uscaled, t2to4_8, rgzo),
        Format::Rg8Iscaled => (Format::Rgba8Iscaled, t2to4_8, rgzo),
        Format::Rg8Uint => (Format::Rgba8Uint, t2to4_8, rgzo),
        Format::Rg8Int => (Format::Rgba8Int, t2to4_8, rgzo),
        Format::Rg8Srgb => (Format::Rgba8Srgb, t2to4_8, rgzo),

        Format::Rgb8Unorm => (Format::Rgba8Unorm, t3to4_8, rgbo),
        Format::Rgb8Inorm => (Format::Rgba8Inorm, t3to4_8, rgbo),
        Format::Rgb8Uscaled => (Format::Rgba8Uscaled, t3to4_8, rgbo),
        Format::Rgb8Iscaled => (Format::Rgba8Iscaled, t3to4_8, rgbo),
        Format::Rgb8Uint => (Format::Rgba8Uint, t3to4_8, rgbo),
        Format::Rgb8Int => (Format::Rgba8Int, t3to4_8, rgbo),
        Format::Rgb8Srgb => (Format::Rgba8Srgb, t3to4_8, rgbo),

        Format::Bgr8Unorm => (Format::Rgba8Unorm, t3to4_8, bgro),
        Format::Bgr8Inorm => (Format::Rgba8Inorm, t3to4_8, bgro),
        Format::Bgr8Uscaled => (Format::Rgba8Uscaled, t3to4_8, bgro),
        Format::Bgr8Iscaled => (Format::Rgba8Iscaled, t3to4_8, bgro),
        Format::Bgr8Uint => (Format::Rgba8Uint, t3to4_8, bgro),
        Format::Bgr8Int => (Format::Rgba8Int, t3to4_8, bgro),
        Format::Bgr8Srgb => (Format::Rgba8Srgb, t3to4_8, bgro),

        Format::Bgra8Unorm => (Format::Rgba8Unorm, t3to4_8, bgra),
        Format::Bgra8Inorm => (Format::Rgba8Inorm, t3to4_8, bgra),
        Format::Bgra8Uscaled => (Format::Rgba8Uscaled, t3to4_8, bgra),
        Format::Bgra8Iscaled => (Format::Rgba8Iscaled, t3to4_8, bgra),
        Format::Bgra8Uint => (Format::Rgba8Uint, t3to4_8, bgra),
        Format::Bgra8Int => (Format::Rgba8Int, t3to4_8, bgra),
        Format::Bgra8Srgb => (Format::Rgba8Srgb, t3to4_8, bgra),

        Format::Rg16Unorm => (Format::Rgba16Unorm, t2to4_16, rgzo),
        Format::Rg16Inorm => (Format::Rgba16Inorm, t2to4_16, rgzo),
        Format::Rg16Uscaled => (Format::Rgba16Uscaled, t2to4_16, rgzo),
        Format::Rg16Iscaled => (Format::Rgba16Iscaled, t2to4_16, rgzo),
        Format::Rg16Uint => (Format::Rgba16Uint, t2to4_16, rgzo),
        Format::Rg16Int => (Format::Rgba16Int, t2to4_16, rgzo),
        Format::Rg16Float => (Format::Rgba16Float, t2to4_16, rgzo),

        Format::Rgb16Unorm => (Format::Rgba16Unorm, t3to4_16, rgbo),
        Format::Rgb16Inorm => (Format::Rgba16Inorm, t3to4_16, rgbo),
        Format::Rgb16Uscaled => (Format::Rgba16Uscaled, t3to4_16, rgbo),
        Format::Rgb16Iscaled => (Format::Rgba16Iscaled, t3to4_16, rgbo),
        Format::Rgb16Uint => (Format::Rgba16Uint, t3to4_16, rgbo),
        Format::Rgb16Int => (Format::Rgba16Int, t3to4_16, rgbo),
        Format::Rgb16Float => (Format::Rgba16Float, t3to4_16, rgbo),

        Format::Rg32Uint => (Format::Rgba32Uint, t2to4_32, rgzo),
        Format::Rg32Int => (Format::Rgba32Int, t2to4_32, rgzo),
        Format::Rg32Float => (Format::Rgba32Float, t2to4_32, rgzo),

        Format::Rgb32Uint => (Format::Rgba32Uint, t3to4_32, rgbo),
        Format::Rgb32Int => (Format::Rgba32Int, t3to4_32, rgbo),
        Format::Rgb32Float => (Format::Rgba32Float, t3to4_32, rgbo),
        // TODO: add more conversions
        _ => return None,
    })
}

fn image_format_supported<B: Backend>(
    factory: &Factory<B>,
    mut info: ImageInfo,
) -> Option<ImageInfo> {
    factory
        .image_format_properties(info)
        .filter(|props| {
            props.max_layers >= info.kind.num_layers()
                && props.max_extent.width >= info.kind.extent().width
                && props.max_extent.height >= info.kind.extent().height
                && props.max_extent.depth >= info.kind.extent().depth
        })
        .map(|props| {
            match &mut info.kind {
                image::Kind::D2(_, _, _, s) if *s & props.sample_count_mask != *s => {
                    let mut new_samples = *s >> 1;
                    while new_samples > 1 && new_samples & props.sample_count_mask != new_samples {
                        new_samples = new_samples >> 1;
                    }
                    *s = new_samples;
                }
                _ => {}
            };
            info
        })
}
