use std::ptr;

use ash::{
    self,
    version::{DeviceV1_0, FunctionPointers},
    vk,
};

use buffer;
use device::Device;
use error;
use image;
use memory;
use MemoryRequirements;
use SharingMode;

impl From<vk::Format> for image::Format {
    fn from(f: vk::Format) -> Self {
        use self::vk::Format::*;
        use image::Format;
        match f {
            Undefined => Format::UNDEFINED,
            R4g4UnormPack8 => Format::R4G4_UNORM_PACK8,
            R4g4b4a4UnormPack16 => Format::R4G4B4A4_UNORM_PACK16,
            B4g4r4a4UnormPack16 => Format::B4G4R4A4_UNORM_PACK16,
            R5g6b5UnormPack16 => Format::R5G6B5_UNORM_PACK16,
            B5g6r5UnormPack16 => Format::B5G6R5_UNORM_PACK16,
            R5g5b5a1UnormPack16 => Format::R5G5B5A1_UNORM_PACK16,
            B5g5r5a1UnormPack16 => Format::B5G5R5A1_UNORM_PACK16,
            A1r5g5b5UnormPack16 => Format::A1R5G5B5_UNORM_PACK16,
            R8Unorm => Format::R8_UNORM,
            R8Snorm => Format::R8_SNORM,
            R8Uscaled => Format::R8_USCALED,
            R8Sscaled => Format::R8_SSCALED,
            R8Uint => Format::R8_UINT,
            R8Sint => Format::R8_SINT,
            R8Srgb => Format::R8_SRGB,
            R8g8Unorm => Format::R8G8_UNORM,
            R8g8Snorm => Format::R8G8_SNORM,
            R8g8Uscaled => Format::R8G8_USCALED,
            R8g8Sscaled => Format::R8G8_SSCALED,
            R8g8Uint => Format::R8G8_UINT,
            R8g8Sint => Format::R8G8_SINT,
            R8g8Srgb => Format::R8G8_SRGB,
            R8g8b8Unorm => Format::R8G8B8_UNORM,
            R8g8b8Snorm => Format::R8G8B8_SNORM,
            R8g8b8Uscaled => Format::R8G8B8_USCALED,
            R8g8b8Sscaled => Format::R8G8B8_SSCALED,
            R8g8b8Uint => Format::R8G8B8_UINT,
            R8g8b8Sint => Format::R8G8B8_SINT,
            R8g8b8Srgb => Format::R8G8B8_SRGB,
            B8g8r8Unorm => Format::B8G8R8_UNORM,
            B8g8r8Snorm => Format::B8G8R8_SNORM,
            B8g8r8Uscaled => Format::B8G8R8_USCALED,
            B8g8r8Sscaled => Format::B8G8R8_SSCALED,
            B8g8r8Uint => Format::B8G8R8_UINT,
            B8g8r8Sint => Format::B8G8R8_SINT,
            B8g8r8Srgb => Format::B8G8R8_SRGB,
            R8g8b8a8Unorm => Format::R8G8B8A8_UNORM,
            R8g8b8a8Snorm => Format::R8G8B8A8_SNORM,
            R8g8b8a8Uscaled => Format::R8G8B8A8_USCALED,
            R8g8b8a8Sscaled => Format::R8G8B8A8_SSCALED,
            R8g8b8a8Uint => Format::R8G8B8A8_UINT,
            R8g8b8a8Sint => Format::R8G8B8A8_SINT,
            R8g8b8a8Srgb => Format::R8G8B8A8_SRGB,
            B8g8r8a8Unorm => Format::B8G8R8A8_UNORM,
            B8g8r8a8Snorm => Format::B8G8R8A8_SNORM,
            B8g8r8a8Uscaled => Format::B8G8R8A8_USCALED,
            B8g8r8a8Sscaled => Format::B8G8R8A8_SSCALED,
            B8g8r8a8Uint => Format::B8G8R8A8_UINT,
            B8g8r8a8Sint => Format::B8G8R8A8_SINT,
            B8g8r8a8Srgb => Format::B8G8R8A8_SRGB,
            A8b8g8r8UnormPack32 => Format::A8B8G8R8_UNORM_PACK32,
            A8b8g8r8SnormPack32 => Format::A8B8G8R8_SNORM_PACK32,
            A8b8g8r8UscaledPack32 => Format::A8B8G8R8_USCALED_PACK32,
            A8b8g8r8SscaledPack32 => Format::A8B8G8R8_SSCALED_PACK32,
            A8b8g8r8UintPack32 => Format::A8B8G8R8_UINT_PACK32,
            A8b8g8r8SintPack32 => Format::A8B8G8R8_SINT_PACK32,
            A8b8g8r8SrgbPack32 => Format::A8B8G8R8_SRGB_PACK32,
            A2r10g10b10UnormPack32 => Format::A2R10G10B10_UNORM_PACK32,
            A2r10g10b10SnormPack32 => Format::A2R10G10B10_SNORM_PACK32,
            A2r10g10b10UscaledPack32 => Format::A2R10G10B10_USCALED_PACK32,
            A2r10g10b10SscaledPack32 => Format::A2R10G10B10_SSCALED_PACK32,
            A2r10g10b10UintPack32 => Format::A2R10G10B10_UINT_PACK32,
            A2r10g10b10SintPack32 => Format::A2R10G10B10_SINT_PACK32,
            A2b10g10r10UnormPack32 => Format::A2B10G10R10_UNORM_PACK32,
            A2b10g10r10SnormPack32 => Format::A2B10G10R10_SNORM_PACK32,
            A2b10g10r10UscaledPack32 => Format::A2B10G10R10_USCALED_PACK32,
            A2b10g10r10SscaledPack32 => Format::A2B10G10R10_SSCALED_PACK32,
            A2b10g10r10UintPack32 => Format::A2B10G10R10_UINT_PACK32,
            A2b10g10r10SintPack32 => Format::A2B10G10R10_SINT_PACK32,
            R16Unorm => Format::R16_UNORM,
            R16Snorm => Format::R16_SNORM,
            R16Uscaled => Format::R16_USCALED,
            R16Sscaled => Format::R16_SSCALED,
            R16Uint => Format::R16_UINT,
            R16Sint => Format::R16_SINT,
            R16Sfloat => Format::R16_SFLOAT,
            R16g16Unorm => Format::R16G16_UNORM,
            R16g16Snorm => Format::R16G16_SNORM,
            R16g16Uscaled => Format::R16G16_USCALED,
            R16g16Sscaled => Format::R16G16_SSCALED,
            R16g16Uint => Format::R16G16_UINT,
            R16g16Sint => Format::R16G16_SINT,
            R16g16Sfloat => Format::R16G16_SFLOAT,
            R16g16b16Unorm => Format::R16G16B16_UNORM,
            R16g16b16Snorm => Format::R16G16B16_SNORM,
            R16g16b16Uscaled => Format::R16G16B16_USCALED,
            R16g16b16Sscaled => Format::R16G16B16_SSCALED,
            R16g16b16Uint => Format::R16G16B16_UINT,
            R16g16b16Sint => Format::R16G16B16_SINT,
            R16g16b16Sfloat => Format::R16G16B16_SFLOAT,
            R16g16b16a16Unorm => Format::R16G16B16A16_UNORM,
            R16g16b16a16Snorm => Format::R16G16B16A16_SNORM,
            R16g16b16a16Uscaled => Format::R16G16B16A16_USCALED,
            R16g16b16a16Sscaled => Format::R16G16B16A16_SSCALED,
            R16g16b16a16Uint => Format::R16G16B16A16_UINT,
            R16g16b16a16Sint => Format::R16G16B16A16_SINT,
            R16g16b16a16Sfloat => Format::R16G16B16A16_SFLOAT,
            R32Uint => Format::R32_UINT,
            R32Sint => Format::R32_SINT,
            R32Sfloat => Format::R32_SFLOAT,
            R32g32Uint => Format::R32G32_UINT,
            R32g32Sint => Format::R32G32_SINT,
            R32g32Sfloat => Format::R32G32_SFLOAT,
            R32g32b32Uint => Format::R32G32B32_UINT,
            R32g32b32Sint => Format::R32G32B32_SINT,
            R32g32b32Sfloat => Format::R32G32B32_SFLOAT,
            R32g32b32a32Uint => Format::R32G32B32A32_UINT,
            R32g32b32a32Sint => Format::R32G32B32A32_SINT,
            R32g32b32a32Sfloat => Format::R32G32B32A32_SFLOAT,
            R64Uint => Format::R64_UINT,
            R64Sint => Format::R64_SINT,
            R64Sfloat => Format::R64_SFLOAT,
            R64g64Uint => Format::R64G64_UINT,
            R64g64Sint => Format::R64G64_SINT,
            R64g64Sfloat => Format::R64G64_SFLOAT,
            R64g64b64Uint => Format::R64G64B64_UINT,
            R64g64b64Sint => Format::R64G64B64_SINT,
            R64g64b64Sfloat => Format::R64G64B64_SFLOAT,
            R64g64b64a64Uint => Format::R64G64B64A64_UINT,
            R64g64b64a64Sint => Format::R64G64B64A64_SINT,
            R64g64b64a64Sfloat => Format::R64G64B64A64_SFLOAT,
            B10g11r11UfloatPack32 => Format::B10G11R11_UFLOAT_PACK32,
            E5b9g9r9UfloatPack32 => Format::E5B9G9R9_UFLOAT_PACK32,
            D16Unorm => Format::D16_UNORM,
            X8D24UnormPack32 => Format::X8_D24_UNORM_PACK32,
            D32Sfloat => Format::D32_SFLOAT,
            S8Uint => Format::S8_UINT,
            D16UnormS8Uint => Format::D16_UNORM_S8_UINT,
            D24UnormS8Uint => Format::D24_UNORM_S8_UINT,
            D32SfloatS8Uint => Format::D32_SFLOAT_S8_UINT,
            Bc1RgbUnormBlock => Format::BC1_RGB_UNORM_BLOCK,
            Bc1RgbSrgbBlock => Format::BC1_RGB_SRGB_BLOCK,
            Bc1RgbaUnormBlock => Format::BC1_RGBA_UNORM_BLOCK,
            Bc1RgbaSrgbBlock => Format::BC1_RGBA_SRGB_BLOCK,
            Bc2UnormBlock => Format::BC2_UNORM_BLOCK,
            Bc2SrgbBlock => Format::BC2_SRGB_BLOCK,
            Bc3UnormBlock => Format::BC3_UNORM_BLOCK,
            Bc3SrgbBlock => Format::BC3_SRGB_BLOCK,
            Bc4UnormBlock => Format::BC4_UNORM_BLOCK,
            Bc4SnormBlock => Format::BC4_SNORM_BLOCK,
            Bc5UnormBlock => Format::BC5_UNORM_BLOCK,
            Bc5SnormBlock => Format::BC5_SNORM_BLOCK,
            Bc6hUfloatBlock => Format::BC6H_UFLOAT_BLOCK,
            Bc6hSfloatBlock => Format::BC6H_SFLOAT_BLOCK,
            Bc7UnormBlock => Format::BC7_UNORM_BLOCK,
            Bc7SrgbBlock => Format::BC7_SRGB_BLOCK,
            Etc2R8g8b8UnormBlock => Format::ETC2_R8G8B8_UNORM_BLOCK,
            Etc2R8g8b8SrgbBlock => Format::ETC2_R8G8B8_SRGB_BLOCK,
            Etc2R8g8b8a1UnormBlock => Format::ETC2_R8G8B8A1_UNORM_BLOCK,
            Etc2R8g8b8a1SrgbBlock => Format::ETC2_R8G8B8A1_SRGB_BLOCK,
            Etc2R8g8b8a8UnormBlock => Format::ETC2_R8G8B8A8_UNORM_BLOCK,
            Etc2R8g8b8a8SrgbBlock => Format::ETC2_R8G8B8A8_SRGB_BLOCK,
            EacR11UnormBlock => Format::EAC_R11_UNORM_BLOCK,
            EacR11SnormBlock => Format::EAC_R11_SNORM_BLOCK,
            EacR11g11UnormBlock => Format::EAC_R11G11_UNORM_BLOCK,
            EacR11g11SnormBlock => Format::EAC_R11G11_SNORM_BLOCK,
            Astc4x4UnormBlock => Format::ASTC_4x4_UNORM_BLOCK,
            Astc4x4SrgbBlock => Format::ASTC_4x4_SRGB_BLOCK,
            Astc5x4UnormBlock => Format::ASTC_5x4_UNORM_BLOCK,
            Astc5x4SrgbBlock => Format::ASTC_5x4_SRGB_BLOCK,
            Astc5x5UnormBlock => Format::ASTC_5x5_UNORM_BLOCK,
            Astc5x5SrgbBlock => Format::ASTC_5x5_SRGB_BLOCK,
            Astc6x5UnormBlock => Format::ASTC_6x5_UNORM_BLOCK,
            Astc6x5SrgbBlock => Format::ASTC_6x5_SRGB_BLOCK,
            Astc6x6UnormBlock => Format::ASTC_6x6_UNORM_BLOCK,
            Astc6x6SrgbBlock => Format::ASTC_6x6_SRGB_BLOCK,
            Astc8x5UnormBlock => Format::ASTC_8x5_UNORM_BLOCK,
            Astc8x5SrgbBlock => Format::ASTC_8x5_SRGB_BLOCK,
            Astc8x6UnormBlock => Format::ASTC_8x6_UNORM_BLOCK,
            Astc8x6SrgbBlock => Format::ASTC_8x6_SRGB_BLOCK,
            Astc8x8UnormBlock => Format::ASTC_8x8_UNORM_BLOCK,
            Astc8x8SrgbBlock => Format::ASTC_8x8_SRGB_BLOCK,
            Astc10x5UnormBlock => Format::ASTC_10x5_UNORM_BLOCK,
            Astc10x5SrgbBlock => Format::ASTC_10x5_SRGB_BLOCK,
            Astc10x6UnormBlock => Format::ASTC_10x6_UNORM_BLOCK,
            Astc10x6SrgbBlock => Format::ASTC_10x6_SRGB_BLOCK,
            Astc10x8UnormBlock => Format::ASTC_10x8_UNORM_BLOCK,
            Astc10x8SrgbBlock => Format::ASTC_10x8_SRGB_BLOCK,
            Astc10x10UnormBlock => Format::ASTC_10x10_UNORM_BLOCK,
            Astc10x10SrgbBlock => Format::ASTC_10x10_SRGB_BLOCK,
            Astc12x10UnormBlock => Format::ASTC_12x10_UNORM_BLOCK,
            Astc12x10SrgbBlock => Format::ASTC_12x10_SRGB_BLOCK,
            Astc12x12UnormBlock => Format::ASTC_12x12_UNORM_BLOCK,
            Astc12x12SrgbBlock => Format::ASTC_12x12_SRGB_BLOCK,
        }
    }
}

impl From<image::Format> for vk::Format {
    fn from(f: image::Format) -> Self {
        use self::vk::Format::*;
        use image::Format;
        match f {
            Format::UNDEFINED => Undefined,
            Format::R4G4_UNORM_PACK8 => R4g4UnormPack8,
            Format::R4G4B4A4_UNORM_PACK16 => R4g4b4a4UnormPack16,
            Format::B4G4R4A4_UNORM_PACK16 => B4g4r4a4UnormPack16,
            Format::R5G6B5_UNORM_PACK16 => R5g6b5UnormPack16,
            Format::B5G6R5_UNORM_PACK16 => B5g6r5UnormPack16,
            Format::R5G5B5A1_UNORM_PACK16 => R5g5b5a1UnormPack16,
            Format::B5G5R5A1_UNORM_PACK16 => B5g5r5a1UnormPack16,
            Format::A1R5G5B5_UNORM_PACK16 => A1r5g5b5UnormPack16,
            Format::R8_UNORM => R8Unorm,
            Format::R8_SNORM => R8Snorm,
            Format::R8_USCALED => R8Uscaled,
            Format::R8_SSCALED => R8Sscaled,
            Format::R8_UINT => R8Uint,
            Format::R8_SINT => R8Sint,
            Format::R8_SRGB => R8Srgb,
            Format::R8G8_UNORM => R8g8Unorm,
            Format::R8G8_SNORM => R8g8Snorm,
            Format::R8G8_USCALED => R8g8Uscaled,
            Format::R8G8_SSCALED => R8g8Sscaled,
            Format::R8G8_UINT => R8g8Uint,
            Format::R8G8_SINT => R8g8Sint,
            Format::R8G8_SRGB => R8g8Srgb,
            Format::R8G8B8_UNORM => R8g8b8Unorm,
            Format::R8G8B8_SNORM => R8g8b8Snorm,
            Format::R8G8B8_USCALED => R8g8b8Uscaled,
            Format::R8G8B8_SSCALED => R8g8b8Sscaled,
            Format::R8G8B8_UINT => R8g8b8Uint,
            Format::R8G8B8_SINT => R8g8b8Sint,
            Format::R8G8B8_SRGB => R8g8b8Srgb,
            Format::B8G8R8_UNORM => B8g8r8Unorm,
            Format::B8G8R8_SNORM => B8g8r8Snorm,
            Format::B8G8R8_USCALED => B8g8r8Uscaled,
            Format::B8G8R8_SSCALED => B8g8r8Sscaled,
            Format::B8G8R8_UINT => B8g8r8Uint,
            Format::B8G8R8_SINT => B8g8r8Sint,
            Format::B8G8R8_SRGB => B8g8r8Srgb,
            Format::R8G8B8A8_UNORM => R8g8b8a8Unorm,
            Format::R8G8B8A8_SNORM => R8g8b8a8Snorm,
            Format::R8G8B8A8_USCALED => R8g8b8a8Uscaled,
            Format::R8G8B8A8_SSCALED => R8g8b8a8Sscaled,
            Format::R8G8B8A8_UINT => R8g8b8a8Uint,
            Format::R8G8B8A8_SINT => R8g8b8a8Sint,
            Format::R8G8B8A8_SRGB => R8g8b8a8Srgb,
            Format::B8G8R8A8_UNORM => B8g8r8a8Unorm,
            Format::B8G8R8A8_SNORM => B8g8r8a8Snorm,
            Format::B8G8R8A8_USCALED => B8g8r8a8Uscaled,
            Format::B8G8R8A8_SSCALED => B8g8r8a8Sscaled,
            Format::B8G8R8A8_UINT => B8g8r8a8Uint,
            Format::B8G8R8A8_SINT => B8g8r8a8Sint,
            Format::B8G8R8A8_SRGB => B8g8r8a8Srgb,
            Format::A8B8G8R8_UNORM_PACK32 => A8b8g8r8UnormPack32,
            Format::A8B8G8R8_SNORM_PACK32 => A8b8g8r8SnormPack32,
            Format::A8B8G8R8_USCALED_PACK32 => A8b8g8r8UscaledPack32,
            Format::A8B8G8R8_SSCALED_PACK32 => A8b8g8r8SscaledPack32,
            Format::A8B8G8R8_UINT_PACK32 => A8b8g8r8UintPack32,
            Format::A8B8G8R8_SINT_PACK32 => A8b8g8r8SintPack32,
            Format::A8B8G8R8_SRGB_PACK32 => A8b8g8r8SrgbPack32,
            Format::A2R10G10B10_UNORM_PACK32 => A2r10g10b10UnormPack32,
            Format::A2R10G10B10_SNORM_PACK32 => A2r10g10b10SnormPack32,
            Format::A2R10G10B10_USCALED_PACK32 => A2r10g10b10UscaledPack32,
            Format::A2R10G10B10_SSCALED_PACK32 => A2r10g10b10SscaledPack32,
            Format::A2R10G10B10_UINT_PACK32 => A2r10g10b10UintPack32,
            Format::A2R10G10B10_SINT_PACK32 => A2r10g10b10SintPack32,
            Format::A2B10G10R10_UNORM_PACK32 => A2b10g10r10UnormPack32,
            Format::A2B10G10R10_SNORM_PACK32 => A2b10g10r10SnormPack32,
            Format::A2B10G10R10_USCALED_PACK32 => A2b10g10r10UscaledPack32,
            Format::A2B10G10R10_SSCALED_PACK32 => A2b10g10r10SscaledPack32,
            Format::A2B10G10R10_UINT_PACK32 => A2b10g10r10UintPack32,
            Format::A2B10G10R10_SINT_PACK32 => A2b10g10r10SintPack32,
            Format::R16_UNORM => R16Unorm,
            Format::R16_SNORM => R16Snorm,
            Format::R16_USCALED => R16Uscaled,
            Format::R16_SSCALED => R16Sscaled,
            Format::R16_UINT => R16Uint,
            Format::R16_SINT => R16Sint,
            Format::R16_SFLOAT => R16Sfloat,
            Format::R16G16_UNORM => R16g16Unorm,
            Format::R16G16_SNORM => R16g16Snorm,
            Format::R16G16_USCALED => R16g16Uscaled,
            Format::R16G16_SSCALED => R16g16Sscaled,
            Format::R16G16_UINT => R16g16Uint,
            Format::R16G16_SINT => R16g16Sint,
            Format::R16G16_SFLOAT => R16g16Sfloat,
            Format::R16G16B16_UNORM => R16g16b16Unorm,
            Format::R16G16B16_SNORM => R16g16b16Snorm,
            Format::R16G16B16_USCALED => R16g16b16Uscaled,
            Format::R16G16B16_SSCALED => R16g16b16Sscaled,
            Format::R16G16B16_UINT => R16g16b16Uint,
            Format::R16G16B16_SINT => R16g16b16Sint,
            Format::R16G16B16_SFLOAT => R16g16b16Sfloat,
            Format::R16G16B16A16_UNORM => R16g16b16a16Unorm,
            Format::R16G16B16A16_SNORM => R16g16b16a16Snorm,
            Format::R16G16B16A16_USCALED => R16g16b16a16Uscaled,
            Format::R16G16B16A16_SSCALED => R16g16b16a16Sscaled,
            Format::R16G16B16A16_UINT => R16g16b16a16Uint,
            Format::R16G16B16A16_SINT => R16g16b16a16Sint,
            Format::R16G16B16A16_SFLOAT => R16g16b16a16Sfloat,
            Format::R32_UINT => R32Uint,
            Format::R32_SINT => R32Sint,
            Format::R32_SFLOAT => R32Sfloat,
            Format::R32G32_UINT => R32g32Uint,
            Format::R32G32_SINT => R32g32Sint,
            Format::R32G32_SFLOAT => R32g32Sfloat,
            Format::R32G32B32_UINT => R32g32b32Uint,
            Format::R32G32B32_SINT => R32g32b32Sint,
            Format::R32G32B32_SFLOAT => R32g32b32Sfloat,
            Format::R32G32B32A32_UINT => R32g32b32a32Uint,
            Format::R32G32B32A32_SINT => R32g32b32a32Sint,
            Format::R32G32B32A32_SFLOAT => R32g32b32a32Sfloat,
            Format::R64_UINT => R64Uint,
            Format::R64_SINT => R64Sint,
            Format::R64_SFLOAT => R64Sfloat,
            Format::R64G64_UINT => R64g64Uint,
            Format::R64G64_SINT => R64g64Sint,
            Format::R64G64_SFLOAT => R64g64Sfloat,
            Format::R64G64B64_UINT => R64g64b64Uint,
            Format::R64G64B64_SINT => R64g64b64Sint,
            Format::R64G64B64_SFLOAT => R64g64b64Sfloat,
            Format::R64G64B64A64_UINT => R64g64b64a64Uint,
            Format::R64G64B64A64_SINT => R64g64b64a64Sint,
            Format::R64G64B64A64_SFLOAT => R64g64b64a64Sfloat,
            Format::B10G11R11_UFLOAT_PACK32 => B10g11r11UfloatPack32,
            Format::E5B9G9R9_UFLOAT_PACK32 => E5b9g9r9UfloatPack32,
            Format::D16_UNORM => D16Unorm,
            Format::X8_D24_UNORM_PACK32 => X8D24UnormPack32,
            Format::D32_SFLOAT => D32Sfloat,
            Format::S8_UINT => S8Uint,
            Format::D16_UNORM_S8_UINT => D16UnormS8Uint,
            Format::D24_UNORM_S8_UINT => D24UnormS8Uint,
            Format::D32_SFLOAT_S8_UINT => D32SfloatS8Uint,
            Format::BC1_RGB_UNORM_BLOCK => Bc1RgbUnormBlock,
            Format::BC1_RGB_SRGB_BLOCK => Bc1RgbSrgbBlock,
            Format::BC1_RGBA_UNORM_BLOCK => Bc1RgbaUnormBlock,
            Format::BC1_RGBA_SRGB_BLOCK => Bc1RgbaSrgbBlock,
            Format::BC2_UNORM_BLOCK => Bc2UnormBlock,
            Format::BC2_SRGB_BLOCK => Bc2SrgbBlock,
            Format::BC3_UNORM_BLOCK => Bc3UnormBlock,
            Format::BC3_SRGB_BLOCK => Bc3SrgbBlock,
            Format::BC4_UNORM_BLOCK => Bc4UnormBlock,
            Format::BC4_SNORM_BLOCK => Bc4SnormBlock,
            Format::BC5_UNORM_BLOCK => Bc5UnormBlock,
            Format::BC5_SNORM_BLOCK => Bc5SnormBlock,
            Format::BC6H_UFLOAT_BLOCK => Bc6hUfloatBlock,
            Format::BC6H_SFLOAT_BLOCK => Bc6hSfloatBlock,
            Format::BC7_UNORM_BLOCK => Bc7UnormBlock,
            Format::BC7_SRGB_BLOCK => Bc7SrgbBlock,
            Format::ETC2_R8G8B8_UNORM_BLOCK => Etc2R8g8b8UnormBlock,
            Format::ETC2_R8G8B8_SRGB_BLOCK => Etc2R8g8b8SrgbBlock,
            Format::ETC2_R8G8B8A1_UNORM_BLOCK => Etc2R8g8b8a1UnormBlock,
            Format::ETC2_R8G8B8A1_SRGB_BLOCK => Etc2R8g8b8a1SrgbBlock,
            Format::ETC2_R8G8B8A8_UNORM_BLOCK => Etc2R8g8b8a8UnormBlock,
            Format::ETC2_R8G8B8A8_SRGB_BLOCK => Etc2R8g8b8a8SrgbBlock,
            Format::EAC_R11_UNORM_BLOCK => EacR11UnormBlock,
            Format::EAC_R11_SNORM_BLOCK => EacR11SnormBlock,
            Format::EAC_R11G11_UNORM_BLOCK => EacR11g11UnormBlock,
            Format::EAC_R11G11_SNORM_BLOCK => EacR11g11SnormBlock,
            Format::ASTC_4x4_UNORM_BLOCK => Astc4x4UnormBlock,
            Format::ASTC_4x4_SRGB_BLOCK => Astc4x4SrgbBlock,
            Format::ASTC_5x4_UNORM_BLOCK => Astc5x4UnormBlock,
            Format::ASTC_5x4_SRGB_BLOCK => Astc5x4SrgbBlock,
            Format::ASTC_5x5_UNORM_BLOCK => Astc5x5UnormBlock,
            Format::ASTC_5x5_SRGB_BLOCK => Astc5x5SrgbBlock,
            Format::ASTC_6x5_UNORM_BLOCK => Astc6x5UnormBlock,
            Format::ASTC_6x5_SRGB_BLOCK => Astc6x5SrgbBlock,
            Format::ASTC_6x6_UNORM_BLOCK => Astc6x6UnormBlock,
            Format::ASTC_6x6_SRGB_BLOCK => Astc6x6SrgbBlock,
            Format::ASTC_8x5_UNORM_BLOCK => Astc8x5UnormBlock,
            Format::ASTC_8x5_SRGB_BLOCK => Astc8x5SrgbBlock,
            Format::ASTC_8x6_UNORM_BLOCK => Astc8x6UnormBlock,
            Format::ASTC_8x6_SRGB_BLOCK => Astc8x6SrgbBlock,
            Format::ASTC_8x8_UNORM_BLOCK => Astc8x8UnormBlock,
            Format::ASTC_8x8_SRGB_BLOCK => Astc8x8SrgbBlock,
            Format::ASTC_10x5_UNORM_BLOCK => Astc10x5UnormBlock,
            Format::ASTC_10x5_SRGB_BLOCK => Astc10x5SrgbBlock,
            Format::ASTC_10x6_UNORM_BLOCK => Astc10x6UnormBlock,
            Format::ASTC_10x6_SRGB_BLOCK => Astc10x6SrgbBlock,
            Format::ASTC_10x8_UNORM_BLOCK => Astc10x8UnormBlock,
            Format::ASTC_10x8_SRGB_BLOCK => Astc10x8SrgbBlock,
            Format::ASTC_10x10_UNORM_BLOCK => Astc10x10UnormBlock,
            Format::ASTC_10x10_SRGB_BLOCK => Astc10x10SrgbBlock,
            Format::ASTC_12x10_UNORM_BLOCK => Astc12x10UnormBlock,
            Format::ASTC_12x10_SRGB_BLOCK => Astc12x10SrgbBlock,
            Format::ASTC_12x12_UNORM_BLOCK => Astc12x12UnormBlock,
            Format::ASTC_12x12_SRGB_BLOCK => Astc12x12SrgbBlock,
            _ => panic!("Format {:?} isn't supported by the hal backend", f),
        }
    }
}

impl<V> Device for ash::Device<V>
where
    V: FunctionPointers,
    ash::Device<V>: DeviceV1_0,
{
    type Sampler = vk::Sampler;
    type Buffer = vk::Buffer;
    type UnboundBuffer = vk::Buffer;
    type BufferView = vk::BufferView;
    type Image = vk::Image;
    type UnboundImage = vk::Image;
    type ImageView = vk::ImageView;

    fn create_buffer(
        &self,
        info: buffer::CreateInfo,
    ) -> Result<Self::UnboundBuffer, memory::OutOfMemoryError> {
        let info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BufferCreateInfo,
            p_next: ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: info.size,
            usage: vk::BufferUsageFlags::from_flags(info.usage.bits()).unwrap(),
            sharing_mode: match info.sharing {
                SharingMode::Exclusive => vk::SharingMode::Exclusive,
            },
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
        };

        unsafe { DeviceV1_0::create_buffer(self, &info, None) }.map_err(|e| {
            use self::vk::types::Result;
            match e {
                Result::ErrorOutOfHostMemory => memory::OutOfMemoryError::OutOfHostMemory,
                Result::ErrorOutOfDeviceMemory => memory::OutOfMemoryError::OutOfDeviceMemory,
                e => panic!("Unexpected error: {:?}", e),
            }
        })
    }

    fn buffer_requirements(&self, buffer: &Self::UnboundBuffer) -> MemoryRequirements {
        let req = DeviceV1_0::get_buffer_memory_requirements(self, *buffer);

        MemoryRequirements {
            size: req.size,
            align: req.alignment,
            mask: req.memory_type_bits,
        }
    }

    unsafe fn bind_buffer(
        &self,
        buffer: Self::UnboundBuffer,
        memory: &Self::Memory,
        offset: u64,
    ) -> Result<Self::Buffer, error::BindError> {
        DeviceV1_0::bind_buffer_memory(self, buffer, *memory, offset).map_err(|e| match e {
            vk::Result::ErrorOutOfHostMemory => {
                error::BindError::OutOfMemoryError(memory::OutOfMemoryError::OutOfHostMemory)
            }
            vk::Result::ErrorOutOfDeviceMemory => {
                error::BindError::OutOfMemoryError(memory::OutOfMemoryError::OutOfDeviceMemory)
            }
            _ => unreachable!(),
        })?;
        Ok(buffer)
    }

    unsafe fn destroy_buffer(&self, buffer: Self::Buffer) {
        DeviceV1_0::destroy_buffer(self, buffer, None);
    }

    fn create_image(
        &self,
        info: image::CreateInfo,
    ) -> Result<Self::UnboundImage, error::ImageCreationError> {
        let info = vk::ImageCreateInfo {
            s_type: vk::StructureType::ImageCreateInfo,
            p_next: ptr::null(),
            flags: vk::ImageCreateFlags::from_flags(info.flags.bits()).unwrap(),
            image_type: match info.kind {
                image::Kind::D1 => vk::ImageType::Type1d,
                image::Kind::D2 => vk::ImageType::Type2d,
                image::Kind::D3 => vk::ImageType::Type3d,
            },
            format: info.format.into(),
            extent: vk::Extent3D {
                width: info.extent.width,
                height: info.extent.height,
                depth: info.extent.depth,
            },
            mip_levels: info.mips,
            array_layers: info.array,
            samples: vk::SampleCountFlags::from_flags(info.samples.bits()).unwrap(),
            tiling: match info.tiling {
                image::ImageTiling::Optimal => vk::ImageTiling::Optimal,
                image::ImageTiling::Linear => vk::ImageTiling::Linear,
            },
            usage: vk::ImageUsageFlags::from_flags(info.usage.bits()).unwrap(),
            sharing_mode: match info.sharing {
                SharingMode::Exclusive => vk::SharingMode::Exclusive,
            },
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            initial_layout: vk::ImageLayout::Undefined,
        };

        Ok(
            unsafe { DeviceV1_0::create_image(self, &info, None) }.map_err(|e| match e {
                vk::Result::ErrorOutOfHostMemory => error::ImageCreationError::OutOfMemoryError(
                    memory::OutOfMemoryError::OutOfHostMemory,
                ),
                vk::Result::ErrorOutOfDeviceMemory => error::ImageCreationError::OutOfMemoryError(
                    memory::OutOfMemoryError::OutOfDeviceMemory,
                ),
                _ => unreachable!(),
            })?,
        )
    }

    fn image_requirements(&self, image: &Self::UnboundImage) -> MemoryRequirements {
        let req = DeviceV1_0::get_image_memory_requirements(self, *image);

        MemoryRequirements {
            size: req.size,
            align: req.alignment,
            mask: req.memory_type_bits,
        }
    }

    unsafe fn bind_image(
        &self,
        image: Self::UnboundImage,
        memory: &Self::Memory,
        offset: u64,
    ) -> Result<Self::Image, error::BindError> {
        DeviceV1_0::bind_image_memory(self, image, *memory, offset).map_err(|e| match e {
            vk::Result::ErrorOutOfHostMemory => {
                error::BindError::OutOfMemoryError(memory::OutOfMemoryError::OutOfHostMemory)
            }
            vk::Result::ErrorOutOfDeviceMemory => {
                error::BindError::OutOfMemoryError(memory::OutOfMemoryError::OutOfDeviceMemory)
            }
            _ => unreachable!(),
        })?;

        Ok(image)
    }

    unsafe fn destroy_image(&self, image: Self::Image) {
        DeviceV1_0::destroy_image(self, image, None);
    }
}
