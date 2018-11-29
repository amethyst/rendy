
use super::usage::MultiShot;

/// Command buffer state in which all buffers start.
/// Resetting also moves buffer to this state.
#[derive(Clone, Copy, Debug, Default)]
pub struct InitialState;

/// Command buffer in recording state could be populated with commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct RecordingState<U = MultiShot, P = ()>(pub U, pub P);

/// Command buffer in executable state can be submitted.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExecutableState<U = MultiShot, P = ()>(pub U, pub P);

/// Command buffer in pending state are submitted to the device.
/// Command buffer in pending state must never be invalidated or reset because device may read it at the moment.
/// Proving device is done with buffer requires nontrivial strategies.
/// Therefore moving buffer from pending state requires `unsafe` method.
#[derive(Clone, Copy, Debug, Default)]
pub struct PendingState<N = ExecutableState>(pub N);

/// One-shot buffers move to invalid state after execution.
/// Invalidating any resource referenced in any command recorded to the buffer implicitly move it to the invalid state.
#[derive(Clone, Copy, Debug, Default)]
pub struct InvalidState;

