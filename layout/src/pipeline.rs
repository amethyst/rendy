use shaders::ShaderStageFlags;

#[derive(Clone, Copy, Debug)]
pub struct PushConstantRange {
    pub stage_flags: ShaderStageFlags,
    pub offset: u32,
    pub size: u32,
}

#[derive(Clone, Debug)]
pub struct PipelineLayoutCreateInfo<S, P> {
    pub sets: S,
    pub push: P,
}
