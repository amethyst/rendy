

extern crate ash;
extern crate failure;
extern crate rendy;

use ash::{
    version::V1_0,
};

use failure::Error;

use rendy::{
    command::Frames,
    factory::{Factory, Renderer, RendererDesc},
    wsi::Target,
    resource::Buffer,
    mesh::Mesh,
};

struct SimpleRenderer {
    vertices: Option<rendy::resource::Buffer>,
}

struct SimpleRendererDesc;

impl Renderer<Factory<V1_0>, ()> for SimpleRenderer {
    type Desc = SimpleRendererDesc;
    fn run(&mut self, factory: &mut Factory<V1_0>, data: &mut (), frames: &mut Frames) {

    }
}

impl RendererDesc<Factory<V1_0>, ()> for SimpleRendererDesc {
    type Renderer = SimpleRenderer;

    fn build(self, targets: Vec<Target>, factory: &mut Factory<V1_0>, data: &mut ()) -> SimpleRenderer {
        SimpleRenderer {
            vertices: None,
        }
    }
}

fn main() -> Result<(), Error> {
    Ok(())
}
