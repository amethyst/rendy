bitflags! {
    /// Shader stages flags.
    pub struct ShaderStageFlags: u32 {
        /// Vertex shader.
        const VERTEX = 0x00000001;

        /// Tessellation control shader.
        const TESSELLATION_CONTROL = 0x00000002;

        /// Tessellation evaluation shader.
        const TESSELLATION_EVALUATION = 0x00000004;

        /// Geometry shader.
        const GEOMETRY = 0x00000008;

        /// Fragment shader.
        const FRAGMENT = 0x00000010;

        /// Compute shader.
        const COMPUTE = 0x00000020;

        /// All graphics shaders.
        const ALL_GRAPHICS = 0x0000001F;

        /// All shaders.
        const ALL = 0x7FFFFFFF;
    }
}
