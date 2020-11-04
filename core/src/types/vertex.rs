//! Built-in vertex formats.

use crate::hal::format::Format;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::{borrow::Cow, fmt::Debug};

/// Trait for vertex attributes to implement
pub trait AsAttribute: Debug + PartialEq + PartialOrd + Copy + Send + Sync + 'static {
    /// Name of the attribute
    const NAME: &'static str;
    /// Attribute format.
    const FORMAT: Format;
}

/// A unique identifier for vertex attribute of given name, format and array index.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AttrUuid(u16);

lazy_static::lazy_static! {
    static ref UUID_MAP: parking_lot::RwLock<HashMap<(Cow<'static, str>, u8, Format), AttrUuid>> =
        Default::default();
}

/// Retreive a unique identifier for vertex attribute of given name, format and array index.
///
/// Non-array attributes should always use index 0.
/// Matrices and arrays must be specified as a series of attributes with the same name and consecutive indices.
pub fn attribute_uuid(name: &str, index: u8, format: Format) -> AttrUuid {
    let read_map = UUID_MAP.read();
    if let Some(val) = read_map.get(&(Cow::Borrowed(name), index, format)) {
        return *val;
    }
    drop(read_map);

    let mut write_map = UUID_MAP.write();
    // First check again if value was not written by previous owner of the lock.
    if let Some(val) = write_map.get(&(Cow::Borrowed(name), index, format)) {
        return *val;
    }

    // uuid 0 is reserved for unused attribute indices
    let val = AttrUuid(write_map.len() as u16 + 1);
    write_map.insert((Cow::Owned(name.to_owned()), index, format), val);
    val
}

/// Type for position attribute of vertex.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Position(pub [f32; 3]);
impl<T> From<T> for Position
where
    T: Into<[f32; 3]>,
{
    fn from(from: T) -> Self {
        Position(from.into())
    }
}
impl AsAttribute for Position {
    const NAME: &'static str = "position";
    const FORMAT: Format = Format::Rgb32Sfloat;
}

/// Type for color attribute of vertex
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Color(pub [f32; 4]);
impl<T> From<T> for Color
where
    T: Into<[f32; 4]>,
{
    fn from(from: T) -> Self {
        Color(from.into())
    }
}
impl AsAttribute for Color {
    const NAME: &'static str = "color";
    const FORMAT: Format = Format::Rgba32Sfloat;
}

/// Type for texture coord attribute of vertex
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Normal(pub [f32; 3]);
impl<T> From<T> for Normal
where
    T: Into<[f32; 3]>,
{
    fn from(from: T) -> Self {
        Normal(from.into())
    }
}

impl AsAttribute for Normal {
    const NAME: &'static str = "normal";
    const FORMAT: Format = Format::Rgb32Sfloat;
}

/// Type for tangent attribute of vertex. W represents handedness and should always be 1 or -1
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tangent(pub [f32; 4]);
impl<T> From<T> for Tangent
where
    T: Into<[f32; 4]>,
{
    fn from(from: T) -> Self {
        Tangent(from.into())
    }
}

impl AsAttribute for Tangent {
    const NAME: &'static str = "tangent";
    const FORMAT: Format = Format::Rgba32Sfloat;
}

/// Type for texture coord attribute of vertex
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TexCoord(pub [f32; 2]);
impl<T> From<T> for TexCoord
where
    T: Into<[f32; 2]>,
{
    fn from(from: T) -> Self {
        TexCoord(from.into())
    }
}

impl AsAttribute for TexCoord {
    const NAME: &'static str = "tex_coord";
    const FORMAT: Format = Format::Rg32Sfloat;
}

/// Vertex format contains information to initialize graphics pipeline
/// Attributes must be sorted by offset.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VertexFormat {
    /// Size of single vertex.
    pub stride: u32,
    /// Attributes for format.
    pub attributes: Vec<Attribute>,
}

impl VertexFormat {
    /// Create new vertex format with specified attributes.
    /// The max attribute offset and format size is used to calculate the stricde.
    pub fn new<I: AsAttributes>(attrs: I) -> Self {
        Self::with_opt_stride(attrs, None)
    }

    /// Create new vertex format with specified attributes and manually specified stride.
    pub fn with_stride<I: AsAttributes>(attrs: I, stride: u32) -> Self {
        Self::with_opt_stride(attrs, Some(stride))
    }

    fn with_opt_stride<I: AsAttributes>(attrs: I, stride: Option<u32>) -> Self {
        let mut attributes: Vec<Attribute> = attrs.attributes().collect();
        attributes.sort_unstable();
        let stride = stride.unwrap_or_else(|| {
            attributes
                .iter()
                .map(|attr| {
                    attr.element.offset + attr.element.format.surface_desc().bits as u32 / 8
                })
                .max()
                .expect("Vertex format cannot be empty")
        });
        Self { stride, attributes }
    }

    /// Convert into gfx digestible type.
    pub fn gfx_vertex_input_desc(
        &self,
        rate: crate::hal::pso::VertexInputRate,
    ) -> (
        Vec<crate::hal::pso::Element<Format>>,
        crate::hal::pso::ElemStride,
        crate::hal::pso::VertexInputRate,
    ) {
        (
            self.attributes
                .iter()
                .map(|attr| attr.element.clone())
                .collect(),
            self.stride,
            rate,
        )
    }
}

/// Represent types that can be interpreted as list of vertex attributes.
pub trait AsAttributes {
    /// The iterator type for retreived attributes
    type Iter: Iterator<Item = Attribute>;
    /// Retreive a list of vertex attributes with offsets relative to beginning of that list
    fn attributes(self) -> Self::Iter;
}

impl AsAttributes for Vec<Attribute> {
    type Iter = std::vec::IntoIter<Attribute>;
    fn attributes(self) -> Self::Iter {
        self.into_iter()
    }
}

impl AsAttributes for VertexFormat {
    type Iter = std::vec::IntoIter<Attribute>;
    fn attributes(self) -> Self::Iter {
        self.attributes.into_iter()
    }
}

/// An iterator adapter that generates a list of attributes with given formats and names
#[derive(Debug)]
pub struct AttrGenIter<N: Into<Cow<'static, str>>, I: Iterator<Item = (Format, N)>> {
    inner: I,
    offset: u32,
    index: u8,
    prev_name: Option<Cow<'static, str>>,
}

impl<N: Into<Cow<'static, str>>, I: Iterator<Item = (Format, N)>> AttrGenIter<N, I> {
    fn new(iter: I) -> Self {
        AttrGenIter {
            inner: iter,
            offset: 0,
            index: 0,
            prev_name: None,
        }
    }
}

impl<N: Into<Cow<'static, str>>, I: Iterator<Item = (Format, N)>> Iterator for AttrGenIter<N, I> {
    type Item = Attribute;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|data| {
            let (format, name) = data;
            let name: Cow<'static, str> = name.into();
            if self.prev_name.as_ref().map(|n| n == &name).unwrap_or(false) {
                self.index += 1;
            } else {
                self.prev_name.replace(name.clone());
                self.index = 0;
            }
            let this_offset = self.offset;
            self.offset += format.surface_desc().bits as u32 / 8;
            Attribute::new(
                name,
                self.index,
                AttributeElem {
                    format,
                    offset: this_offset,
                },
            )
        })
    }
}

impl<N: Into<Cow<'static, str>>> AsAttributes for Vec<(Format, N)> {
    type Iter = AttrGenIter<N, std::vec::IntoIter<(Format, N)>>;
    fn attributes(self) -> Self::Iter {
        AttrGenIter::new(self.into_iter())
    }
}

impl<N: Into<Cow<'static, str>>> AsAttributes for Option<(Format, N)> {
    type Iter = AttrGenIter<N, std::option::IntoIter<(Format, N)>>;
    fn attributes(self) -> Self::Iter {
        AttrGenIter::new(self.into_iter())
    }
}

impl<N: Into<Cow<'static, str>>> AsAttributes for (Format, N) {
    type Iter = AttrGenIter<N, std::option::IntoIter<(Format, N)>>;
    fn attributes(self) -> Self::Iter {
        AttrGenIter::new(Some(self).into_iter())
    }
}

/// raw hal type for vertex attribute
type AttributeElem = crate::hal::pso::Element<Format>;

/// Vertex attribute type.
#[derive(Clone, Debug)]
pub struct Attribute {
    /// globally unique identifier for attribute's semantic
    uuid: AttrUuid,
    /// hal type with offset and format
    element: AttributeElem,
    /// Attribute array index. Matrix attributes are treated like array of vectors.
    index: u8,
    /// Attribute name as used in the shader
    name: Cow<'static, str>,
}

impl PartialEq for Attribute {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid && self.element == other.element
    }
}

impl Eq for Attribute {}

impl std::hash::Hash for Attribute {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.uuid.hash(state);
        self.element.hash(state);
    }
}

impl Attribute {
    /// globally unique identifier for attribute's semantic
    pub fn uuid(&self) -> AttrUuid {
        self.uuid
    }
    /// hal type with offset and format
    pub fn element(&self) -> &AttributeElem {
        &self.element
    }
    /// Attribute array index. Matrix attributes are treated like array of vectors.
    pub fn index(&self) -> u8 {
        self.index
    }
    /// Attribute name as used in the shader
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl PartialOrd for Attribute {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(
            self.element
                .offset
                .cmp(&other.element.offset)
                .then_with(|| self.name.cmp(&other.name))
                .then_with(|| self.index.cmp(&other.index)),
        )
    }
}

impl Ord for Attribute {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[cfg(feature = "serde")]
mod serde_attribute {
    use serde::{
        de::{Error, MapAccess, SeqAccess, Visitor},
        ser::SerializeStruct,
        Deserialize, Deserializer, Serialize, Serializer,
    };

    impl Serialize for super::Attribute {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut s = serializer.serialize_struct("Attribute", 3)?;
            s.serialize_field("element", &self.element)?;
            s.serialize_field("index", &self.index)?;
            s.serialize_field("name", &self.name)?;
            s.end()
        }
    }

    impl<'de> Deserialize<'de> for super::Attribute {
        fn deserialize<D>(deserializer: D) -> Result<super::Attribute, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(field_identifier, rename_all = "lowercase")]
            enum Field {
                Element,
                Index,
                Name,
            }

            struct AttributeVisitor;
            impl<'de> Visitor<'de> for AttributeVisitor {
                type Value = super::Attribute;
                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("Attribute struct")
                }

                fn visit_seq<V>(self, mut seq: V) -> Result<super::Attribute, V::Error>
                where
                    V: SeqAccess<'de>,
                {
                    let element = seq
                        .next_element()?
                        .ok_or_else(|| V::Error::invalid_length(0, &self))?;
                    let index = seq
                        .next_element()?
                        .ok_or_else(|| V::Error::invalid_length(1, &self))?;
                    let name = seq
                        .next_element::<String>()?
                        .ok_or_else(|| V::Error::invalid_length(2, &self))?;
                    Ok(super::Attribute::new(name, index, element))
                }
                fn visit_map<V: MapAccess<'de>>(
                    self,
                    mut map: V,
                ) -> Result<super::Attribute, V::Error> {
                    let mut element = None;
                    let mut index = None;
                    let mut name: Option<&'de str> = None;
                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::Element => {
                                if element.is_some() {
                                    return Err(Error::duplicate_field("element"));
                                }
                                element.replace(map.next_value()?);
                            }
                            Field::Index => {
                                if index.is_some() {
                                    return Err(Error::duplicate_field("index"));
                                }
                                index.replace(map.next_value()?);
                            }
                            Field::Name => {
                                if name.is_some() {
                                    return Err(Error::duplicate_field("name"));
                                }
                                name.replace(map.next_value()?);
                            }
                        }
                    }
                    let element = element.ok_or_else(|| Error::missing_field("element"))?;
                    let index = index.ok_or_else(|| Error::missing_field("index"))?;
                    let name = name.ok_or_else(|| Error::missing_field("name"))?;
                    Ok(super::Attribute::new(String::from(name), index, element))
                }
            }
            deserializer.deserialize_struct(
                "Attribute",
                &["element", "index", "name"],
                AttributeVisitor,
            )
        }
    }
}

impl Attribute {
    /// Define new vertex attribute with given name and array index. Use index 0 for non-array attributes.
    pub fn new(name: impl Into<Cow<'static, str>>, index: u8, element: AttributeElem) -> Self {
        let name = name.into();
        Self {
            uuid: attribute_uuid(&name, index, element.format),
            element,
            index,
            name: name.into(),
        }
    }
}

/// Trait implemented by all valid vertex formats.
pub trait AsVertex: Debug + PartialEq + PartialOrd + Copy + Sized + Send + Sync + 'static {
    /// List of all attributes formats with name and offset.
    fn vertex() -> VertexFormat;
}

impl<T> AsVertex for T
where
    T: AsAttribute,
{
    fn vertex() -> VertexFormat {
        VertexFormat::new(Some((T::FORMAT, T::NAME)))
    }
}

/// Vertex format with position and RGBA8 color attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PosColor {
    /// Position of the vertex in 3D space.
    pub position: Position,
    /// RGBA color value of the vertex.
    pub color: Color,
}

impl AsVertex for PosColor {
    fn vertex() -> VertexFormat {
        VertexFormat::new((Position::vertex(), Color::vertex()))
    }
}

/// Vertex format with position and normal attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PosNorm {
    /// Position of the vertex in 3D space.
    pub position: Position,
    /// Normal vector of the vertex.
    pub normal: Normal,
}

impl AsVertex for PosNorm {
    fn vertex() -> VertexFormat {
        VertexFormat::new((Position::vertex(), Normal::vertex()))
    }
}

/// Vertex format with position, color and normal attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PosColorNorm {
    /// Position of the vertex in 3D space.
    pub position: Position,
    /// RGBA color value of the vertex.
    pub color: Color,
    /// Normal vector of the vertex.
    pub normal: Normal,
}

impl AsVertex for PosColorNorm {
    fn vertex() -> VertexFormat {
        VertexFormat::new((Position::vertex(), Color::vertex(), Normal::vertex()))
    }
}

/// Vertex format with position and UV texture coordinate attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PosTex {
    /// Position of the vertex in 3D space.
    pub position: Position,
    /// UV texture coordinates used by the vertex.
    pub tex_coord: TexCoord,
}

impl AsVertex for PosTex {
    fn vertex() -> VertexFormat {
        VertexFormat::new((Position::vertex(), TexCoord::vertex()))
    }
}

/// Vertex format with position, normal and UV texture coordinate attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PosNormTex {
    /// Position of the vertex in 3D space.
    pub position: Position,
    /// Normal vector of the vertex.
    pub normal: Normal,
    /// UV texture coordinates used by the vertex.
    pub tex_coord: TexCoord,
}

impl AsVertex for PosNormTex {
    fn vertex() -> VertexFormat {
        VertexFormat::new((Position::vertex(), Normal::vertex(), TexCoord::vertex()))
    }
}

/// Vertex format with position, normal, tangent, and UV texture coordinate attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PosNormTangTex {
    /// Position of the vertex in 3D space.
    pub position: Position,
    /// Normal vector of the vertex.
    pub normal: Normal,
    /// Tangent vector of the vertex.
    pub tangent: Tangent,
    /// UV texture coordinates used by the vertex.
    pub tex_coord: TexCoord,
}

impl AsVertex for PosNormTangTex {
    fn vertex() -> VertexFormat {
        VertexFormat::new((
            (Position::vertex()),
            (Normal::vertex()),
            (Tangent::vertex()),
            (TexCoord::vertex()),
        ))
    }
}

/// Full vertex transformation attribute.
/// Typically provided on per-instance basis.
/// It takes 4 attribute locations.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Model(pub [[f32; 4]; 4]);
impl<T> From<T> for Model
where
    T: Into<[[f32; 4]; 4]>,
{
    fn from(from: T) -> Self {
        Model(from.into())
    }
}

impl AsVertex for Model {
    fn vertex() -> VertexFormat {
        VertexFormat::new((
            (Format::Rgba32Sfloat, "model"),
            (Format::Rgba32Sfloat, "model"),
            (Format::Rgba32Sfloat, "model"),
            (Format::Rgba32Sfloat, "model"),
        ))
    }
}

macro_rules! impl_as_attributes {
    ($($a:ident),*) => {
        impl<$($a),*> AsAttributes for ($($a,)*) where $($a: AsAttributes),* {
            type Iter = std::vec::IntoIter<Attribute>;
            fn attributes(self) -> Self::Iter {
                let _offset: u32 = 0;
                let mut _attrs: Vec<Attribute> = Vec::new();
                #[allow(non_snake_case)]
                let ($($a,)*) = self;
                $(
                    let mut next_offset = _offset;
                    let v = $a.attributes();
                    _attrs.extend(v.map(|mut attr| {
                        attr.element.offset += _offset;
                        next_offset = next_offset.max(attr.element.offset + attr.element.format.surface_desc().bits as u32 / 8);
                        attr
                    }));
                    let _offset = next_offset;
                )*
                _attrs.into_iter()
            }
        }

        impl_as_attributes!(@ $($a),*);
    };
    (@) => {};
    (@ $head:ident $(,$tail:ident)*) => {
        impl_as_attributes!($($tail),*);
    };
}

impl_as_attributes!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
