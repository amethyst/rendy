//! Module for creating a `Texture` from an image
use {
    crate::{
        core::{cast_cow, cast_slice},
        factory::{Factory, ImageState, UploadError},
        memory::Data,
        pixel::AsPixel,
        resource::{
            Escape, Handle, Image, ImageCreationError, ImageInfo, ImageView,
            ImageViewCreationError, ImageViewInfo, Sampler,
        },
    },
    rendy_core::hal::{
        format::{Component, Format, Swizzle},
        image, Backend,
    },
    std::num::NonZeroU8,
    thread_profiler::profile_scope,
};

/// Static image.
/// Can be loaded from various of formats.
#[derive(Debug)]
pub struct Texture<B: Backend> {
    image: Handle<Image<B>>,
    view: Escape<ImageView<B>>,
    sampler: Handle<Sampler<B>>,
    premultiplied: bool,
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

    /// Get whether texture has premultiplied alpha
    pub fn premultiplied_alpha(&self) -> bool {
        self.premultiplied
    }
}

/// Number of mip levels
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MipLevels {
    /// Generate mip levels automaticaly from image size, each mip level
    /// decreasing in resolution by half until 1x1
    GenerateAuto,
    /// Generate mip levels up to a certain level, each mip level
    /// decreasing in resolution by half
    GenerateLevels(NonZeroU8),
    /// Create the image with raw mip levels but without blitting the main
    /// texture data into them
    Levels(NonZeroU8),
}

/// Calculate the number of mip levels for a 2D image with given dimensions
pub fn mip_levels_from_dims(width: u32, height: u32) -> u8 {
    ((32 - width.max(height).leading_zeros()).max(1) as u8).min(rendy_core::hal::image::MAX_LEVEL)
}

#[derive(Debug)]
pub enum BuildError {
    Format(Format),
    Image(ImageCreationError),
    Upload(UploadError),
    ImageView(ImageViewCreationError),
    Mipmap(rendy_core::hal::device::OutOfMemory),
    Sampler(rendy_core::hal::device::AllocationError),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::Format(format) => write!(fmt, "Format unsupported: {:?}", format),
            BuildError::Image(err) => write!(fmt, "Texture build failed: {:?}", err),
            BuildError::Upload(err) => write!(fmt, "Texture build failed: {:?}", err),
            BuildError::ImageView(err) => write!(fmt, "Texture build failed: {:?}", err),
            BuildError::Mipmap(err) => write!(fmt, "Texture build failed: {:?}", err),
            BuildError::Sampler(err) => write!(fmt, "Texture build failed: {:?}", err),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BuildError::Format(_) => None,
            BuildError::Image(err) => Some(err),
            BuildError::Upload(err) => Some(err),
            BuildError::ImageView(err) => Some(err),
            BuildError::Mipmap(err) => Some(err),
            BuildError::Sampler(err) => Some(err),
        }
    }
}

/// Generics-free texture builder.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Struct for staging data in preparation of building a `Texture`
pub struct TextureBuilder<'a> {
    kind: image::Kind,
    view_kind: image::ViewKind,
    format: Format,
    data: std::borrow::Cow<'a, [u8]>,
    data_width: u32,
    data_height: u32,
    sampler_info: rendy_core::hal::image::SamplerDesc,
    swizzle: Swizzle,
    mip_levels: MipLevels,
    premultiplied: bool,
}

impl<'a> std::fmt::Debug for TextureBuilder<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("TextureBuilder")
            .field("kind", &self.kind)
            .field("view_kind", &self.view_kind)
            .field("format", &self.format)
            .field("data", &"<raw-data>")
            .field("data_width", &self.data_width)
            .field("data_height", &self.data_height)
            .field("sampler_info", &self.sampler_info)
            .field("swizzle", &self.swizzle)
            .field("mip_levels", &self.mip_levels)
            .field("premultiplied", &self.premultiplied)
            .finish()
    }
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
            sampler_info: rendy_core::hal::image::SamplerDesc::new(
                rendy_core::hal::image::Filter::Linear,
                rendy_core::hal::image::WrapMode::Clamp,
            ),
            swizzle: Swizzle::NO,
            mip_levels: MipLevels::Levels(NonZeroU8::new(1).unwrap()),
            premultiplied: false,
        }
    }

    /// Set whether the image has premultiplied alpha
    pub fn set_premultiplied_alpha(&mut self, premultiplied: bool) -> &mut Self {
        self.premultiplied = premultiplied;
        self
    }

    /// Set whether the image has premultiplied alpha
    pub fn with_premultiplied_alpha(mut self, premultiplied: bool) -> Self {
        self.set_premultiplied_alpha(premultiplied);
        self
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

    /// Set number of generated or raw mip levels
    pub fn with_mip_levels(mut self, mip_levels: MipLevels) -> Self {
        self.set_mip_levels(mip_levels);
        self
    }

    /// Set number of generated or raw mip levels
    pub fn set_mip_levels(&mut self, mip_levels: MipLevels) -> &mut Self {
        self.mip_levels = mip_levels;
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
    pub fn with_sampler_info(mut self, sampler_info: rendy_core::hal::image::SamplerDesc) -> Self {
        self.set_sampler_info(sampler_info);
        self
    }

    /// Set image sampler info.
    pub fn set_sampler_info(
        &mut self,
        sampler_info: rendy_core::hal::image::SamplerDesc,
    ) -> &mut Self {
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
    ) -> Result<Texture<B>, BuildError>
    where
        B: Backend,
    {
        profile_scope!("build");

        let view_caps = match self.view_kind {
            rendy_core::hal::image::ViewKind::D2Array => {
                rendy_core::hal::image::ViewCapabilities::KIND_2D_ARRAY
            }
            rendy_core::hal::image::ViewKind::Cube
            | rendy_core::hal::image::ViewKind::CubeArray => {
                rendy_core::hal::image::ViewCapabilities::KIND_CUBE
            }
            _ => rendy_core::hal::image::ViewCapabilities::empty(),
        };

        let (mip_levels, generate_mips) = match self.mip_levels {
            MipLevels::GenerateLevels(val) => (val.get(), true),
            MipLevels::Levels(val) => (val.get(), false),
            MipLevels::GenerateAuto => match self.kind {
                rendy_core::hal::image::Kind::D1(_, _) => (1, false),
                rendy_core::hal::image::Kind::D2(w, h, _, _) => (mip_levels_from_dims(w, h), true),
                rendy_core::hal::image::Kind::D3(_, _, _) => (1, false),
            },
        };

        let (info, transform, transform_swizzle) = find_compatible_format(
            factory,
            ImageInfo {
                kind: self.kind,
                levels: mip_levels,
                format: self.format,
                tiling: rendy_core::hal::image::Tiling::Optimal,
                view_caps,
                usage: rendy_core::hal::image::Usage::SAMPLED
                    | rendy_core::hal::image::Usage::TRANSFER_DST
                    | rendy_core::hal::image::Usage::TRANSFER_SRC,
            },
        )
        .ok_or(BuildError::Format(self.format))?;

        let image: Handle<Image<B>> = factory
            .create_image(info, Data)
            .map_err(BuildError::Image)?
            .into();

        let mut transformed_vec: Vec<u8>;

        let buffer: &[u8] = match transform {
            BufferTransform::Intact => &self.data,
            BufferTransform::AddPadding { stride, padding } => {
                profile_scope!("add_padding");
                let new_stride = stride + padding.len();
                let data_len = self.data.len() / stride * new_stride;

                transformed_vec = vec![0; data_len];
                let dst_slice: &mut [u8] = &mut transformed_vec;
                // optimize most common cases
                match (stride, padding) {
                    (2, &[0u8, std::u8::MAX]) => {
                        buf_add_padding(&self.data, dst_slice, stride, padding)
                    }
                    (3, &[std::u8::MAX]) => buf_add_padding(&self.data, dst_slice, stride, padding),
                    _ => buf_add_padding(&self.data, dst_slice, stride, padding),
                }
                &transformed_vec
            }
        };

        let mip_state = ImageState {
            queue: next_state.queue,
            stage: rendy_core::hal::pso::PipelineStage::TRANSFER,
            access: image::Access::TRANSFER_READ,
            layout: image::Layout::TransferSrcOptimal,
        };

        let undef_state = ImageState {
            queue: next_state.queue,
            stage: rendy_core::hal::pso::PipelineStage::TOP_OF_PIPE,
            access: image::Access::empty(),
            layout: image::Layout::Undefined,
        };

        // The reason that factory.upload_image is unsafe is that the image being uploaded
        // must have been created by the same factory and that it is not in use; we guarantee
        // that here because we just created the image on the same factory right before.
        unsafe {
            profile_scope!("upload_image");

            factory
                .upload_image(
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
                    if !generate_mips || mip_levels == 1 {
                        next_state
                    } else {
                        mip_state
                    },
                )
                .map_err(BuildError::Upload)?;
        }

        if mip_levels > 1 && generate_mips {
            profile_scope!("fill_mips");
            unsafe {
                factory
                    .blitter()
                    .fill_mips(
                        factory.device(),
                        image.clone(),
                        image::Filter::Linear,
                        std::iter::once(mip_state).chain(std::iter::repeat(undef_state)),
                        std::iter::repeat(next_state),
                    )
                    .map_err(BuildError::Mipmap)?;
            }
        } else if mip_levels > 1 && !generate_mips {
            unsafe {
                factory.transition_image(
                    image.clone(),
                    image::SubresourceRange {
                        aspects: info.format.surface_desc().aspects,
                        levels: 1..mip_levels,
                        layers: 0..info.kind.num_layers(),
                    },
                    image::Layout::Undefined,
                    next_state,
                );
            }
        }

        let view = {
            profile_scope!("create_image_view");
            factory
                .create_image_view(
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
                )
                .map_err(BuildError::ImageView)?
        };

        let sampler = factory
            .get_sampler(self.sampler_info.clone())
            .map_err(BuildError::Sampler)?;

        Ok(Texture {
            image,
            view,
            sampler,
            premultiplied: self.premultiplied,
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
    profile_scope!("find_compatible_format");

    if let Some(info) = image_format_supported(factory, info) {
        return Some((info, BufferTransform::Intact, Swizzle::NO));
    }
    if let Some((format, transform, swizzle)) = expand_format_channels(info.format) {
        let mut new_info = info;
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
        Format::Rg8Snorm => (Format::Rgba8Snorm, t2_u8, rgba),
        Format::Rg8Uscaled => (Format::Rgba8Uscaled, t2_u8, rgba),
        Format::Rg8Sscaled => (Format::Rgba8Sscaled, t2_u8, rgba),
        Format::Rg8Uint => (Format::Rgba8Uint, t2_u8, rgba),
        Format::Rg8Sint => (Format::Rgba8Sint, t2_u8, rgba),
        Format::Rg8Srgb => (Format::Rgba8Srgb, t2_u8, rgba),

        Format::Rgb8Unorm => (Format::Rgba8Unorm, t3_u8, rgba),
        Format::Rgb8Snorm => (Format::Rgba8Snorm, t3_u8, rgba),
        Format::Rgb8Uscaled => (Format::Rgba8Uscaled, t3_u8, rgba),
        Format::Rgb8Sscaled => (Format::Rgba8Sscaled, t3_u8, rgba),
        Format::Rgb8Uint => (Format::Rgba8Uint, t3_u8, rgba),
        Format::Rgb8Sint => (Format::Rgba8Sint, t3_u8, rgba),
        Format::Rgb8Srgb => (Format::Rgba8Srgb, t3_u8, rgba),

        Format::Bgr8Unorm => (Format::Rgba8Unorm, t3_u8, bgra),
        Format::Bgr8Snorm => (Format::Rgba8Snorm, t3_u8, bgra),
        Format::Bgr8Uscaled => (Format::Rgba8Uscaled, t3_u8, bgra),
        Format::Bgr8Sscaled => (Format::Rgba8Sscaled, t3_u8, bgra),
        Format::Bgr8Uint => (Format::Rgba8Uint, t3_u8, bgra),
        Format::Bgr8Sint => (Format::Rgba8Sint, t3_u8, bgra),
        Format::Bgr8Srgb => (Format::Rgba8Srgb, t3_u8, bgra),

        Format::Bgra8Unorm => (Format::Rgba8Unorm, intact, bgra),
        Format::Bgra8Snorm => (Format::Rgba8Snorm, intact, bgra),
        Format::Bgra8Uscaled => (Format::Rgba8Uscaled, intact, bgra),
        Format::Bgra8Sscaled => (Format::Rgba8Sscaled, intact, bgra),
        Format::Bgra8Uint => (Format::Rgba8Uint, intact, bgra),
        Format::Bgra8Sint => (Format::Rgba8Sint, intact, bgra),
        Format::Bgra8Srgb => (Format::Rgba8Srgb, intact, bgra),

        Format::Rg16Unorm => (Format::Rgba16Unorm, t2_u16, rgba),
        Format::Rg16Snorm => (Format::Rgba16Snorm, t2_u16, rgba),
        Format::Rg16Uscaled => (Format::Rgba16Uscaled, t2_u16, rgba),
        Format::Rg16Sscaled => (Format::Rgba16Sscaled, t2_u16, rgba),
        Format::Rg16Uint => (Format::Rgba16Uint, t2_u16, rgba),
        Format::Rg16Sint => (Format::Rgba16Sint, t2_u16, rgba),
        Format::Rg16Sfloat => (Format::Rgba16Sfloat, t2_f16, rgba),

        Format::Rgb16Unorm => (Format::Rgba16Unorm, t3_u16, rgba),
        Format::Rgb16Snorm => (Format::Rgba16Snorm, t3_u16, rgba),
        Format::Rgb16Uscaled => (Format::Rgba16Uscaled, t3_u16, rgba),
        Format::Rgb16Sscaled => (Format::Rgba16Sscaled, t3_u16, rgba),
        Format::Rgb16Uint => (Format::Rgba16Uint, t3_u16, rgba),
        Format::Rgb16Sint => (Format::Rgba16Sint, t3_u16, rgba),
        Format::Rgb16Sfloat => (Format::Rgba16Sfloat, t3_f16, rgba),

        Format::Rg32Uint => (Format::Rgba32Uint, t2_u32, rgba),
        Format::Rg32Sint => (Format::Rgba32Sint, t2_u32, rgba),
        Format::Rg32Sfloat => (Format::Rgba32Sfloat, t2_f32, rgba),

        Format::Rgb32Uint => (Format::Rgba32Uint, t3_u32, rgba),
        Format::Rgb32Sint => (Format::Rgba32Sint, t3_u32, rgba),
        Format::Rgb32Sfloat => (Format::Rgba32Sfloat, t3_f32, rgba),
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
                        new_samples >>= 1;
                    }
                    *s = new_samples;
                }
                _ => {}
            };
            info
        })
}

#[inline(always)]
fn buf_add_padding(buffer: &[u8], dst_slice: &mut [u8], stride: usize, padding: &'static [u8]) {
    let lad_len = padding.len();
    for (chunk, dst_chunk) in buffer
        .chunks_exact(stride)
        .zip(dst_slice.chunks_exact_mut(stride + lad_len))
    {
        // those loops gets unrolled in special-cased scenarios
        for i in 0..stride {
            dst_chunk[i] = chunk[i];
        }
        for i in 0..lad_len {
            dst_chunk[stride + i] = padding[i];
        }
    }
}
