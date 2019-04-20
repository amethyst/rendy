//! Module for creating a `Texture` from an image
use {
    crate::{
        factory::{Factory, ImageState},
        memory::Data,
        pixel::AsPixel,
        resource::{Escape, Handle, Image, ImageInfo, ImageView, ImageViewInfo, Sampler},
        util::{cast_cow, cast_slice},
    },
    gfx_hal::{
        format::{Component, Format, Swizzle},
        image, Backend,
    },
    std::num::NonZeroU8,
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

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MipLevel {
    Auto,
    Level(NonZeroU8),
}

/// Generics-free texture builder.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Struct for staging data in preparation of building a `Texture`
pub struct TextureBuilder<'a> {
    kind: image::Kind,
    view_kind: image::ViewKind,
    format: Format,
    data: std::borrow::Cow<'a, [u8]>,
    data_width: u32,
    data_height: u32,
    sampler_info: gfx_hal::image::SamplerInfo,
    swizzle: Swizzle,
    mip_level: MipLevel,
}

impl<'a> TextureBuilder<'a> {
    /// New empty `TextureBuilder`
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
            mip_level: MipLevel::Level(NonZeroU8::new(1).unwrap()),
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

    /// Set number of generated mipmaps.
    pub fn with_mip_levels(mut self, mip_level: MipLevel) -> Self {
        self.set_mip_levels(mip_level);
        self
    }

    /// Set number of generated mipmaps.
    pub fn set_mip_levels(&mut self, mip_level: MipLevel) -> &mut Self {
        self.mip_level = mip_level;
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

        let mip_levels = match self.mip_level {
            MipLevel::Level(val) => val.get(),
            MipLevel::Auto => match self.kind {
                gfx_hal::image::Kind::D1(_, _) => 1,
                gfx_hal::image::Kind::D2(w, h, _, _) => {
                    ((32 - w.max(h).leading_zeros()).max(1) as u8).min(gfx_hal::image::MAX_LEVEL)
                }
                gfx_hal::image::Kind::D3(_, _, _) => 1,
            },
        };

        let (info, transform, transform_swizzle) = find_compatible_format(
            factory,
            ImageInfo {
                kind: self.kind,
                levels: mip_levels,
                format: self.format,
                tiling: gfx_hal::image::Tiling::Optimal,
                view_caps,
                usage: gfx_hal::image::Usage::SAMPLED
                    | gfx_hal::image::Usage::TRANSFER_DST
                    | gfx_hal::image::Usage::TRANSFER_SRC,
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
                transformed_vec.reserve_exact(self.data.len() / stride * (stride + padding.len()));
                transformed_vec.extend(
                    self.data
                        .chunks_exact(stride)
                        .flat_map(|chunk| chunk.iter().cloned().chain(padding.iter().cloned())),
                );

                &transformed_vec
            }
        };

        let mip_state = ImageState {
            queue: next_state.queue,
            stage: gfx_hal::pso::PipelineStage::TRANSFER,
            access: image::Access::TRANSFER_READ,
            layout: image::Layout::TransferSrcOptimal,
        };

        let undef_state = ImageState {
            queue: next_state.queue,
            stage: gfx_hal::pso::PipelineStage::TOP_OF_PIPE,
            access: image::Access::empty(),
            layout: image::Layout::Undefined,
        };

        // The reason that factory.upload_image is unsafe is that the image being uploaded
        // must have been created by the same factory and that it is not in use; we guarantee
        // that here because we just created the image on the same factory right before.
        unsafe {
            factory.upload_image(
                image.clone(),
                self.data_width,
                self.data_height,
                image::SubresourceLayers {
                    aspects: info.format.surface_desc().aspects,
                    level: 0,
                    layers: 0..info.kind.num_layers(),
                },
                image::Offset::ZERO,
                info.kind.extent(),
                buffer,
                image::Layout::Undefined,
                if mip_levels == 1 {
                    next_state
                } else {
                    mip_state
                },
            )?;
        }

        if mip_levels > 1 {
            unsafe {
                factory.blitter().fill_mips(
                    factory.device(),
                    image.clone(),
                    image::Filter::Linear,
                    std::iter::once(mip_state).chain(std::iter::repeat(undef_state)),
                    std::iter::repeat(next_state),
                )?;
            }
        }

        let view = factory.create_image_view(
            image.clone(),
            ImageViewInfo {
                view_kind: self.view_kind,
                format: info.format,
                swizzle: double_swizzle(self.swizzle, transform_swizzle),
                range: image::SubresourceRange {
                    aspects: info.format.surface_desc().aspects,
                    levels: 0..info.levels,
                    layers: 0..info.kind.num_layers(),
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
    AddPadding {
        stride: usize,
        padding: &'static [u8],
    },
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
    const ONE_F16: u16 = 15360u16;

    let t2_u8 = BufferTransform::AddPadding {
        stride: 2,
        padding: &[0u8, std::u8::MAX],
    };

    let t2_u16 = BufferTransform::AddPadding {
        stride: 4,
        padding: cast_slice(&[0u16, std::u16::MAX]),
    };

    let t2_f16 = BufferTransform::AddPadding {
        stride: 4,
        padding: cast_slice(&[0u16, ONE_F16]),
    };

    let t2_u32 = BufferTransform::AddPadding {
        stride: 8,
        padding: cast_slice(&[0u32, std::u32::MAX]),
    };

    let t2_f32 = BufferTransform::AddPadding {
        stride: 8,
        padding: cast_slice(&[0.0f32, 1.0f32]),
    };

    let t3_u8 = BufferTransform::AddPadding {
        stride: 3,
        padding: &[std::u8::MAX],
    };

    let t3_u16 = BufferTransform::AddPadding {
        stride: 6,
        padding: cast_slice(&[std::u16::MAX]),
    };

    let t3_f16 = BufferTransform::AddPadding {
        stride: 6,
        padding: cast_slice(&[ONE_F16]),
    };

    let t3_u32 = BufferTransform::AddPadding {
        stride: 12,
        padding: cast_slice(&[std::u32::MAX]),
    };

    let t3_f32 = BufferTransform::AddPadding {
        stride: 12,
        padding: cast_slice(&[1.0f32]),
    };

    let intact = BufferTransform::Intact;

    let rgba = Swizzle(Component::R, Component::G, Component::B, Component::A);
    let bgra = Swizzle(Component::B, Component::G, Component::R, Component::A);

    Some(match format {
        // Destination formats chosen according to this table
        // https://vulkan.gpuinfo.org/listformats.php
        Format::Rg8Unorm => (Format::Rgba8Unorm, t2_u8, rgba),
        Format::Rg8Inorm => (Format::Rgba8Inorm, t2_u8, rgba),
        Format::Rg8Uscaled => (Format::Rgba8Uscaled, t2_u8, rgba),
        Format::Rg8Iscaled => (Format::Rgba8Iscaled, t2_u8, rgba),
        Format::Rg8Uint => (Format::Rgba8Uint, t2_u8, rgba),
        Format::Rg8Int => (Format::Rgba8Int, t2_u8, rgba),
        Format::Rg8Srgb => (Format::Rgba8Srgb, t2_u8, rgba),

        Format::Rgb8Unorm => (Format::Rgba8Unorm, t3_u8, rgba),
        Format::Rgb8Inorm => (Format::Rgba8Inorm, t3_u8, rgba),
        Format::Rgb8Uscaled => (Format::Rgba8Uscaled, t3_u8, rgba),
        Format::Rgb8Iscaled => (Format::Rgba8Iscaled, t3_u8, rgba),
        Format::Rgb8Uint => (Format::Rgba8Uint, t3_u8, rgba),
        Format::Rgb8Int => (Format::Rgba8Int, t3_u8, rgba),
        Format::Rgb8Srgb => (Format::Rgba8Srgb, t3_u8, rgba),

        Format::Bgr8Unorm => (Format::Rgba8Unorm, t3_u8, bgra),
        Format::Bgr8Inorm => (Format::Rgba8Inorm, t3_u8, bgra),
        Format::Bgr8Uscaled => (Format::Rgba8Uscaled, t3_u8, bgra),
        Format::Bgr8Iscaled => (Format::Rgba8Iscaled, t3_u8, bgra),
        Format::Bgr8Uint => (Format::Rgba8Uint, t3_u8, bgra),
        Format::Bgr8Int => (Format::Rgba8Int, t3_u8, bgra),
        Format::Bgr8Srgb => (Format::Rgba8Srgb, t3_u8, bgra),

        Format::Bgra8Unorm => (Format::Rgba8Unorm, intact, bgra),
        Format::Bgra8Inorm => (Format::Rgba8Inorm, intact, bgra),
        Format::Bgra8Uscaled => (Format::Rgba8Uscaled, intact, bgra),
        Format::Bgra8Iscaled => (Format::Rgba8Iscaled, intact, bgra),
        Format::Bgra8Uint => (Format::Rgba8Uint, intact, bgra),
        Format::Bgra8Int => (Format::Rgba8Int, intact, bgra),
        Format::Bgra8Srgb => (Format::Rgba8Srgb, intact, bgra),

        Format::Rg16Unorm => (Format::Rgba16Unorm, t2_u16, rgba),
        Format::Rg16Inorm => (Format::Rgba16Inorm, t2_u16, rgba),
        Format::Rg16Uscaled => (Format::Rgba16Uscaled, t2_u16, rgba),
        Format::Rg16Iscaled => (Format::Rgba16Iscaled, t2_u16, rgba),
        Format::Rg16Uint => (Format::Rgba16Uint, t2_u16, rgba),
        Format::Rg16Int => (Format::Rgba16Int, t2_u16, rgba),
        Format::Rg16Float => (Format::Rgba16Float, t2_f16, rgba),

        Format::Rgb16Unorm => (Format::Rgba16Unorm, t3_u16, rgba),
        Format::Rgb16Inorm => (Format::Rgba16Inorm, t3_u16, rgba),
        Format::Rgb16Uscaled => (Format::Rgba16Uscaled, t3_u16, rgba),
        Format::Rgb16Iscaled => (Format::Rgba16Iscaled, t3_u16, rgba),
        Format::Rgb16Uint => (Format::Rgba16Uint, t3_u16, rgba),
        Format::Rgb16Int => (Format::Rgba16Int, t3_u16, rgba),
        Format::Rgb16Float => (Format::Rgba16Float, t3_f16, rgba),

        Format::Rg32Uint => (Format::Rgba32Uint, t2_u32, rgba),
        Format::Rg32Int => (Format::Rgba32Int, t2_u32, rgba),
        Format::Rg32Float => (Format::Rgba32Float, t2_f32, rgba),

        Format::Rgb32Uint => (Format::Rgba32Uint, t3_u32, rgba),
        Format::Rgb32Int => (Format::Rgba32Int, t3_u32, rgba),
        Format::Rgb32Float => (Format::Rgba32Float, t3_f32, rgba),
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
