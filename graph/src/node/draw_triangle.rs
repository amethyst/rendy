use rendy_core::hal;

use crate::factory::Factory;
use crate::scheduler::ImageId;
use crate::parameter::{Parameter, ParameterStore};
use crate::resource::{Buffer, BufferInfo};
use crate::memory::Dynamic;
use rendy_mesh::{PosColor, AsVertex};

pub struct DrawTriangle<B: hal::Backend> {
    vbuf: Escape<Buffer<B>>,
}

impl<B: hal::Backend> DrawTriangle<B> {

    pub fn new(factory: &Factory<B>) {

        let vbuf_size = PosColor::vertex().stride as u64 * 3;

        let mut vbuf = factory
            .create_buffer(
                BufferInfo {
                    size: vbuf_size,
                    usage: hal::buffer::Usage::VERTEX,
                },
                Dynamic,
            )
            .unwrap();

        unsafe {
            // Fresh buffer.
            factory
                .upload_visible_buffer(
                    &mut vbuf,
                    0,
                    &[
                        PosColor {
                            position: [0.0, -0.5, 0.0].into(),
                            color: [1.0, 0.0, 0.0, 1.0].into(),
                        },
                        PosColor {
                            position: [0.5, 0.5, 0.0].into(),
                            color: [0.0, 1.0, 0.0, 1.0].into(),
                        },
                        PosColor {
                            position: [-0.5, 0.5, 0.0].into(),
                            color: [0.0, 0.0, 1.0, 1.0].into(),
                        },
                    ],
                )
                .unwrap();
        }

        DrawTriangle {
            vbuf,
        }
    }

}
