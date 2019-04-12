//! Module that generates functions to load a single color into a 1x1 `Texture`. Which function
//! to use depends on what color space the user is using.
//!
use crate::{pixel, texture::TextureBuilder};
use palette::{white_point::WhitePoint, Component};

macro_rules! define_load {
    (@swizzle NO) => { gfx_hal::format::Swizzle::NO };
    (@swizzle ($r:ident, $g:ident, $b:ident, $a:ident)) => {
        gfx_hal::format::Swizzle(
            gfx_hal::format::Component::$r,
            gfx_hal::format::Component::$g,
            gfx_hal::format::Component::$b,
            gfx_hal::format::Component::$a,
        )
    };
    ($(pub fn $load_fn:ident<$($ty:ident: $where:path),*>($palette:ident) -> $pixel:ident $swizzle:tt;)*) => {$(
        /// Function to load texture from `palette` pixels.
        pub fn $load_fn<$($ty),*>(
            palette: palette::$palette<$($ty),*>,
        ) -> TextureBuilder<'static>
        where
            $($ty: $where),*,
            palette::$palette<$($ty),*>: Into<pixel::$pixel>,
        {
            TextureBuilder::new()
                .with_kind(gfx_hal::image::Kind::D2(1, 1, 1, 1))
                .with_view_kind(gfx_hal::image::ViewKind::D2)
                .with_data_width(1)
                .with_data_height(1)
                .with_data(vec![palette.into()])
                .with_swizzle(define_load!(@swizzle $swizzle))
        }
    )*};
}

define_load! {
    pub fn load_from_srgb<T: Component>(Srgb) -> Rgb8Srgb NO;
    pub fn load_from_srgba<T: Component>(Srgba) -> Rgba8Srgb NO;
    pub fn load_from_linear_rgb<T: Component>(LinSrgb) -> Rgb8Unorm NO;
    pub fn load_from_linear_rgba<T: Component>(LinSrgba) -> Rgba8Unorm NO;
    pub fn load_from_linear_rgb_u16<T: Component>(LinSrgb) -> Rgb16Unorm NO;
    pub fn load_from_linear_rgba_u16<T: Component>(LinSrgba) -> Rgba16Unorm NO;
    pub fn load_from_linear_rgb_f32<T: Component>(LinSrgb) -> Rgb32Float NO;
    pub fn load_from_linear_rgba_f32<T: Component>(LinSrgba) -> Rgba32Float NO;
    pub fn load_from_luma<T: Component>(SrgbLuma) -> R8Srgb (R, R, R, One);
    pub fn load_from_lumaa<T: Component>(SrgbLumaa) -> Rg8Srgb (R, R, R, G);
    pub fn load_from_linear_luma<W: WhitePoint, T: Component>(LinLuma) -> R8Unorm (R, R, R, One);
    pub fn load_from_linear_lumaa<W: WhitePoint, T: Component>(LinLumaa) -> Rg8Unorm (R, R, R, G);
    pub fn load_from_linear_luma_f32<W: WhitePoint, T: Component>(LinLuma) -> R32Float (R, R, R, One);
    pub fn load_from_linear_lumaa_f32<W: WhitePoint, T: Component>(LinLumaa) -> Rg32Float (R, R, R, G);
}
