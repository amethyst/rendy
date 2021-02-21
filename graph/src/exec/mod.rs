use std::marker::PhantomData;

use crate::core::hal;
use crate::resource::Image;

pub struct ExecCtx<B: hal::Backend> {
    phantom: PhantomData<B>,
}

impl<B: hal::Backend> ExecCtx<B> {
    pub fn get_image(&self) -> Image<B> {
        todo!()
    }
}
