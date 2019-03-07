//! Loading mesh data from obj format.

use {
    crate::{
        mesh::MeshBuilder,
        vertex::{Normal, PosNormTex, Position, TexCoord},
    },
    wavefront_obj::obj,
};

/// Load mesh data from obj.
pub fn load_from_obj(bytes: &[u8], _: ()) -> Result<MeshBuilder<'static>, failure::Error> {
    let string = std::str::from_utf8(bytes)?;
    let set = obj::parse(string).map_err(|e| {
        failure::format_err!(
            "Error during parsing obj-file at line '{}': {}",
            e.line_number,
            e.message
        )
    })?;
    let posnormtex = from_data(set);
    Ok(MeshBuilder::new().with_vertices(posnormtex))
}

fn convert(
    object: &obj::Object,
    vi: obj::VertexIndex,
    ti: Option<obj::TextureIndex>,
    ni: Option<obj::NormalIndex>,
) -> PosNormTex {
    PosNormTex {
        position: {
            let vertex: obj::Vertex = object.vertices[vi];
            Position([vertex.x as f32, vertex.y as f32, vertex.z as f32])
        },
        normal: ni
            .map(|i| {
                let normal: obj::Normal = object.normals[i];
                Normal([normal.x as f32, normal.y as f32, normal.z as f32])
            })
            .unwrap_or(Normal([0.0, 0.0, 0.0])),
        tex_coord: ti
            .map(|i| {
                let tvertex: obj::TVertex = object.tex_vertices[i];
                TexCoord([tvertex.u as f32, tvertex.v as f32])
            })
            .unwrap_or(TexCoord([0.0, 0.0])),
    }
}

fn convert_primitive(object: &obj::Object, prim: &obj::Primitive) -> Option<[PosNormTex; 3]> {
    match *prim {
        obj::Primitive::Triangle(v1, v2, v3) => Some([
            convert(object, v1.0, v1.1, v1.2),
            convert(object, v2.0, v2.1, v2.2),
            convert(object, v3.0, v3.1, v3.2),
        ]),
        _ => None,
    }
}

fn from_data(obj_set: obj::ObjSet) -> Vec<PosNormTex> {
    // Takes a list of objects that contain geometries that contain shapes that contain
    // vertex/texture/normal indices into the main list of vertices, and converts to a
    // flat vec of `PosNormTex` objects.
    // TODO: Doesn't differentiate between objects in a `*.obj` file, treats
    // them all as a single mesh.
    let vertices = obj_set.objects.iter().flat_map(|object| {
        object.geometry.iter().flat_map(move |geometry| {
            geometry
                .shapes
                .iter()
                .filter_map(move |s| convert_primitive(object, &s.primitive))
        })
    });

    let mut result = Vec::new();
    for vvv in vertices {
        result.push(vvv[0]);
        result.push(vvv[1]);
        result.push(vvv[2]);
    }
    result
}
