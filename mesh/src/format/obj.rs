//! Loading mesh data from obj format.

use {
    crate::{mesh::MeshBuilder, Normal, PosNormTex, Position, TexCoord},
    wavefront_obj::obj,
};
use log::trace;

/// Load mesh data from obj.
pub fn load_from_obj(bytes: &[u8]) -> Result<Vec<(MeshBuilder<'static>, Option<String>)>, failure::Error> {
    let string = std::str::from_utf8(bytes)?;
    let set = obj::parse(string).map_err(|e| {
        failure::format_err!(
            "Error during parsing obj-file at line '{}': {}",
            e.line_number,
            e.message
        )
    })?;
    load_from_data(set)
}

fn convert(
    object: &obj::Object,
    vi: obj::VertexIndex,
    ti: Option<obj::TextureIndex>,
    ni: Option<obj::NormalIndex>,
) -> (Position, Normal, TexCoord) {
    let vertex: obj::Vertex = object.vertices[vi];
    
    let normal = ni
        .map(|i| {
            let normal: obj::Normal = object.normals[i];
            Normal([normal.x as f32, normal.y as f32, normal.z as f32])
        })
        .unwrap_or(Normal([0.0, 0.0, 0.0]));
    let tex_coord = ti
        .map(|i| {
            let tvertex: obj::TVertex = object.tex_vertices[i];
            TexCoord([tvertex.u as f32, tvertex.v as f32])
        })
        .unwrap_or(TexCoord([0.0, 0.0]));

    (Position([vertex.x as f32, vertex.y as f32, vertex.z as f32]), normal, tex_coord)
}

fn convert_primitive(object: &obj::Object, prim: &obj::Primitive) -> Option<[(Position, Normal, TexCoord); 3]> {
    match *prim {
        obj::Primitive::Triangle(v1, v2, v3) => Some([
            convert(object, v1.0, v1.1, v1.2),
            convert(object, v2.0, v2.1, v2.2),
            convert(object, v3.0, v3.1, v3.2),
        ]),
        _ => None,
    }
}

fn load_from_data(obj_set: obj::ObjSet) -> Result<Vec<(MeshBuilder<'static>, Option<String>)>, failure::Error> {
    // Takes a list of objects that contain geometries that contain shapes that contain
    // vertex/texture/normal indices into the main list of vertices, and converts to 
    // MeshBuilders with Position, Normal, TexCoord.
    trace!("Loading mesh");
    let mut objects = vec![];

    for object in obj_set.objects {
        for geometry in &object.geometry {
            let mut builder = MeshBuilder::new();

            let mut indices = Vec::new();

            geometry
                .shapes
                .iter()
                .for_each(|shape| {
                    if let obj::Primitive::Triangle(v1, v2, v3) = shape.primitive {
                        indices.push(v1);
                        indices.push(v2);
                        indices.push(v3);
                    }
                });

            let positions = object.vertices
                .iter()
                .map(|vertex| {
                    Position([vertex.x as f32, vertex.y as f32, vertex.z as f32])
                })
                .collect::<Vec<_>>();

            trace!("Loading normals");
            let normals = indices
                .iter()
                .map(|index| index.2
                    .map(|i| {
                        let normal: obj::Normal = object.normals[i];
                        Normal([normal.x as f32, normal.y as f32, normal.z as f32])
                    })
                    .unwrap_or(Normal([0.0, 0.0, 0.0]))
                )
                .collect::<Vec<_>>();

            let tex_coords = indices
                .iter()
                .map(|index| index.1
                    .map(|i| {
                        let tvertex: obj::TVertex = object.tex_vertices[i];
                        TexCoord([tvertex.u as f32, tvertex.v as f32])
                    })
                    .unwrap_or(TexCoord([0.0, 0.0]))
                )
                .collect::<Vec<_>>();

            {
                let indices2 : Vec<u16> = indices.iter().map(|i| i.0 as u16).collect::<Vec<u16>>();
                builder.set_indices(indices2);
            }

            builder.add_vertices(positions);
            builder.add_vertices(normals);
            builder.add_vertices(tex_coords);

            // TODO: Add Material loading
            objects.push((builder, geometry.material_name.clone()))
        }
    }
    trace!("Loaded mesh");
    Ok(objects)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load_from_obj() {
        let tetra = b"v 1.000000 1.000000 1.000000\nv 2.000000 1.000000 1.000000\n
v 1.000000 2.000000 1.000000\nv 1.000000 1.000000 2.000000\n
vt 0.500000 0.500000\nvt 0.000000 1.000000\nvt 1.000000 1.000000\n
vn 0.000000 0.000000 1.000000\nvn 0.000000 1.000000 0.000000\nvn 0.000000 0.000000 -1.000000\nvn 0.000000 0.000000 -1.000000\n
s 1\nf 1/1/1 3/3/3 2/3/2\nf 1/1/1 4/2/4 2/3/2\nf 1/1/1 2/2/2 4/3/4\nf 2/1/2 3/2/2 4/3/4\n";
        let result = load_from_obj(tetra).ok().unwrap();
        assert_eq!(result.len(), 1);
        
    }
}