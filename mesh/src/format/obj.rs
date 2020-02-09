//! Loading mesh data from obj format.

use log::trace;
use {
    crate::{mesh::MeshBuilder, Normal, Position, Tangent, TexCoord},
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
        }
    }
}

/// Load mesh data from obj.
pub fn load_from_obj(
    bytes: &[u8],
) -> Result<Vec<(MeshBuilder<'static>, Option<String>)>, ObjError> {
    let string = std::str::from_utf8(bytes).map_err(ObjError::Utf8)?;
    obj::parse(string)
        .and_then(load_from_data)
        .map_err(ObjError::Parse)
}

fn load_from_data(
    obj_set: obj::ObjSet,
) -> Result<Vec<(MeshBuilder<'static>, Option<String>)>, wavefront_obj::ParseError> {
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
            let tris = geometry
                .shapes
                .iter()
                .flat_map(|shape| match shape.primitive {
                    obj::Primitive::Triangle(i1, i2, i3) => Some([i1, i2, i3]),
                    _ => None,
                })
                .collect::<Vec<_>>();

            let indices = tris.iter().flatten().collect::<BTreeSet<_>>();

            let positions = indices
                .iter()
                .map(|i| {
                    let obj::Vertex { x, y, z } = object.vertices[i.0];
                    Position([x as f32, y as f32, z as f32])
                })
                .collect::<Vec<_>>();

            let normals = indices
                .iter()
                .map(|i| {
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
                .map(|i| {
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

            let mut tangents = vec![Tangent([0.0, 0.0, 0.0, 1.0]); positions.len()];

            // since reindex is flattened from tris, there should never be a remainder
            for tri in reindex.chunks_exact(3) {
                let (i1, i2, i3) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
                let tri_obj = [&positions[i1], &positions[i2], &positions[i3]];
                let tri_tex = [&tex_coords[i1], &tex_coords[i2], &tex_coords[i3]];
                let tangent = compute_tangent(tri_obj, tri_tex);
                accumulate_tangent(&mut tangents[i1], &tangent);
                accumulate_tangent(&mut tangents[i2], &tangent);
                accumulate_tangent(&mut tangents[i3], &tangent);
            }

            for tan in tangents.iter_mut() {
                *tan = normalize_tangent(tan);
            }

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

fn accumulate_tangent(acc: &mut Tangent, other: &Tangent) {
    acc.0[0] += other.0[0];
    acc.0[1] += other.0[1];
    acc.0[2] += other.0[2];
}

fn normalize_tangent(Tangent([x, y, z, w]): &Tangent) -> Tangent {
    let len = x * x + y * y + z * z;
    Tangent([x / len, y / len, z / len, *w])
}

// compute tangent for the first vertex of a tri from vertex positions
// and texture coordinates
fn compute_tangent(tri_obj: [&Position; 3], tri_tex: [&TexCoord; 3]) -> Tangent {
    let (a_obj, b_obj, c_obj) = (tri_obj[0].0, tri_obj[1].0, tri_obj[2].0);
    let (a_tex, b_tex, c_tex) = (tri_tex[0].0, tri_tex[1].0, tri_tex[2].0);

    let tspace_1_1 = b_tex[0] - a_tex[0];
    let tspace_2_1 = b_tex[1] - a_tex[1];

    let tspace_1_2 = c_tex[0] - a_tex[0];
    let tspace_2_2 = c_tex[1] - a_tex[1];

    let ospace_1_1 = b_obj[0] - a_obj[0];
    let ospace_2_1 = b_obj[1] - a_obj[1];
    let ospace_3_1 = b_obj[2] - a_obj[2];

    let ospace_1_2 = c_obj[0] - a_obj[0];
    let ospace_2_2 = c_obj[1] - a_obj[1];
    let ospace_3_2 = c_obj[2] - a_obj[2];

    let tspace_det = tspace_1_1 * tspace_2_2 - tspace_1_2 * tspace_2_1;

    let tspace_inv_1_1 = tspace_2_2 / tspace_det;
    let tspace_inv_2_1 = -tspace_2_1 / tspace_det;
    Tangent([
        ospace_1_1 * tspace_inv_1_1 + ospace_1_2 * tspace_inv_2_1,
        ospace_2_1 * tspace_inv_1_1 + ospace_2_2 * tspace_inv_2_1,
        ospace_3_1 * tspace_inv_1_1 + ospace_3_2 * tspace_inv_2_1,
        1.0,
    ])
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
}
