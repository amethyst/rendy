use std::any::{Any, TypeId};
use std::collections::{BTreeSet, BTreeMap};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// Contains types and traits for declaring a schedule.
pub mod interface;
pub use interface::{ImageId, BufferId, EntityId};

/// Contains types and traits for the interface between the schedule builder and
/// the scheduler implementation itself.
pub mod input;

/// Implementation of a simple procedural builder.
///
/// Implements the interface from the `interface` module for declaring the
/// schedule, implements the interface from the `input` module for providing
/// that declared schedule to the scheduler inplementation itself.
pub mod procedural;

/// Data structures for declaring information related to resources.
pub mod resources;

/// The implementation of the scheduler algorithm itself.
///
/// Consumes input through the interface defined in the `input` module.
mod scheduler;
pub use scheduler::{Scheduler, ScheduleEntry, RenderPassData, RenderPass, ExternalSignal};

pub mod sync;

pub mod schedule_iterator;

pub trait SchedulerTypes {
    type Semaphore;
    type Image;
    type Buffer;
    type NodeValue;
}

enum IterEither<A, B> {
    A(A),
    B(B),
}
impl<A: Iterator<Item = T>, B: Iterator<Item = T>, T> Iterator for IterEither<A, B> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IterEither::A(inner) => inner.next(),
            IterEither::B(inner) => inner.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        builder::ProceduralBuilder,
        scheduler::Scheduler,
        interface::{GraphCtx, EntityCtx, PassEntityCtx},
        resources::{ImageInfo, ImageMode},
    };

    use rendy_core::hal;

    #[test]
    fn simple() {
        simple_logger::init().unwrap();

        //let config: Config = Default::default();
        //let rendy = Rendy::<VulkanBackend>::init(&config).unwrap();

        let mut builder: ProceduralBuilder = ProceduralBuilder::new();
        let mut scheduler = Scheduler::new();

        let i1 = builder.create_image(ImageInfo {
            kind: Some(hal::image::Kind::D2(100, 100, 1, 1)),
            levels: 1,
            format: hal::format::Format::Rgb8Uint,
            mode: ImageMode::Clear {
                transient: false,
                clear: hal::command::ClearValue {
                    color: hal::command::ClearColor {
                        uint32: [0, 0, 0, 0],
                    },
                },
            },
        });
        let i2 = builder.create_image(ImageInfo {
            kind: Some(hal::image::Kind::D2(100, 100, 1, 1)),
            levels: 1,
            format: hal::format::Format::Rgb8Uint,
            mode: ImageMode::Clear {
                transient: false,
                clear: hal::command::ClearValue {
                    color: hal::command::ClearColor {
                        uint32: [0, 0, 0, 0],
                    },
                },
            },
        });

        let p1;
        {
            builder.start_pass();
            p1 = builder.id();
            builder.use_color(0, i1, false);
            builder.use_color(1, i2, false);
            builder.commit();
        }

        {
            builder.start_pass();
            builder.use_color(0, i1, false);
            builder.commit();
        }

        {
            builder.start_pass();
            builder.use_color(0, i2, false);
            builder.commit();
        }

        let p3;
        {
            builder.start_pass();
            p3 = builder.id();
            builder.use_color(0, i1, false);
            builder.commit();
        }

        builder.mark_render_pass(p1, p3);

        let sched_input = builder.make_scheduler_input();
        scheduler.plan(&sched_input);

        println!("Scheduled order: {:?}", scheduler.scheduled_order);

        panic!("yay!");

    }

}
