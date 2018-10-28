

extern crate ash;
extern crate failure;
extern crate rendy;

use ash::{
    version::V1_0,
};

use failure::Error;

use rendy::{
    frame::Frames,
    factory::Factory,
    renderer::{Renderer, RendererDesc},
    wsi::Target,
    mesh::Mesh,
};

struct SimpleRenderer {
    vertices: Option<Mesh>,
}

struct SimpleRendererDesc;

impl Renderer<()> for SimpleRenderer {
    type Desc = SimpleRendererDesc;
    fn run(&mut self, factory: &mut Factory, data: &mut (), frames: &mut Frames) {

    }
}

impl RendererDesc<()> for SimpleRendererDesc {
    type Renderer = SimpleRenderer;

    fn build(self, targets: Vec<Target>, factory: &mut Factory, data: &mut ()) -> SimpleRenderer {
        SimpleRenderer {
            vertices: None,
        }
    }
}

fn main() -> Result<(), Error> {
    Ok(())
}
