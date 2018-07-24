
/// Tracks lifetime of resources.
/// Preserves them from being destroyed until device stop using them.
pub trait Track<T> {
    fn track(&T);
}

impl Track<Item<T>> for Resources<T> {
    fn track(item: &Item<T>) {
        assert!(self.terminal.owns(item.inner));
    }
}
