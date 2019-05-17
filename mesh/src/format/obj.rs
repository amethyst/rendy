//! Loading mesh data from obj format.

use {
    crate::{mesh::MeshBuilder, Normal, Position, TexCoord},
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
                        indices.push(v3);
                        indices.push(v2);
                        indices.push(v1);
                    }
                });
            // We can't use the vertices directly because we have per face normals and not per vertex normals in most obj files
            // TODO: Compress duplicates and return indices for indexbuffer.
            let positions = indices
                .iter()
                .map(|index| {
                    let vertex: obj::Normal = object.vertices[index.0];
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

            // builder.set_indices(indices.iter().map(|i| i.0 as u16).collect::<Vec<u16>>());

            debug_assert!(&normals.len() == &positions.len());
            debug_assert!(&tex_coords.len() == &positions.len());

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