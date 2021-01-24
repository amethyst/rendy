//! This module defines types to reason about what resources referenced in what submissions.
//! How commands from those submissions access resources.
//! This information allows to derive synchronization required.

mod link;

use std::ops::BitOr;

use derivative::Derivative;

pub use self::link::{Link, LinkNode};
use crate::resource::Resource;

/// This type corresponds to resource category.
/// All resources from the same category must be accessed as permitted by links of the chain.
#[derive(Clone, Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct Chain<R: Resource> {
    links: Vec<Link<R>>,
}

impl<R> Chain<R>
where
    R: Resource,
{
    /// Get links slice
    pub fn links(&self) -> &[Link<R>] {
        &self.links
    }

    /// Get links slice
    pub fn last_link_mut(&mut self) -> Option<&mut Link<R>> {
        self.links.last_mut()
    }

    /// Add new link to the chain.
    pub fn add_link(&mut self, link: Link<R>) -> &mut Link<R> {
        self.links.push(link);
        self.links.last_mut().unwrap()
    }

    /// Get total usage.
    pub fn usage(&self) -> R::Usage {
        self.links
            .iter()
            .map(Link::usage)
            .fold(R::no_usage(), BitOr::bitor)
    }
}
