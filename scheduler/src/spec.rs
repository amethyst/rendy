use rendy_core::hal;

/// https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkAccessFlagBits.html
/// Table 1
pub fn filter_image_access_for_pipeline_stages(
    stages: hal::pso::PipelineStage,
    access: hal::image::Access,
) -> hal::image::Access {
    use hal::pso::PipelineStage as S;
    use hal::image::Access as A;

    let mut out = hal::image::Access::empty();
    if stages.intersects(S::TOP_OF_PIPE | S::BOTTOM_OF_PIPE) {
        return out;
    }

    if stages.intersects(S::FRAGMENT_SHADER) {
        out |= access & A::INPUT_ATTACHMENT_READ;
    }
    if stages.intersects(S::COLOR_ATTACHMENT_OUTPUT) {
        out |= access & (A::COLOR_ATTACHMENT_READ | A::COLOR_ATTACHMENT_WRITE);
    }
    if stages.intersects(S::EARLY_FRAGMENT_TESTS | S::LATE_FRAGMENT_TESTS) {
        out |= access & (A::DEPTH_STENCIL_ATTACHMENT_READ | A::DEPTH_STENCIL_ATTACHMENT_WRITE);
    }
    if stages.intersects(S::TRANSFER) {
        out |= access & (A::TRANSFER_READ | A::TRANSFER_WRITE);
    }
    if stages.intersects(S::HOST) {
        out |= access & (A::HOST_READ | A::HOST_WRITE);
    }
    out |= access & (A::MEMORY_READ | A::MEMORY_WRITE);

    out
}
