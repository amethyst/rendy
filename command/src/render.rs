
/// Render manages command execution on GPU
pub struct Render<Q> {
    families: Vec<Family<Q>>,
}

impl<Q> Render<Q> {
    /// Access queue families.
    pub fn families(&mut self) -> &mut [Family<Q>] {
        &mut self.families
    }
}
