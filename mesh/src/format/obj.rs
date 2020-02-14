//! Loading mesh data from obj format.

use log::trace;
use {
    crate::{mesh::MeshBuilder, Normal, Position, Tangent, TexCoord},
    mikktspace::Geometry,
    std::collections::{BTreeSet, HashMap},
    wavefront_obj::obj,
};

/// Object loading error.Option
#[derive(Debug, PartialEq)]
pub enum ObjError {
    /// The passed bytes were improper UTF-8 data.
    Utf8(std::str::Utf8Error),
    /// Parsing of the obj failed.
    Parse(wavefront_obj::ParseError),
    /// Geometry is unsuitable for tangent generation
    Tangent,
}

impl std::error::Error for ObjError {}
impl std::fmt::Display for ObjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjError::Utf8(e) => write!(f, "{}", e),
            ObjError::Parse(e) => write!(
                f,
                "Error parsing object file at line {}: {}",
                e.line_number, e.message
            ),
            ObjError::Tangent => write!(f, "Geometry is unsuitable for tangent generation"),
        }
    }
}

/// Load mesh data from obj.
pub fn load_from_obj(
    bytes: &[u8],
) -> Result<Vec<(MeshBuilder<'static>, Option<String>)>, ObjError> {
    let string = std::str::from_utf8(bytes).map_err(ObjError::Utf8)?;
    obj::parse(string)
        .map_err(ObjError::Parse)
        .and_then(load_from_data)
}

fn load_from_data(
    obj_set: obj::ObjSet,
) -> Result<Vec<(MeshBuilder<'static>, Option<String>)>, ObjError> {
    // Takes a list of objects that contain geometries that contain shapes that contain
    // vertex/texture/normal indices into the main list of vertices, and converts to
    // MeshBuilders with Position, Normal, TexCoord.
    trace!("Loading mesh");
    let mut objects = vec![];

    for object in obj_set.objects {
        for geometry in &object.geometry {
            let mut builder = MeshBuilder::new();

            // Since vertices, normals, tangents, and texture coordinates share
            // indices in rendy, we need an index for each unique VTNIndex.
            // E.x. f 1/1/1, 2/2/1, and 1/2/1 needs three different vertices, even
            // though only two vertices are referenced in the soure wavefron OBJ.
            // We also don't want triangle with opposite windings to share a vertex.
            let tris = geometry
                .shapes
                .iter()
                .flat_map(|shape| match shape.primitive {
                    obj::Primitive::Triangle(i1, i2, i3) => {
                        let h = winding(
                            i1.1.map(|i| object.tex_vertices[i])
                                .unwrap_or(obj::TVertex {
                                    u: 0.0,
                                    v: 0.0,
                                    w: 0.0,
                                }),
                            i2.1.map(|i| object.tex_vertices[i])
                                .unwrap_or(obj::TVertex {
                                    u: 0.0,
                                    v: 0.0,
                                    w: 0.0,
                                }),
                            i3.1.map(|i| object.tex_vertices[i])
                                .unwrap_or(obj::TVertex {
                                    u: 0.0,
                                    v: 0.0,
                                    w: 0.0,
                                }),
                        );
                        Some([(i1, h), (i2, h), (i3, h)])
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();

            let indices = tris.iter().flatten().collect::<BTreeSet<_>>();

            let positions = indices
                .iter()
                .map(|(i, _)| {
                    let obj::Vertex { x, y, z } = object.vertices[i.0];
                    Position([x as f32, y as f32, z as f32])
                })
                .collect::<Vec<_>>();

            let normals = indices
                .iter()
                .map(|(i, _)| {
                    if let Some(j) = i.2 {
                        let obj::Normal { x, y, z } = object.normals[j];
                        Normal([x as f32, y as f32, z as f32])
                    } else {
                        Normal([0.0, 0.0, 0.0])
                    }
                })
                .collect::<Vec<_>>();

            let tex_coords = indices
                .iter()
                .map(|(i, _)| {
                    if let Some(j) = i.1 {
                        let obj::TVertex { u, v, .. } = object.tex_vertices[j];
                        TexCoord([u as f32, v as f32])
                    } else {
                        TexCoord([0.0, 0.0])
                    }
                })
                .collect::<Vec<_>>();

            let index_map = indices
                .iter()
                .enumerate()
                .map(|(v, k)| (k, v as u32))
                .collect::<HashMap<_, _>>();

            let reindex = tris
                .iter()
                .flatten()
                .map(|i| index_map[&i])
                .collect::<Vec<_>>();

            let tangents = {
                let mut obj_geom = ObjGeometry::new(&positions, &normals, &tex_coords, &reindex);
                if !mikktspace::generate_tangents(&mut obj_geom) {
                    return Err(ObjError::Tangent);
                }
                obj_geom.get_tangents()
            };

            debug_assert!(&normals.len() == &positions.len());
            debug_assert!(&tangents.len() == &positions.len());
            debug_assert!(&tex_coords.len() == &positions.len());

            builder.add_vertices(positions);
            builder.add_vertices(normals);
            builder.add_vertices(tangents);
            builder.add_vertices(tex_coords);
            builder.set_indices(reindex);

            // TODO: Add Material loading
            objects.push((builder, geometry.material_name.clone()))
        }
    }
    trace!("Loaded mesh");
    Ok(objects)
}

fn winding(a: obj::TVertex, b: obj::TVertex, c: obj::TVertex) -> i8 {
    let d = obj::TVertex {
        u: b.u - a.u,
        v: b.v - a.v,
        w: b.w - a.w,
    };
    let e = obj::TVertex {
        u: c.u - a.u,
        v: c.v - a.v,
        w: c.w - a.w,
    };
    // only need w component of cross product
    let w = d.u * e.v - d.v * e.u;
    w.signum() as i8
}

// Only supports tris, therefore indices.len() must be divisible by 3, and
// assumes each 3 vertices represents a tri
struct ObjGeometry<'a> {
    positions: &'a Vec<Position>,
    normals: &'a Vec<Normal>,
    tex_coords: &'a Vec<TexCoord>,
    indices: &'a Vec<u32>,
    tangents: Vec<Tangent>,
}

impl<'a> ObjGeometry<'a> {
    fn new(
        positions: &'a Vec<Position>,
        normals: &'a Vec<Normal>,
        tex_coords: &'a Vec<TexCoord>,
        indices: &'a Vec<u32>,
    ) -> Self {
        Self {
            positions,
            normals,
            tex_coords,
            indices,
            tangents: vec![Tangent([0.0, 0.0, 0.0, 1.0]); positions.len()],
        }
    }

    fn accumulate_tangent(&mut self, index: usize, other: [f32; 4]) {
        let acc = &mut self.tangents[index];
        acc.0[0] += other[0];
        acc.0[1] += other[1];
        acc.0[2] += other[2];
        acc.0[3] = other[3];
    }

    fn normalize_tangent(Tangent([x, y, z, w]): &Tangent) -> Tangent {
        let len = x * x + y * y + z * z;
        Tangent([x / len, y / len, z / len, *w])
    }

    fn get_tangents(&self) -> Vec<Tangent> {
        self.tangents
            .iter()
            .map(Self::normalize_tangent)
            .collect::<Vec<_>>()
    }
}

impl Geometry for ObjGeometry<'_> {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }

    fn num_vertices_of_face(&self, _: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        self.positions[self.indices[face * 3 + vert] as usize].0
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        self.normals[self.indices[face * 3 + vert] as usize].0
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.tex_coords[self.indices[face * 3 + vert] as usize].0
    }

    fn set_tangent_encoded(&mut self, tangent: [f32; 4], face: usize, vert: usize) {
        // Not supposed to just average tangents over existing index,
        // since triangles could be welded using different asumptions than
        // Mikkelsen used. However, we *do* use basically the same assumptions,
        // except that some vertices Mikkelsen expects to be welded may not be
        // if they aren't in the source OBJ.
        self.accumulate_tangent(self.indices[face * 3 + vert] as usize, tangent);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load_from_obj() {
        let quad = b"v -1.000000 -1.000000 1.000000\nv 1.000000 -1.000000 1.000000\nv -1.000000 1.000000 1.000000\nv 1.000000 1.000000 1.000000\nv -1.000000 1.000000 -1.000000\nv 1.000000 1.000000 -1.000000\nv -1.000000 -1.000000 -1.000000\nv 1.000000 -1.000000 -1.000000\n
vt 0.000000 0.000000\nvt 1.000000 0.000000\nvt 0.000000 1.000000\nvt 1.000000 1.000000\n
vn 0.000000 0.000000 1.000000\nvn 0.000000 1.000000 0.000000\nvn 0.000000 0.000000 -1.000000\nvn 0.000000 -1.000000 0.000000\nvn 1.000000 0.000000 0.000000\nvn -1.000000 0.000000 0.000000\n
s 1
f 1/1/1 2/2/1 3/3/1\nf 3/3/1 2/2/1 4/4/1
s 2
f 3/1/2 4/2/2 5/3/2\nf 5/3/2 4/2/2 6/4/2
s 3
f 5/4/3 6/3/3 7/2/3\nf 7/2/3 6/3/3 8/1/3
s 4
f 7/1/4 8/2/4 1/3/4\nf 1/3/4 8/2/4 2/4/4
s 5
f 2/1/5 8/2/5 4/3/5\nf 4/3/5 8/2/5 6/4/5
s 6
f 7/1/6 1/2/6 5/3/6\nf 5/3/6 1/2/6 3/4/6
";
        let result = load_from_obj(quad).ok().unwrap();
        // dbg!(& result);
        assert_eq!(result.len(), 1);

        // When compressed into unique vertices there should be 4 vertices per side of the quad
        // assert!()
    }

    #[test]
    fn test_winding() {
        let a = obj::TVertex {
            u: 0.0,
            v: 0.0,
            w: 0.0,
        };
        let b = obj::TVertex {
            u: 1.0,
            v: 0.0,
            w: 0.0,
        };
        let c = obj::TVertex {
            u: 0.0,
            v: 1.0,
            w: 0.0,
        };
        let counter_clockwise = winding(a, b, c);
        assert_eq!(counter_clockwise, 1);
        let clockwise = winding(a, c, b);
        assert_eq!(clockwise, -1);
    }
}
