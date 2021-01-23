//! This module defines types to reason about what resources referenced in what submissions.
//! How commands from those submissions access resources.
//! This information allows to derive synchronization required.

use std::collections::HashMap;

mod link;

use std::ops::BitOr;

use crate::{
    resource::{Buffer, Image, Resource},
    Id,
};

pub use self::link::{Link, LinkNode};

use derivative::Derivative;

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

    // /// Get link by index.
    // pub(crate) fn link(&self, index: usize) -> &Link<R> {
    //     &self.links[index]
    // }

    // /// Get link by index.
    // pub(crate) fn link_mut(&mut self, index: usize) -> &mut Link<R> {
    //     &mut self.links[index]
    // }

    // /// Get link by index.
    // pub(crate) fn next_link(&self, index: usize) -> &Link<R> {
    //     let index = (index + 1) % self.links.len();
    //     self.link(index)
    // }

    // /// Get link by index.
    // pub(crate) fn next_link_mut(&mut self, index: usize) -> &mut Link<R> {
    //     let index = (index + 1) % self.links.len();
    //     self.link_mut(index)
    // }

    // /// Get link by index.
    // pub(crate) fn prev_link(&self, index: usize) -> &Link<R> {
    //     let index = (index + self.links.len() - 1) % self.links.len();
    //     self.link(index)
    // }

    // /// Get link by index.
    // pub(crate) fn prev_link_mut(&mut self, index: usize) -> &mut Link<R> {
    //     let index = (index + self.links.len() - 1) % self.links.len();
    //     self.link_mut(index)
    // }

    /// Get total usage.
    pub fn usage(&self) -> R::Usage {
        self.links
            .iter()
            .map(Link::usage)
            .fold(R::no_usage(), BitOr::bitor)
    }
}

/// Type alias for map of chains by id for buffers.
pub(crate) type BufferChains = HashMap<Id, Chain<Buffer>>;

/// Type alias for map of chains by id for images.
pub(crate) type ImageChains = HashMap<Id, Chain<Image>>;
