use std::marker::PhantomData;

use crate::factory::Factory;
use crate::scheduler::{
    ImageId,
    builder::ProceduralBuilder,
    resources::ImageInfo,
    interface::GraphCtx,
};
use super::super::parameter::{Parameter, ParameterStore};
use super::super::builder::{Node, GraphConstructCtx};

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
    type Result = Parameter<ImageId>;
    fn construct(
        &mut self,
        factory: &mut Factory<B>,
        ctx: &mut GraphConstructCtx<B>,
        _store: &ParameterStore,
    ) -> Result<ImageId, ()> {
        let image = ctx.create_image(self.info);
        Ok(image)
    }
}
