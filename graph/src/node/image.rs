use std::marker::PhantomData;

use crate::factory::Factory;
use crate::scheduler::{
    ImageId,
    builder::ProceduralBuilder,
    resources::ImageInfo,
    interface::GraphCtx,
};
use crate::parameter::{Parameter, ParameterStore};
use crate::Node;
use crate::graph::GraphConstructCtx;

use rendy_core::hal;

pub struct Image<B: hal::Backend> {
    backend: PhantomData<B>,
    info: ImageInfo,
}

impl<B: hal::Backend> Image<B> {
    pub fn new(info: ImageInfo) -> Self {
        Image {
            backend: PhantomData,
            info,
        }
    }
}

impl<B: hal::Backend> Node<B> for Image<B> {
    type Argument = ();
    type Result = ImageId;
    fn construct(
        &mut self,
        factory: &Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        _arg: (),
    ) -> Result<ImageId, ()> {
        let image = ctx.create_image(self.info);
        Ok(image)
    }
}
