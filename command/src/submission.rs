


pub struct Submission<B> {
    waits: SmallVec<>,
    submits: SmallVec<Submit<B>>,
    signals: SmallVec<>,
}

