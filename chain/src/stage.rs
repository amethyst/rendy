
use ash::vk::PipelineStageFlags;

/// Graphics pipeline stage.
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum GraphicsPipelineStage {
    /// Pseudo-stage that comes before any operations.
    TopOfPipe,

    /// Indirect buffer reading stage.
    DrawIndirect,

    /// Vertex input consuming stage.
    VertexInput,

    /// Vertex shader execution stage.
    VertexShader,

    /// ???
    TessellationControlShader,

    /// ???
    TessellationEvaluationShader,

    /// Geometry shader execution stage.
    GeometryShader,

    /// First fragment depth-testing stage.
    EarlyFragmentTests,

    /// Fragment shader execution stage.
    FragmentShader,

    /// Last fragment depth-testing stage.
    LateFragmentTests,

    /// Color attachment writing stage.
    ColorAttachmentOutput,

    /// Pseudo-stage that comes after all operations.
    BottomOfPipe,
}

impl From<GraphicsPipelineStage> for PipelineStageFlags {
    fn from(stage: GraphicsPipelineStage) -> Self {
        match stage {
            GraphicsPipelineStage::TopOfPipe => Self::TOP_OF_PIPE,
            GraphicsPipelineStage::DrawIndirect => Self::DRAW_INDIRECT,
            GraphicsPipelineStage::VertexInput => Self::VERTEX_INPUT,
            GraphicsPipelineStage::VertexShader => Self::VERTEX_SHADER,
            GraphicsPipelineStage::TessellationControlShader => Self::TESSELLATION_CONTROL_SHADER,
            GraphicsPipelineStage::TessellationEvaluationShader => Self::TESSELLATION_EVALUATION_SHADER,
            GraphicsPipelineStage::GeometryShader => Self::GEOMETRY_SHADER,
            GraphicsPipelineStage::EarlyFragmentTests => Self::EARLY_FRAGMENT_TESTS,
            GraphicsPipelineStage::FragmentShader => Self::FRAGMENT_SHADER,
            GraphicsPipelineStage::LateFragmentTests => Self::LATE_FRAGMENT_TESTS,
            GraphicsPipelineStage::ColorAttachmentOutput => Self::COLOR_ATTACHMENT_OUTPUT,
            GraphicsPipelineStage::BottomOfPipe => Self::BOTTOM_OF_PIPE,
        }
    }
}

/// Compute pipeline stage.
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum ComputePipelineStage {
    /// Pseudo-stage that comes before any operations.
    TopOfPipe,

    /// Indirect buffer reading stage.
    DrawIndirect,

    /// Compute shader execution stage.
    ComputeShader,

    /// Pseudo-stage that comes after all operations.
    BottomOfPipe,
}

impl From<ComputePipelineStage> for PipelineStageFlags {
    fn from(stage: ComputePipelineStage) -> Self {
        match stage {
            ComputePipelineStage::TopOfPipe => Self::TOP_OF_PIPE,
            ComputePipelineStage::DrawIndirect => Self::DRAW_INDIRECT,
            ComputePipelineStage::ComputeShader => Self::COMPUTE_SHADER,
            ComputePipelineStage::BottomOfPipe => Self::BOTTOM_OF_PIPE,
        }
    }
}

/// Transfer pipeline stage.
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum TransferPipelineStage {
    /// Pseudo-stage that comes before any operations.
    TopOfPipe,

    /// Transfer operation execution stage.
    Transfer,

    /// Pseudo-stage that comes after all operations.
    BottomOfPipe,
}

impl From<TransferPipelineStage> for PipelineStageFlags {
    fn from(stage: TransferPipelineStage) -> Self {
        match stage {
            TransferPipelineStage::TopOfPipe => Self::TOP_OF_PIPE,
            TransferPipelineStage::Transfer => Self::TRANSFER,
            TransferPipelineStage::BottomOfPipe => Self::BOTTOM_OF_PIPE,
        }
    }
}

/// Pseudo-stage in which host operations are performed.
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct HostStage;

impl From<HostStage> for PipelineStageFlags {
    fn from(_: HostStage) -> Self {
        PipelineStageFlags::HOST
    }
}
