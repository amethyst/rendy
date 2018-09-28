bitflags! {
    /// Bitmask specifying intended usage of an image.
    /// See Vulkan docs for detailed info:
    /// <https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkImageUsageFlagBits.html>
    #[repr(transparent)]
    pub struct UsageFlags: u32 {
        /// Specifies that the image can be used as the source of a transfer command.
        const TRANSFER_SRC = 0x00000001;

        /// Specifies that the image can be used as the destination of a transfer command.
        const TRANSFER_DST = 0x00000002;

        /// Specifies that the image can be used to create a `ImageView` suitable for occupying a descriptor set slot either of
        /// type `SAMPLED_IMAGE` or `COMBINED_IMAGE_SAMPLER`, and be sampled by a shader.
        const SAMPLED = 0x00000004;

        /// Specifies that the image can be used to create a `ImageView` suitable for occupying a descriptor set slot of type `STORAGE_IMAGE`.
        const STORAGE = 0x00000008;

        /// Specifies that the image can be used to create a `ImageView` suitable for use as a color or resolve attachment in a `Framebuffer`.
        const COLOR_ATTACHMENT = 0x00000010;

        /// Specifies that the image can be used to create a `ImageView` suitable for use as a depth/stencil attachment in a `Framebuffer`.
        const DEPTH_STENCIL_ATTACHMENT = 0x00000020;

        /// Specifies that the memory bound to this image will have been allocated with the VK_MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT
        /// (see <https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/vkspec.html#memory> for more detail).
        /// This bit can be set for any image that can be used to create a `ImageView` suitable for use as a color, resolve, depth/stencil, or input attachment.
        const TRANSIENT_ATTACHMENT = 0x00000040;

        /// Specifies that the image can be used to create a `ImageView` suitable for occupying descriptor set slot of type `INPUT_ATTACHMENT`;
        /// be read from a shader as an input attachment; and be used as an input attachment in a framebuffer.
        const INPUT_ATTACHMENT = 0x00000080;
    }
}
