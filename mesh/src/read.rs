use crate::core::hal::format::Format;

/// Type that can be parsed from a vertex buffer.
///
/// This module provides readers for:
/// * Integer types and fixed-sizes arrays of length `1-4`. Bitness, number of components, and sign must match.
///   `*norm` and `*scale` formats are not changed. SRGB values can be read as unsigned integers.
/// * `f32/64` and fixed-sizes arrays of length `1-4`: Number of components must match. `*norm` and `*scale` formats
///   are converted.
/// * 2-tuples, 3-tuples, and 4-tuples for all types that implement this trait
///   for size 2, 3, and 4 arrays.
///
/// For the above, BGR, BGRA, and ARGB formats are rearranged into RGB(A).
pub trait FromVertexBuffer {
    /// Can this type be parsed from a buffer containing this format?
    fn is_format_compatible(format: Format) -> bool;
    /// Parses a section of a vertex buffer as the supplied format and
    /// returns the result.
    ///
    /// Panics
    /// ======
    ///
    /// Can panic if the format is not supported (`is_format_compatible(format)` would return false),
    /// or the buffer is not sized according to the format.
    fn read_one(format: Format, section: &[u8]) -> Self;
}

/// Helper trait for reading components and converting them from `norm` and `scaled` formats.
trait ReadComponent {
    const SLICE_SIZE: usize;
    fn read_one_raw(buf: &[u8]) -> Self;
    fn normalize32(&self) -> f32;
    fn normalize64(&self) -> f64;
    fn scale32(&self) -> f32;
    fn scale64(&self) -> f64;
}

macro_rules! impl_normalize_and_scale {
    (unsigned) => {
        fn normalize32(&self) -> f32 { *self as f32 / Self::max_value() as f32 }
        fn normalize64(&self) -> f64 { *self as f64 / Self::max_value() as f64 }
        fn scale32(&self) -> f32 { *self as f32 }
        fn scale64(&self) -> f64 { *self as f64 }
    };
    (signed) => {
        fn normalize32(&self) -> f32 { (*self as f32 / Self::max_value() as f32).max(-1.0) }
        fn normalize64(&self) -> f64 { (*self as f64 / Self::max_value() as f64).max(-1.0) }
        fn scale32(&self) -> f32 { *self as f32 }
        fn scale64(&self) -> f64 { *self as f64 }
    };
}

impl ReadComponent for u8 {
    const SLICE_SIZE: usize = 1;
    fn read_one_raw(buf: &[u8]) -> Self {
        buf[0]
    }
    impl_normalize_and_scale!(unsigned);
}
impl ReadComponent for i8 {
    const SLICE_SIZE: usize = 1;
    fn read_one_raw(buf: &[u8]) -> Self {
        buf[0] as i8
    }
    impl_normalize_and_scale!(signed);
}
impl ReadComponent for u16 {
    const SLICE_SIZE: usize = 2;
    fn read_one_raw(buf: &[u8]) -> Self {
        Self::from_ne_bytes([buf[0], buf[1]])
    }
    impl_normalize_and_scale!(unsigned);
}
impl ReadComponent for i16 {
    const SLICE_SIZE: usize = 2;
    fn read_one_raw(buf: &[u8]) -> Self {
        Self::from_ne_bytes([buf[0], buf[1]])
    }
    impl_normalize_and_scale!(signed);
}
impl ReadComponent for u32 {
    const SLICE_SIZE: usize = 4;
    fn read_one_raw(buf: &[u8]) -> Self {
        Self::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]])
    }
    impl_normalize_and_scale!(unsigned);
}
impl ReadComponent for i32 {
    const SLICE_SIZE: usize = 4;
    fn read_one_raw(buf: &[u8]) -> Self {
        Self::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]])
    }
    impl_normalize_and_scale!(signed);
}
impl ReadComponent for u64 {
    const SLICE_SIZE: usize = 8;
    fn read_one_raw(buf: &[u8]) -> Self {
        Self::from_ne_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ])
    }
    impl_normalize_and_scale!(unsigned);
}
impl ReadComponent for i64 {
    const SLICE_SIZE: usize = 8;
    fn read_one_raw(buf: &[u8]) -> Self {
        Self::from_ne_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ])
    }
    impl_normalize_and_scale!(signed);
}
impl ReadComponent for f32 {
    const SLICE_SIZE: usize = 4;
    fn read_one_raw(buf: &[u8]) -> Self {
        f32::from_bits(u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]))
    }
    fn normalize32(&self) -> f32 {
        *self
    }
    fn normalize64(&self) -> f64 {
        *self as f64
    }
    fn scale32(&self) -> f32 {
        *self
    }
    fn scale64(&self) -> f64 {
        *self as f64
    }
}
impl ReadComponent for f64 {
    const SLICE_SIZE: usize = 8;
    fn read_one_raw(buf: &[u8]) -> Self {
        f64::from_bits(u64::from_ne_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ]))
    }
    fn normalize32(&self) -> f32 {
        *self as f32
    }
    fn normalize64(&self) -> f64 {
        *self
    }
    fn scale32(&self) -> f32 {
        *self as f32
    }
    fn scale64(&self) -> f64 {
        *self
    }
}

// Helper trait for using the appropriate `normalize`/`scale` functions
trait FromComponent {
    fn from_normalized<T: ReadComponent>(v: T) -> Self;
    fn from_scaled<T: ReadComponent>(v: T) -> Self;
}
impl FromComponent for f32 {
    fn from_normalized<T: ReadComponent>(v: T) -> Self {
        v.normalize32()
    }
    fn from_scaled<T: ReadComponent>(v: T) -> Self {
        v.scale32()
    }
}
impl FromComponent for f64 {
    fn from_normalized<T: ReadComponent>(v: T) -> Self {
        v.normalize64()
    }
    fn from_scaled<T: ReadComponent>(v: T) -> Self {
        v.scale64()
    }
}

// Helper trait for casting f32 to f64 and vice versa
trait Cast<To> {
    fn cast(self) -> To;
}
impl<T> Cast<T> for T {
    fn cast(self) -> T {
        self
    }
}
impl Cast<f32> for f64 {
    fn cast(self) -> f32 {
        self as f32
    }
}
impl Cast<f64> for f32 {
    fn cast(self) -> f64 {
        self as f64
    }
}

fn iter_components<'a, T: ReadComponent>(slice: &'a [u8]) -> impl Iterator<Item = T> + 'a {
    assert_eq!(slice.len() % T::SLICE_SIZE, 0);
    (0..slice.len() / T::SLICE_SIZE)
        .map(move |i| T::read_one_raw(&slice[i * T::SLICE_SIZE..(i + 1) * T::SLICE_SIZE]))
}

trait CollectFixed<Out> {
    fn collect_fixed(self) -> Out;
}
impl<Iter: Iterator> CollectFixed<[Iter::Item; 1]> for Iter {
    fn collect_fixed(mut self) -> [Iter::Item; 1] {
        let v = self.next().unwrap();
        assert!(self.next().is_none());
        [v]
    }
}
impl<Iter: Iterator> CollectFixed<[Iter::Item; 2]> for Iter {
    fn collect_fixed(mut self) -> [Iter::Item; 2] {
        let v1 = self.next().unwrap();
        let v2 = self.next().unwrap();
        assert!(self.next().is_none());
        [v1, v2]
    }
}
impl<Iter: Iterator> CollectFixed<[Iter::Item; 3]> for Iter {
    fn collect_fixed(mut self) -> [Iter::Item; 3] {
        let v1 = self.next().unwrap();
        let v2 = self.next().unwrap();
        let v3 = self.next().unwrap();
        assert!(self.next().is_none());
        [v1, v2, v3]
    }
}
impl<Iter: Iterator> CollectFixed<[Iter::Item; 4]> for Iter {
    fn collect_fixed(mut self) -> [Iter::Item; 4] {
        let v1 = self.next().unwrap();
        let v2 = self.next().unwrap();
        let v3 = self.next().unwrap();
        let v4 = self.next().unwrap();
        assert!(self.next().is_none());
        [v1, v2, v3, v4]
    }
}

macro_rules! impl_from_vertex_buffer {
    (|$slice:ident| {$(
        $fmt1:pat $( | $fmtn:pat)* => $e:expr
    ),+$(,)?}) => {
        fn is_format_compatible(format: Format) -> bool {
            match format {
                $($fmt1 $( | $fmtn)* => true),+,
                _ => false,
            }
        }

        fn read_one(format: Format, $slice: &[u8]) -> Self {
            match format {
                $($fmt1 $( | $fmtn)* => $e ),*
                _ => { panic!("Can not read format {:?} as Self"); }
            }
        }
    };
}

// Some helpers
fn ident<T>(v: T) -> T {
    v
}
fn bgr2rgb<T: Copy>(v: [T; 3]) -> [T; 3] {
    [v[2], v[1], v[0]]
}
fn bgra2rgba<T: Copy>(v: [T; 4]) -> [T; 4] {
    [v[2], v[1], v[0], v[3]]
}
fn abgr2rgba<T: Copy>(v: [T; 4]) -> [T; 4] {
    [v[3], v[2], v[1], v[0]]
}

macro_rules! cpxf {
    ($slice:expr, $in_typ:ty, scaled) => {
        cpxf!($slice, $in_typ, FromComponent::from_scaled)
    };
    ($slice:expr, $in_typ:ty, normalized) => {
        cpxf!($slice, $in_typ, FromComponent::from_normalized)
    };
    ($slice:expr, $in_typ:ty, $comp_xform:expr) => {
        iter_components::<$in_typ>($slice)
            .map($comp_xform)
            .collect_fixed()
    };
}

// TODO: clean up the copy-paste code below.
//
// The biggest issue is that we need to concatenate the number of components (R/Rg/Rgb/Rgba), the
// type width (8/16/32/64), the signedness (S/U), and the storage interpretation (norm/scaled/int/float)
// into an identifier (ex. R8Unorm). This currently is impossible, since `concat_idents` is nightly only,
// and procedural macros (ex. the paste crate) cannot be used in match patters as of this writing.

impl FromVertexBuffer for [u8; 1] {
    impl_from_vertex_buffer!(|slice| {
        Format::R8Unorm |
        Format::R8Uscaled |
        Format::R8Uint |
        Format::R8Srgb => { cpxf!(slice, u8, ident) },
    });
}
impl FromVertexBuffer for [u8; 2] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rg8Unorm |
        Format::Rg8Uscaled |
        Format::Rg8Uint |
        Format::Rg8Srgb => { cpxf!(slice, u8, ident) },
    });
}
impl FromVertexBuffer for [u8; 3] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgb8Unorm |
        Format::Rgb8Uscaled |
        Format::Rgb8Uint |
        Format::Rgb8Srgb => { cpxf!(slice, u8, ident) },

        Format::Bgr8Unorm |
        Format::Bgr8Uscaled |
        Format::Bgr8Uint |
        Format::Bgr8Srgb => { bgr2rgb(cpxf!(slice, u8, ident)) },
    });
}
impl FromVertexBuffer for [u8; 4] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgba8Unorm |
        Format::Rgba8Uscaled |
        Format::Rgba8Uint |
        Format::Rgba8Srgb => { cpxf!(slice, u8, ident) },

        Format::Bgra8Unorm |
        Format::Bgra8Uscaled |
        Format::Bgra8Uint |
        Format::Bgra8Srgb => { bgra2rgba(cpxf!(slice, u8, ident)) },

        Format::Abgr8Unorm |
        Format::Abgr8Uscaled |
        Format::Abgr8Uint |
        Format::Abgr8Srgb => { abgr2rgba(cpxf!(slice, u8, ident)) },
    });
}
impl FromVertexBuffer for u8 {
    fn is_format_compatible(format: Format) -> bool {
        <[u8; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[u8; 1]>::read_one(format, section)[0]
    }
}

impl FromVertexBuffer for [u16; 1] {
    impl_from_vertex_buffer!(|slice| {
        Format::R16Unorm |
        Format::R16Uscaled |
        Format::R16Uint => { cpxf!(slice, u16, ident) },
    });
}
impl FromVertexBuffer for [u16; 2] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rg16Unorm |
        Format::Rg16Uscaled |
        Format::Rg16Uint => { cpxf!(slice, u16, ident) },
    });
}
impl FromVertexBuffer for [u16; 3] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgb16Unorm |
        Format::Rgb16Uscaled |
        Format::Rgb16Uint => { cpxf!(slice, u16, ident) },
    });
}
impl FromVertexBuffer for [u16; 4] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgba16Unorm |
        Format::Rgba16Uscaled |
        Format::Rgba16Uint => { cpxf!(slice, u16, ident) },
    });
}
impl FromVertexBuffer for u16 {
    fn is_format_compatible(format: Format) -> bool {
        <[u16; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[u16; 1]>::read_one(format, section)[0]
    }
}

impl FromVertexBuffer for [u32; 1] {
    impl_from_vertex_buffer!(|slice| {
        Format::R32Uint => { cpxf!(slice, u32, ident) },
    });
}
impl FromVertexBuffer for [u32; 2] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rg32Uint => { cpxf!(slice, u32, ident) },
    });
}
impl FromVertexBuffer for [u32; 3] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgb32Uint => { cpxf!(slice, u32, ident) },
    });
}
impl FromVertexBuffer for [u32; 4] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgba32Uint => { cpxf!(slice, u32, ident) },
    });
}
impl FromVertexBuffer for u32 {
    fn is_format_compatible(format: Format) -> bool {
        <[u32; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[u32; 1]>::read_one(format, section)[0]
    }
}

impl FromVertexBuffer for [u64; 1] {
    impl_from_vertex_buffer!(|slice| {
        Format::R64Uint => { cpxf!(slice, u64, ident) },
    });
}
impl FromVertexBuffer for [u64; 2] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rg64Uint => { cpxf!(slice, u64, ident) },
    });
}
impl FromVertexBuffer for [u64; 3] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgb64Uint => { cpxf!(slice, u64, ident) },
    });
}
impl FromVertexBuffer for [u64; 4] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgba64Uint => { cpxf!(slice, u64, ident) },
    });
}
impl FromVertexBuffer for u64 {
    fn is_format_compatible(format: Format) -> bool {
        <[u64; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[u64; 1]>::read_one(format, section)[0]
    }
}

impl FromVertexBuffer for [i8; 1] {
    impl_from_vertex_buffer!(|slice| {
        Format::R8Snorm |
        Format::R8Sscaled |
        Format::R8Sint => { cpxf!(slice, i8, ident) },
    });
}
impl FromVertexBuffer for [i8; 2] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rg8Snorm |
        Format::Rg8Sscaled |
        Format::Rg8Sint => { cpxf!(slice, i8, ident) },
    });
}
impl FromVertexBuffer for [i8; 3] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgb8Snorm |
        Format::Rgb8Sscaled |
        Format::Rgb8Sint => { cpxf!(slice, i8, ident) },

        Format::Bgr8Snorm |
        Format::Bgr8Sscaled |
        Format::Bgr8Sint => { bgr2rgb(cpxf!(slice, i8, ident)) },
    });
}
impl FromVertexBuffer for [i8; 4] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgba8Snorm |
        Format::Rgba8Sscaled |
        Format::Rgba8Sint => { cpxf!(slice, i8, ident) },

        Format::Bgra8Snorm |
        Format::Bgra8Sscaled |
        Format::Bgra8Sint => { bgra2rgba(cpxf!(slice, i8, ident)) },

        Format::Abgr8Snorm |
        Format::Abgr8Sscaled |
        Format::Abgr8Sint => { abgr2rgba(cpxf!(slice, i8, ident)) },
    });
}
impl FromVertexBuffer for i8 {
    fn is_format_compatible(format: Format) -> bool {
        <[i8; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[i8; 1]>::read_one(format, section)[0]
    }
}

impl FromVertexBuffer for [i16; 1] {
    impl_from_vertex_buffer!(|slice| {
        Format::R16Snorm |
        Format::R16Sscaled |
        Format::R16Sint => { cpxf!(slice, i16, ident) },
    });
}
impl FromVertexBuffer for [i16; 2] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rg16Snorm |
        Format::Rg16Sscaled |
        Format::Rg16Sint => { cpxf!(slice, i16, ident) },
    });
}
impl FromVertexBuffer for [i16; 3] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgb16Snorm |
        Format::Rgb16Sscaled |
        Format::Rgb16Sint => { cpxf!(slice, i16, ident) },
    });
}
impl FromVertexBuffer for [i16; 4] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgba16Snorm |
        Format::Rgba16Sscaled |
        Format::Rgba16Sint => { cpxf!(slice, i16, ident) },
    });
}
impl FromVertexBuffer for i16 {
    fn is_format_compatible(format: Format) -> bool {
        <[i16; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[i16; 1]>::read_one(format, section)[0]
    }
}

impl FromVertexBuffer for [i32; 1] {
    impl_from_vertex_buffer!(|slice| {
        Format::R32Sint => { cpxf!(slice, i32, ident) },
    });
}
impl FromVertexBuffer for [i32; 2] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rg32Sint => { cpxf!(slice, i32, ident) },
    });
}
impl FromVertexBuffer for [i32; 3] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgb32Sint => { cpxf!(slice, i32, ident) },
    });
}
impl FromVertexBuffer for [i32; 4] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgba32Sint => { cpxf!(slice, i32, ident) },
    });
}
impl FromVertexBuffer for i32 {
    fn is_format_compatible(format: Format) -> bool {
        <[i32; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[i32; 1]>::read_one(format, section)[0]
    }
}

impl FromVertexBuffer for [i64; 1] {
    impl_from_vertex_buffer!(|slice| {
        Format::R64Sint => { cpxf!(slice, i64, ident) },
    });
}
impl FromVertexBuffer for [i64; 2] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rg64Sint => { cpxf!(slice, i64, ident) },
    });
}
impl FromVertexBuffer for [i64; 3] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgb64Sint => { cpxf!(slice, i64, ident) },
    });
}
impl FromVertexBuffer for [i64; 4] {
    impl_from_vertex_buffer!(|slice| {
        Format::Rgba64Sint => { cpxf!(slice, i64, ident) },
    });
}
impl FromVertexBuffer for i64 {
    fn is_format_compatible(format: Format) -> bool {
        <[i64; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[i64; 1]>::read_one(format, section)[0]
    }
}

macro_rules! impl_for_both_floats {
    ($n:tt, |$slice:ident| $content:tt) => {
        impl FromVertexBuffer for [f32; $n] {
            impl_from_vertex_buffer!(|$slice| $content);
        }
        impl FromVertexBuffer for [f64; $n] {
            impl_from_vertex_buffer!(|$slice| $content);
        }
    };
}

impl_for_both_floats!(1, |slice| {
    Format::R8Unorm    => { cpxf!(slice, u8, normalized) },
    Format::R8Uscaled  => { cpxf!(slice, u8, scaled) },
    Format::R8Snorm    => { cpxf!(slice, i8, normalized) },
    Format::R8Sscaled  => { cpxf!(slice, i8, scaled) },

    Format::R16Unorm   => { cpxf!(slice, u16, normalized) },
    Format::R16Uscaled => { cpxf!(slice, u16, scaled) },
    Format::R16Snorm   => { cpxf!(slice, i16, normalized) },
    Format::R16Sscaled => { cpxf!(slice, i16, scaled) },

    Format::R32Sfloat  => { cpxf!(slice, f32, Cast::cast) },
    Format::R64Sfloat  => { cpxf!(slice, f64, Cast::cast) },
});

impl_for_both_floats!(2, |slice| {
    Format::Rg8Unorm    => { cpxf!(slice, u8, normalized) },
    Format::Rg8Uscaled  => { cpxf!(slice, u8, scaled) },
    Format::Rg8Snorm    => { cpxf!(slice, i8, normalized) },
    Format::Rg8Sscaled  => { cpxf!(slice, i8, scaled) },

    Format::Rg16Unorm   => { cpxf!(slice, u16, normalized) },
    Format::Rg16Uscaled => { cpxf!(slice, u16, scaled) },
    Format::Rg16Snorm   => { cpxf!(slice, i16, normalized) },
    Format::Rg16Sscaled => { cpxf!(slice, i16, scaled) },

    Format::Rg32Sfloat  => { cpxf!(slice, f32, Cast::cast) },
    Format::Rg64Sfloat  => { cpxf!(slice, f64, Cast::cast) },
});

impl_for_both_floats!(3, |slice| {
    Format::Rgb8Unorm    => { cpxf!(slice, u8, normalized) },
    Format::Rgb8Uscaled  => { cpxf!(slice, u8, scaled) },
    Format::Rgb8Snorm    => { cpxf!(slice, i8, normalized) },
    Format::Rgb8Sscaled  => { cpxf!(slice, i8, scaled) },

    Format::Bgr8Unorm    => { bgr2rgb(cpxf!(slice, u8, normalized)) },
    Format::Bgr8Uscaled  => { bgr2rgb(cpxf!(slice, u8, scaled)) },
    Format::Bgr8Snorm    => { bgr2rgb(cpxf!(slice, i8, normalized)) },
    Format::Bgr8Sscaled  => { bgr2rgb(cpxf!(slice, i8, scaled)) },

    Format::Rgb16Unorm   => { cpxf!(slice, u16, normalized) },
    Format::Rgb16Uscaled => { cpxf!(slice, u16, scaled) },
    Format::Rgb16Snorm   => { cpxf!(slice, i16, normalized) },
    Format::Rgb16Sscaled => { cpxf!(slice, i16, scaled) }, 

    Format::Rgb32Sfloat  => { cpxf!(slice, f32, Cast::cast) },
    Format::Rgb64Sfloat  => { cpxf!(slice, f64, Cast::cast) },
});

impl_for_both_floats!(4, |slice| {
    Format::Rgba8Unorm    => { cpxf!(slice, u8, normalized) },
    Format::Rgba8Uscaled  => { cpxf!(slice, u8, scaled) },
    Format::Rgba8Snorm    => { cpxf!(slice, i8, normalized) },
    Format::Rgba8Sscaled  => { cpxf!(slice, i8, scaled) },

    Format::Bgra8Unorm    => { bgra2rgba(cpxf!(slice, u8, normalized)) },
    Format::Bgra8Uscaled  => { bgra2rgba(cpxf!(slice, u8, scaled)) },
    Format::Bgra8Snorm    => { bgra2rgba(cpxf!(slice, i8, normalized)) },
    Format::Bgra8Sscaled  => { bgra2rgba(cpxf!(slice, i8, scaled)) },

    Format::Abgr8Unorm    => { abgr2rgba(cpxf!(slice, u8, normalized)) },
    Format::Abgr8Uscaled  => { abgr2rgba(cpxf!(slice, u8, scaled)) },
    Format::Abgr8Snorm    => { abgr2rgba(cpxf!(slice, i8, normalized)) },
    Format::Abgr8Sscaled  => { abgr2rgba(cpxf!(slice, i8, scaled)) },

    Format::Rgba16Unorm   => { cpxf!(slice, u16, normalized) },
    Format::Rgba16Uscaled => { cpxf!(slice, u16, scaled) },
    Format::Rgba16Snorm   => { cpxf!(slice, i16, normalized) },
    Format::Rgba16Sscaled => { cpxf!(slice, i16, scaled) },

    Format::Rgba32Sfloat  => { cpxf!(slice, f32, Cast::cast) },
    Format::Rgba64Sfloat  => { cpxf!(slice, f64, Cast::cast) },
});

impl FromVertexBuffer for f32 {
    fn is_format_compatible(format: Format) -> bool {
        <[f32; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[f32; 1]>::read_one(format, section)[0]
    }
}

impl FromVertexBuffer for f64 {
    fn is_format_compatible(format: Format) -> bool {
        <[f64; 1]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        <[f64; 1]>::read_one(format, section)[0]
    }
}

impl<T> FromVertexBuffer for (T, T)
where
    T: Clone,
    [T; 2]: FromVertexBuffer,
{
    fn is_format_compatible(format: Format) -> bool {
        <[T; 2]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        let arr = <[T; 2]>::read_one(format, section);
        (arr[0].clone(), arr[1].clone())
    }
}
impl<T> FromVertexBuffer for (T, T, T)
where
    T: Clone,
    [T; 3]: FromVertexBuffer,
{
    fn is_format_compatible(format: Format) -> bool {
        <[T; 3]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        let arr = <[T; 3]>::read_one(format, section);
        (arr[0].clone(), arr[1].clone(), arr[2].clone())
    }
}
impl<T> FromVertexBuffer for (T, T, T, T)
where
    T: Clone,
    [T; 4]: FromVertexBuffer,
{
    fn is_format_compatible(format: Format) -> bool {
        <[T; 4]>::is_format_compatible(format)
    }
    fn read_one(format: Format, section: &[u8]) -> Self {
        let arr = <[T; 4]>::read_one(format, section);
        (
            arr[0].clone(),
            arr[1].clone(),
            arr[2].clone(),
            arr[3].clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unpack_floats(d: &[f32]) -> Vec<u8> {
        let mut vec = Vec::with_capacity(d.len() * 4);
        for v in d.iter() {
            let u = v.to_bits();
            let bytes = u.to_ne_bytes();
            vec.extend(bytes.iter());
        }
        vec
    }

    #[test]
    fn read_rgb32sfloat() {
        assert_eq!(
            <[f32; 3] as FromVertexBuffer>::read_one(
                Format::Rgb32Sfloat,
                &unpack_floats(&[1., 2., 3.])
            ),
            [1., 2., 3.]
        );
    }
}
