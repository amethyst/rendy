use std::borrow::Borrow;
use std::marker::PhantomData;

use hal;
use hal::Device as HalDevice;

use buffer;
use device::Device;
use error;
use image;
use memory;
use MemoryRequirements;

impl From<hal::device::BindError> for error::BindError {
    fn from(e: hal::device::BindError) -> Self {
        match e {
            hal::device::BindError::WrongMemory => error::BindError::WrongMemory,
            hal::device::BindError::OutOfBounds => error::BindError::OutOfBounds,
        }
    }
}

impl From<hal::image::CreationError> for error::ImageCreationError {
    fn from(e: hal::image::CreationError) -> Self {
        use error::ImageCreationError;
        use hal::image::CreationError as HalCreationError;
        match e {
            HalCreationError::Format(f) => ImageCreationError::UnsupportedFormat(f.into()),
            HalCreationError::Kind => ImageCreationError::Kind,
            HalCreationError::Samples(n) => {
                ImageCreationError::Samples(image::SampleCountFlags::from_bits(n.into()).unwrap())
            }
            HalCreationError::Size(s) => ImageCreationError::UnsupportedSize(s),
            HalCreationError::Data(_) => ImageCreationError::DataSizeMismatch,
            HalCreationError::Usage(u) => ImageCreationError::UnsupportedUsage(
                image::UsageFlags::from_bits(u.bits()).unwrap(),
            ),
        }
    }
}

impl From<hal::format::Format> for image::Format {
    fn from(f: hal::format::Format) -> Self {
        use hal::format::Format::*;
        use image::Format;
        match f {
            __NumFormats => panic!(),
            Rg4Unorm => Format::R4G4_UNORM_PACK8,
            Rgba4Unorm => Format::R4G4B4A4_UNORM_PACK16,
            Bgra4Unorm => Format::B4G4R4A4_UNORM_PACK16,
            R5g6b5Unorm => Format::R5G6B5_UNORM_PACK16,
            B5g6r5Unorm => Format::B5G6R5_UNORM_PACK16,
            R5g5b5a1Unorm => Format::R5G5B5A1_UNORM_PACK16,
            B5g5r5a1Unorm => Format::B5G5R5A1_UNORM_PACK16,
            A1r5g5b5Unorm => Format::A1R5G5B5_UNORM_PACK16,
            R8Unorm => Format::R8_UNORM,
            R8Inorm => Format::R8_SNORM,
            R8Uscaled => Format::R8_USCALED,
            R8Iscaled => Format::R8_SSCALED,
            R8Uint => Format::R8_UINT,
            R8Int => Format::R8_SINT,
            R8Srgb => Format::R8_SRGB,
            Rg8Unorm => Format::R8G8_UNORM,
            Rg8Inorm => Format::R8G8_SNORM,
            Rg8Uscaled => Format::R8G8_USCALED,
            Rg8Iscaled => Format::R8G8_SSCALED,
            Rg8Uint => Format::R8G8_UINT,
            Rg8Int => Format::R8G8_SINT,
            Rg8Srgb => Format::R8G8_SRGB,
            Rgb8Unorm => Format::R8G8B8_UNORM,
            Rgb8Inorm => Format::R8G8B8_SNORM,
            Rgb8Uscaled => Format::R8G8B8_USCALED,
            Rgb8Iscaled => Format::R8G8B8_SSCALED,
            Rgb8Uint => Format::R8G8B8_UINT,
            Rgb8Int => Format::R8G8B8_SINT,
            Rgb8Srgb => Format::R8G8B8_SRGB,
            Bgr8Unorm => Format::B8G8R8_UNORM,
            Bgr8Inorm => Format::B8G8R8_SNORM,
            Bgr8Uscaled => Format::B8G8R8_USCALED,
            Bgr8Iscaled => Format::B8G8R8_SSCALED,
            Bgr8Uint => Format::B8G8R8_UINT,
            Bgr8Int => Format::B8G8R8_SINT,
            Bgr8Srgb => Format::B8G8R8_SRGB,
            Rgba8Unorm => Format::R8G8B8A8_UNORM,
            Rgba8Inorm => Format::R8G8B8A8_SNORM,
            Rgba8Uscaled => Format::R8G8B8A8_USCALED,
            Rgba8Iscaled => Format::R8G8B8A8_SSCALED,
            Rgba8Uint => Format::R8G8B8A8_UINT,
            Rgba8Int => Format::R8G8B8A8_SINT,
            Rgba8Srgb => Format::R8G8B8A8_SRGB,
            Bgra8Unorm => Format::B8G8R8A8_UNORM,
            Bgra8Inorm => Format::B8G8R8A8_SNORM,
            Bgra8Uscaled => Format::B8G8R8A8_USCALED,
            Bgra8Iscaled => Format::B8G8R8A8_SSCALED,
            Bgra8Uint => Format::B8G8R8A8_UINT,
            Bgra8Int => Format::B8G8R8A8_SINT,
            Bgra8Srgb => Format::B8G8R8A8_SRGB,
            Abgr8Unorm => Format::A8B8G8R8_UNORM_PACK32,
            Abgr8Inorm => Format::A8B8G8R8_SNORM_PACK32,
            Abgr8Uscaled => Format::A8B8G8R8_USCALED_PACK32,
            Abgr8Iscaled => Format::A8B8G8R8_SSCALED_PACK32,
            Abgr8Uint => Format::A8B8G8R8_UINT_PACK32,
            Abgr8Int => Format::A8B8G8R8_SINT_PACK32,
            Abgr8Srgb => Format::A8B8G8R8_SRGB_PACK32,
            A2r10g10b10Unorm => Format::A2R10G10B10_UNORM_PACK32,
            A2r10g10b10Inorm => Format::A2R10G10B10_SNORM_PACK32,
            A2r10g10b10Uscaled => Format::A2R10G10B10_USCALED_PACK32,
            A2r10g10b10Iscaled => Format::A2R10G10B10_SSCALED_PACK32,
            A2r10g10b10Uint => Format::A2R10G10B10_UINT_PACK32,
            A2r10g10b10Int => Format::A2R10G10B10_SINT_PACK32,
            A2b10g10r10Unorm => Format::A2B10G10R10_UNORM_PACK32,
            A2b10g10r10Inorm => Format::A2B10G10R10_SNORM_PACK32,
            A2b10g10r10Uscaled => Format::A2B10G10R10_USCALED_PACK32,
            A2b10g10r10Iscaled => Format::A2B10G10R10_SSCALED_PACK32,
            A2b10g10r10Uint => Format::A2B10G10R10_UINT_PACK32,
            A2b10g10r10Int => Format::A2B10G10R10_SINT_PACK32,
            R16Unorm => Format::R16_UNORM,
            R16Inorm => Format::R16_SNORM,
            R16Uscaled => Format::R16_USCALED,
            R16Iscaled => Format::R16_SSCALED,
            R16Uint => Format::R16_UINT,
            R16Int => Format::R16_SINT,
            R16Float => Format::R16_SFLOAT,
            Rg16Unorm => Format::R16G16_UNORM,
            Rg16Inorm => Format::R16G16_SNORM,
            Rg16Uscaled => Format::R16G16_USCALED,
            Rg16Iscaled => Format::R16G16_SSCALED,
            Rg16Uint => Format::R16G16_UINT,
            Rg16Int => Format::R16G16_SINT,
            Rg16Float => Format::R16G16_SFLOAT,
            Rgb16Unorm => Format::R16G16B16_UNORM,
            Rgb16Inorm => Format::R16G16B16_SNORM,
            Rgb16Uscaled => Format::R16G16B16_USCALED,
            Rgb16Iscaled => Format::R16G16B16_SSCALED,
            Rgb16Uint => Format::R16G16B16_UINT,
            Rgb16Int => Format::R16G16B16_SINT,
            Rgb16Float => Format::R16G16B16_SFLOAT,
            Rgba16Unorm => Format::R16G16B16A16_UNORM,
            Rgba16Inorm => Format::R16G16B16A16_SNORM,
            Rgba16Uscaled => Format::R16G16B16A16_USCALED,
            Rgba16Iscaled => Format::R16G16B16A16_SSCALED,
            Rgba16Uint => Format::R16G16B16A16_UINT,
            Rgba16Int => Format::R16G16B16A16_SINT,
            Rgba16Float => Format::R16G16B16A16_SFLOAT,
            R32Uint => Format::R32_UINT,
            R32Int => Format::R32_SINT,
            R32Float => Format::R32_SFLOAT,
            Rg32Uint => Format::R32G32_UINT,
            Rg32Int => Format::R32G32_SINT,
            Rg32Float => Format::R32G32_SFLOAT,
            Rgb32Uint => Format::R32G32B32_UINT,
            Rgb32Int => Format::R32G32B32_SINT,
            Rgb32Float => Format::R32G32B32_SFLOAT,
            Rgba32Uint => Format::R32G32B32A32_UINT,
            Rgba32Int => Format::R32G32B32A32_SINT,
            Rgba32Float => Format::R32G32B32A32_SFLOAT,
            R64Uint => Format::R64_UINT,
            R64Int => Format::R64_SINT,
            R64Float => Format::R64_SFLOAT,
            Rg64Uint => Format::R64G64_UINT,
            Rg64Int => Format::R64G64_SINT,
            Rg64Float => Format::R64G64_SFLOAT,
            Rgb64Uint => Format::R64G64B64_UINT,
            Rgb64Int => Format::R64G64B64_SINT,
            Rgb64Float => Format::R64G64B64_SFLOAT,
            Rgba64Uint => Format::R64G64B64A64_UINT,
            Rgba64Int => Format::R64G64B64A64_SINT,
            Rgba64Float => Format::R64G64B64A64_SFLOAT,
            B10g11r11Ufloat => Format::B10G11R11_UFLOAT_PACK32,
            E5b9g9r9Ufloat => Format::E5B9G9R9_UFLOAT_PACK32,
            D16Unorm => Format::D16_UNORM,
            X8D24Unorm => Format::X8_D24_UNORM_PACK32,
            D32Float => Format::D32_SFLOAT,
            S8Uint => Format::S8_UINT,
            D16UnormS8Uint => Format::D16_UNORM_S8_UINT,
            D24UnormS8Uint => Format::D24_UNORM_S8_UINT,
            D32FloatS8Uint => Format::D32_SFLOAT_S8_UINT,
            Bc1RgbUnorm => Format::BC1_RGB_UNORM_BLOCK,
            Bc1RgbSrgb => Format::BC1_RGB_SRGB_BLOCK,
            Bc1RgbaUnorm => Format::BC1_RGBA_UNORM_BLOCK,
            Bc1RgbaSrgb => Format::BC1_RGBA_SRGB_BLOCK,
            Bc2Unorm => Format::BC2_UNORM_BLOCK,
            Bc2Srgb => Format::BC2_SRGB_BLOCK,
            Bc3Unorm => Format::BC3_UNORM_BLOCK,
            Bc3Srgb => Format::BC3_SRGB_BLOCK,
            Bc4Unorm => Format::BC4_UNORM_BLOCK,
            Bc4Inorm => Format::BC4_SNORM_BLOCK,
            Bc5Unorm => Format::BC5_UNORM_BLOCK,
            Bc5Inorm => Format::BC5_SNORM_BLOCK,
            Bc6hUfloat => Format::BC6H_UFLOAT_BLOCK,
            Bc6hFloat => Format::BC6H_SFLOAT_BLOCK,
            Bc7Unorm => Format::BC7_UNORM_BLOCK,
            Bc7Srgb => Format::BC7_SRGB_BLOCK,
            Etc2R8g8b8Unorm => Format::ETC2_R8G8B8_UNORM_BLOCK,
            Etc2R8g8b8Srgb => Format::ETC2_R8G8B8_SRGB_BLOCK,
            Etc2R8g8b8a1Unorm => Format::ETC2_R8G8B8A1_UNORM_BLOCK,
            Etc2R8g8b8a1Srgb => Format::ETC2_R8G8B8A1_SRGB_BLOCK,
            Etc2R8g8b8a8Unorm => Format::ETC2_R8G8B8A8_UNORM_BLOCK,
            Etc2R8g8b8a8Srgb => Format::ETC2_R8G8B8A8_SRGB_BLOCK,
            EacR11Unorm => Format::EAC_R11_UNORM_BLOCK,
            EacR11Inorm => Format::EAC_R11_SNORM_BLOCK,
            EacR11g11Unorm => Format::EAC_R11G11_UNORM_BLOCK,
            EacR11g11Inorm => Format::EAC_R11G11_SNORM_BLOCK,
            Astc4x4Unorm => Format::ASTC_4x4_UNORM_BLOCK,
            Astc4x4Srgb => Format::ASTC_4x4_SRGB_BLOCK,
            Astc5x4Unorm => Format::ASTC_5x4_UNORM_BLOCK,
            Astc5x4Srgb => Format::ASTC_5x4_SRGB_BLOCK,
            Astc5x5Unorm => Format::ASTC_5x5_UNORM_BLOCK,
            Astc5x5Srgb => Format::ASTC_5x5_SRGB_BLOCK,
            Astc6x5Unorm => Format::ASTC_6x5_UNORM_BLOCK,
            Astc6x5Srgb => Format::ASTC_6x5_SRGB_BLOCK,
            Astc6x6Unorm => Format::ASTC_6x6_UNORM_BLOCK,
            Astc6x6Srgb => Format::ASTC_6x6_SRGB_BLOCK,
            Astc8x5Unorm => Format::ASTC_8x5_UNORM_BLOCK,
            Astc8x5Srgb => Format::ASTC_8x5_SRGB_BLOCK,
            Astc8x6Unorm => Format::ASTC_8x6_UNORM_BLOCK,
            Astc8x6Srgb => Format::ASTC_8x6_SRGB_BLOCK,
            Astc8x8Unorm => Format::ASTC_8x8_UNORM_BLOCK,
            Astc8x8Srgb => Format::ASTC_8x8_SRGB_BLOCK,
            Astc10x5Unorm => Format::ASTC_10x5_UNORM_BLOCK,
            Astc10x5Srgb => Format::ASTC_10x5_SRGB_BLOCK,
            Astc10x6Unorm => Format::ASTC_10x6_UNORM_BLOCK,
            Astc10x6Srgb => Format::ASTC_10x6_SRGB_BLOCK,
            Astc10x8Unorm => Format::ASTC_10x8_UNORM_BLOCK,
            Astc10x8Srgb => Format::ASTC_10x8_SRGB_BLOCK,
            Astc10x10Unorm => Format::ASTC_10x10_UNORM_BLOCK,
            Astc10x10Srgb => Format::ASTC_10x10_SRGB_BLOCK,
            Astc12x10Unorm => Format::ASTC_12x10_UNORM_BLOCK,
            Astc12x10Srgb => Format::ASTC_12x10_SRGB_BLOCK,
            Astc12x12Unorm => Format::ASTC_12x12_UNORM_BLOCK,
            Astc12x12Srgb => Format::ASTC_12x12_SRGB_BLOCK,
        }
    }
}

impl From<image::Format> for hal::format::Format {
    fn from(f: image::Format) -> Self {
        use hal::format::Format::*;
        use image::Format;
        match f {
            Format::UNDEFINED => panic!("Attempt to use undefined format"),
            Format::R4G4_UNORM_PACK8 => Rg4Unorm,
            Format::R4G4B4A4_UNORM_PACK16 => Rgba4Unorm,
            Format::B4G4R4A4_UNORM_PACK16 => Bgra4Unorm,
            Format::R5G6B5_UNORM_PACK16 => R5g6b5Unorm,
            Format::B5G6R5_UNORM_PACK16 => B5g6r5Unorm,
            Format::R5G5B5A1_UNORM_PACK16 => R5g5b5a1Unorm,
            Format::B5G5R5A1_UNORM_PACK16 => B5g5r5a1Unorm,
            Format::A1R5G5B5_UNORM_PACK16 => A1r5g5b5Unorm,
            Format::R8_UNORM => R8Unorm,
            Format::R8_SNORM => R8Inorm,
            Format::R8_USCALED => R8Uscaled,
            Format::R8_SSCALED => R8Iscaled,
            Format::R8_UINT => R8Uint,
            Format::R8_SINT => R8Int,
            Format::R8_SRGB => R8Srgb,
            Format::R8G8_UNORM => Rg8Unorm,
            Format::R8G8_SNORM => Rg8Inorm,
            Format::R8G8_USCALED => Rg8Uscaled,
            Format::R8G8_SSCALED => Rg8Iscaled,
            Format::R8G8_UINT => Rg8Uint,
            Format::R8G8_SINT => Rg8Int,
            Format::R8G8_SRGB => Rg8Srgb,
            Format::R8G8B8_UNORM => Rgb8Unorm,
            Format::R8G8B8_SNORM => Rgb8Inorm,
            Format::R8G8B8_USCALED => Rgb8Uscaled,
            Format::R8G8B8_SSCALED => Rgb8Iscaled,
            Format::R8G8B8_UINT => Rgb8Uint,
            Format::R8G8B8_SINT => Rgb8Int,
            Format::R8G8B8_SRGB => Rgb8Srgb,
            Format::B8G8R8_UNORM => Bgr8Unorm,
            Format::B8G8R8_SNORM => Bgr8Inorm,
            Format::B8G8R8_USCALED => Bgr8Uscaled,
            Format::B8G8R8_SSCALED => Bgr8Iscaled,
            Format::B8G8R8_UINT => Bgr8Uint,
            Format::B8G8R8_SINT => Bgr8Int,
            Format::B8G8R8_SRGB => Bgr8Srgb,
            Format::R8G8B8A8_UNORM => Rgba8Unorm,
            Format::R8G8B8A8_SNORM => Rgba8Inorm,
            Format::R8G8B8A8_USCALED => Rgba8Uscaled,
            Format::R8G8B8A8_SSCALED => Rgba8Iscaled,
            Format::R8G8B8A8_UINT => Rgba8Uint,
            Format::R8G8B8A8_SINT => Rgba8Int,
            Format::R8G8B8A8_SRGB => Rgba8Srgb,
            Format::B8G8R8A8_UNORM => Bgra8Unorm,
            Format::B8G8R8A8_SNORM => Bgra8Inorm,
            Format::B8G8R8A8_USCALED => Bgra8Uscaled,
            Format::B8G8R8A8_SSCALED => Bgra8Iscaled,
            Format::B8G8R8A8_UINT => Bgra8Uint,
            Format::B8G8R8A8_SINT => Bgra8Int,
            Format::B8G8R8A8_SRGB => Bgra8Srgb,
            Format::A8B8G8R8_UNORM_PACK32 => Abgr8Unorm,
            Format::A8B8G8R8_SNORM_PACK32 => Abgr8Inorm,
            Format::A8B8G8R8_USCALED_PACK32 => Abgr8Uscaled,
            Format::A8B8G8R8_SSCALED_PACK32 => Abgr8Iscaled,
            Format::A8B8G8R8_UINT_PACK32 => Abgr8Uint,
            Format::A8B8G8R8_SINT_PACK32 => Abgr8Int,
            Format::A8B8G8R8_SRGB_PACK32 => Abgr8Srgb,
            Format::A2R10G10B10_UNORM_PACK32 => A2r10g10b10Unorm,
            Format::A2R10G10B10_SNORM_PACK32 => A2r10g10b10Inorm,
            Format::A2R10G10B10_USCALED_PACK32 => A2r10g10b10Uscaled,
            Format::A2R10G10B10_SSCALED_PACK32 => A2r10g10b10Iscaled,
            Format::A2R10G10B10_UINT_PACK32 => A2r10g10b10Uint,
            Format::A2R10G10B10_SINT_PACK32 => A2r10g10b10Int,
            Format::A2B10G10R10_UNORM_PACK32 => A2b10g10r10Unorm,
            Format::A2B10G10R10_SNORM_PACK32 => A2b10g10r10Inorm,
            Format::A2B10G10R10_USCALED_PACK32 => A2b10g10r10Uscaled,
            Format::A2B10G10R10_SSCALED_PACK32 => A2b10g10r10Iscaled,
            Format::A2B10G10R10_UINT_PACK32 => A2b10g10r10Uint,
            Format::A2B10G10R10_SINT_PACK32 => A2b10g10r10Int,
            Format::R16_UNORM => R16Unorm,
            Format::R16_SNORM => R16Inorm,
            Format::R16_USCALED => R16Uscaled,
            Format::R16_SSCALED => R16Iscaled,
            Format::R16_UINT => R16Uint,
            Format::R16_SINT => R16Int,
            Format::R16_SFLOAT => R16Float,
            Format::R16G16_UNORM => Rg16Unorm,
            Format::R16G16_SNORM => Rg16Inorm,
            Format::R16G16_USCALED => Rg16Uscaled,
            Format::R16G16_SSCALED => Rg16Iscaled,
            Format::R16G16_UINT => Rg16Uint,
            Format::R16G16_SINT => Rg16Int,
            Format::R16G16_SFLOAT => Rg16Float,
            Format::R16G16B16_UNORM => Rgb16Unorm,
            Format::R16G16B16_SNORM => Rgb16Inorm,
            Format::R16G16B16_USCALED => Rgb16Uscaled,
            Format::R16G16B16_SSCALED => Rgb16Iscaled,
            Format::R16G16B16_UINT => Rgb16Uint,
            Format::R16G16B16_SINT => Rgb16Int,
            Format::R16G16B16_SFLOAT => Rgb16Float,
            Format::R16G16B16A16_UNORM => Rgba16Unorm,
            Format::R16G16B16A16_SNORM => Rgba16Inorm,
            Format::R16G16B16A16_USCALED => Rgba16Uscaled,
            Format::R16G16B16A16_SSCALED => Rgba16Iscaled,
            Format::R16G16B16A16_UINT => Rgba16Uint,
            Format::R16G16B16A16_SINT => Rgba16Int,
            Format::R16G16B16A16_SFLOAT => Rgba16Float,
            Format::R32_UINT => R32Uint,
            Format::R32_SINT => R32Int,
            Format::R32_SFLOAT => R32Float,
            Format::R32G32_UINT => Rg32Uint,
            Format::R32G32_SINT => Rg32Int,
            Format::R32G32_SFLOAT => Rg32Float,
            Format::R32G32B32_UINT => Rgb32Uint,
            Format::R32G32B32_SINT => Rgb32Int,
            Format::R32G32B32_SFLOAT => Rgb32Float,
            Format::R32G32B32A32_UINT => Rgba32Uint,
            Format::R32G32B32A32_SINT => Rgba32Int,
            Format::R32G32B32A32_SFLOAT => Rgba32Float,
            Format::R64_UINT => R64Uint,
            Format::R64_SINT => R64Int,
            Format::R64_SFLOAT => R64Float,
            Format::R64G64_UINT => Rg64Uint,
            Format::R64G64_SINT => Rg64Int,
            Format::R64G64_SFLOAT => Rg64Float,
            Format::R64G64B64_UINT => Rgb64Uint,
            Format::R64G64B64_SINT => Rgb64Int,
            Format::R64G64B64_SFLOAT => Rgb64Float,
            Format::R64G64B64A64_UINT => Rgba64Uint,
            Format::R64G64B64A64_SINT => Rgba64Int,
            Format::R64G64B64A64_SFLOAT => Rgba64Float,
            Format::B10G11R11_UFLOAT_PACK32 => B10g11r11Ufloat,
            Format::E5B9G9R9_UFLOAT_PACK32 => E5b9g9r9Ufloat,
            Format::D16_UNORM => D16Unorm,
            Format::X8_D24_UNORM_PACK32 => X8D24Unorm,
            Format::D32_SFLOAT => D32Float,
            Format::S8_UINT => S8Uint,
            Format::D16_UNORM_S8_UINT => D16UnormS8Uint,
            Format::D24_UNORM_S8_UINT => D24UnormS8Uint,
            Format::D32_SFLOAT_S8_UINT => D32FloatS8Uint,
            Format::BC1_RGB_UNORM_BLOCK => Bc1RgbUnorm,
            Format::BC1_RGB_SRGB_BLOCK => Bc1RgbSrgb,
            Format::BC1_RGBA_UNORM_BLOCK => Bc1RgbaUnorm,
            Format::BC1_RGBA_SRGB_BLOCK => Bc1RgbaSrgb,
            Format::BC2_UNORM_BLOCK => Bc2Unorm,
            Format::BC2_SRGB_BLOCK => Bc2Srgb,
            Format::BC3_UNORM_BLOCK => Bc3Unorm,
            Format::BC3_SRGB_BLOCK => Bc3Srgb,
            Format::BC4_UNORM_BLOCK => Bc4Unorm,
            Format::BC4_SNORM_BLOCK => Bc4Inorm,
            Format::BC5_UNORM_BLOCK => Bc5Unorm,
            Format::BC5_SNORM_BLOCK => Bc5Inorm,
            Format::BC6H_UFLOAT_BLOCK => Bc6hUfloat,
            Format::BC6H_SFLOAT_BLOCK => Bc6hFloat,
            Format::BC7_UNORM_BLOCK => Bc7Unorm,
            Format::BC7_SRGB_BLOCK => Bc7Srgb,
            Format::ETC2_R8G8B8_UNORM_BLOCK => Etc2R8g8b8Unorm,
            Format::ETC2_R8G8B8_SRGB_BLOCK => Etc2R8g8b8Srgb,
            Format::ETC2_R8G8B8A1_UNORM_BLOCK => Etc2R8g8b8a1Unorm,
            Format::ETC2_R8G8B8A1_SRGB_BLOCK => Etc2R8g8b8a1Srgb,
            Format::ETC2_R8G8B8A8_UNORM_BLOCK => Etc2R8g8b8a8Unorm,
            Format::ETC2_R8G8B8A8_SRGB_BLOCK => Etc2R8g8b8a8Srgb,
            Format::EAC_R11_UNORM_BLOCK => EacR11Unorm,
            Format::EAC_R11_SNORM_BLOCK => EacR11Inorm,
            Format::EAC_R11G11_UNORM_BLOCK => EacR11g11Unorm,
            Format::EAC_R11G11_SNORM_BLOCK => EacR11g11Inorm,
            Format::ASTC_4x4_UNORM_BLOCK => Astc4x4Unorm,
            Format::ASTC_4x4_SRGB_BLOCK => Astc4x4Srgb,
            Format::ASTC_5x4_UNORM_BLOCK => Astc5x4Unorm,
            Format::ASTC_5x4_SRGB_BLOCK => Astc5x4Srgb,
            Format::ASTC_5x5_UNORM_BLOCK => Astc5x5Unorm,
            Format::ASTC_5x5_SRGB_BLOCK => Astc5x5Srgb,
            Format::ASTC_6x5_UNORM_BLOCK => Astc6x5Unorm,
            Format::ASTC_6x5_SRGB_BLOCK => Astc6x5Srgb,
            Format::ASTC_6x6_UNORM_BLOCK => Astc6x6Unorm,
            Format::ASTC_6x6_SRGB_BLOCK => Astc6x6Srgb,
            Format::ASTC_8x5_UNORM_BLOCK => Astc8x5Unorm,
            Format::ASTC_8x5_SRGB_BLOCK => Astc8x5Srgb,
            Format::ASTC_8x6_UNORM_BLOCK => Astc8x6Unorm,
            Format::ASTC_8x6_SRGB_BLOCK => Astc8x6Srgb,
            Format::ASTC_8x8_UNORM_BLOCK => Astc8x8Unorm,
            Format::ASTC_8x8_SRGB_BLOCK => Astc8x8Srgb,
            Format::ASTC_10x5_UNORM_BLOCK => Astc10x5Unorm,
            Format::ASTC_10x5_SRGB_BLOCK => Astc10x5Srgb,
            Format::ASTC_10x6_UNORM_BLOCK => Astc10x6Unorm,
            Format::ASTC_10x6_SRGB_BLOCK => Astc10x6Srgb,
            Format::ASTC_10x8_UNORM_BLOCK => Astc10x8Unorm,
            Format::ASTC_10x8_SRGB_BLOCK => Astc10x8Srgb,
            Format::ASTC_10x10_UNORM_BLOCK => Astc10x10Unorm,
            Format::ASTC_10x10_SRGB_BLOCK => Astc10x10Srgb,
            Format::ASTC_12x10_UNORM_BLOCK => Astc12x10Unorm,
            Format::ASTC_12x10_SRGB_BLOCK => Astc12x10Srgb,
            Format::ASTC_12x12_UNORM_BLOCK => Astc12x12Unorm,
            Format::ASTC_12x12_SRGB_BLOCK => Astc12x12Srgb,
            _ => panic!("Format {:?} isn't supported by the hal backend", f),
        }
    }
}

impl<D, B> Device for (D, PhantomData<B>)
where
    B: hal::Backend,
    D: Borrow<B::Device>,
{
    type Sampler = B::Sampler;
    type Buffer = B::Buffer;
    type UnboundBuffer = B::UnboundBuffer;
    type BufferView = B::BufferView;
    type Image = B::Image;
    type UnboundImage = B::UnboundImage;
    type ImageView = B::ImageView;

    fn create_buffer(
        &self,
        info: buffer::CreateInfo,
    ) -> Result<Self::UnboundBuffer, memory::OutOfMemoryError> {
        let usage = hal::buffer::Usage::from_bits(info.usage.bits()).unwrap();
        self.0
            .borrow()
            .create_buffer(info.size, usage)
            .map_err(|e| {
                use hal::buffer::CreationError;
                match e {
                    CreationError::OutOfHostMemory => memory::OutOfMemoryError::OutOfHostMemory,
                    CreationError::OutOfDeviceMemory => memory::OutOfMemoryError::OutOfDeviceMemory,
                    CreationError::UnsupportedUsage { .. } => {
                        panic!("Backend doesn't support this usage")
                    }
                }
            })
    }

    fn buffer_requirements(&self, buffer: &Self::UnboundBuffer) -> MemoryRequirements {
        let req = self.0.borrow().get_buffer_requirements(buffer);
        MemoryRequirements {
            size: req.size,
            align: req.alignment,
            mask: req.type_mask as u32,
        }
    }

    unsafe fn bind_buffer(
        &self,
        buffer: Self::UnboundBuffer,
        memory: &Self::Memory,
        offset: u64,
    ) -> Result<Self::Buffer, error::BindError> {
        Ok(self.0.borrow().bind_buffer_memory(memory, offset, buffer)?)
    }

    unsafe fn destroy_buffer(&self, buffer: Self::Buffer) {
        self.0.borrow().destroy_buffer(buffer);
    }

    fn create_image(
        &self,
        info: image::CreateInfo,
    ) -> Result<Self::UnboundImage, error::ImageCreationError> {
        let kind = match info.kind {
            image::Kind::D1 => hal::image::Kind::D1(info.extent.width, info.array as u16),
            image::Kind::D2 => hal::image::Kind::D2(
                info.extent.width,
                info.extent.height,
                info.array as u16,
                info.samples.bits() as u8,
            ),
            image::Kind::D3 => {
                hal::image::Kind::D3(info.extent.width, info.extent.height, info.extent.depth)
            }
        };
        let format = info.format.into();
        let tiling = match info.tiling {
            image::ImageTiling::Optimal => hal::image::Tiling::Optimal,
            image::ImageTiling::Linear => hal::image::Tiling::Linear,
        };
        let usage = hal::image::Usage::from_bits(info.usage.bits()).unwrap();
        let view_caps = hal::image::ViewCapabilities::from_bits(info.flags.bits()).unwrap();

        Ok(self
            .0
            .borrow()
            .create_image(kind, info.mips as u8, format, tiling, usage, view_caps)?)
    }

    fn image_requirements(&self, image: &Self::UnboundImage) -> MemoryRequirements {
        let req = self.0.borrow().get_image_requirements(image);
        MemoryRequirements {
            size: req.size,
            align: req.alignment,
            mask: req.type_mask as u32,
        }
    }

    unsafe fn bind_image(
        &self,
        image: Self::UnboundImage,
        memory: &Self::Memory,
        offset: u64,
    ) -> Result<Self::Image, error::BindError> {
        Ok(self.0.borrow().bind_image_memory(memory, offset, image)?)
    }

    unsafe fn destroy_image(&self, image: Self::Image) {
        self.0.borrow().destroy_image(image);
    }
}
