use std::borrow::Borrow;

use rendy_core::hal;
use hal::pass::ATTACHMENT_UNUSED;
use hal::format::Format;
use rendy_resource::Layout;

use crate::resource::render_pass::SubpassDesc;

#[derive(Debug, PartialEq, Eq, Hash)]
struct AttachmentCompatibilityData {
    format: Option<Format>,
    samples: u8,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct SubpassCompatibilityData {
    depth_stencil: bool,
    colors: usize,
    inputs: usize,
    resolves: usize,
    preserves: usize,
}

/// If two subpasses are compatible, their CompatibilityData will both be equal
/// and hash to the same value.
#[derive(Default, Debug, PartialEq, Eq, Hash)]
pub struct RenderPassCompatibilityData {
    attachments: Vec<AttachmentCompatibilityData>,
    subpasses: Vec<SubpassCompatibilityData>,
    dependencies: Vec<hal::pass::SubpassDependency>,
    attachment_references: Vec<usize>,
}

fn get_head_used(data: &[(usize, Layout)]) -> &[(usize, Layout)] {
    let mut num_unused = 0;
    for entry in data.iter().rev() {
        if entry.0 != ATTACHMENT_UNUSED {
            break;
        }
        num_unused += 1;
    }
    &data[0..(data.len()-num_unused)]
}

impl RenderPassCompatibilityData {

    pub fn new<AI, AB, SI, SB, DI, DB>(
        attachments: AI,
        subpasses: SI,
        dependencies: DI,
    ) -> Self
    where
        AI: IntoIterator<Item = AB>,
        SI: IntoIterator<Item = SB>,
        DI: IntoIterator<Item = DB>,
        AB: Borrow<hal::pass::Attachment>,
        SB: Borrow<super::SubpassDesc>,
        DB: Borrow<hal::pass::SubpassDependency>,
    {
        let mut data = RenderPassCompatibilityData::default();

        for attachment in attachments.into_iter() {
            let attachment = attachment.borrow();
            data.attachments.push(AttachmentCompatibilityData {
                format: attachment.format,
                samples: attachment.samples,
            });
        }

        for subpass in subpasses.into_iter() {
            let subpass = subpass.borrow();
            data.subpasses.push(SubpassCompatibilityData {
                depth_stencil: {
                    if let Some((idx, _layout)) = subpass.depth_stencil {
                        data.attachment_references.push(idx);
                        true
                    } else {
                        false
                    }
                },
                colors: {
                    let head = get_head_used(&subpass.colors);
                    for (idx, _layout) in head {
                        data.attachment_references.push(*idx);
                    }
                    head.len()
                },
                inputs: {
                    let head = get_head_used(&subpass.inputs);
                    for (idx, _layout) in head {
                        data.attachment_references.push(*idx);
                    }
                    head.len()
                },
                resolves: {
                    let head = get_head_used(&subpass.resolves);
                    for (idx, _layout) in head {
                        data.attachment_references.push(*idx);
                    }
                    head.len()
                },
                preserves: {
                    for idx in subpass.preserves.iter() {
                        assert!(*idx == ATTACHMENT_UNUSED);
                        data.attachment_references.push(*idx);
                    }
                    subpass.preserves.len()
                },
            });
        }

        for dependency in dependencies.into_iter() {
            let dependency = dependency.borrow();
            data.dependencies.push(dependency.clone());
        }

        data
    }

}
