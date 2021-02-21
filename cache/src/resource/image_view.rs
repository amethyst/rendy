use std::marker::PhantomData;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use rendy_core::{hal, Device, hal::device::Device as DeviceTrait};
use rendy_resource::{CreationError, SubresourceRange, ImageViewInfo};

use crate::{
    handle::{HasValue, HasKey},
    resource::{
        Managed,
        image::{ManagedImage, ImageHandle},
    },
};

pub type ManagedImageView<B> = Managed<ImageViewMarker<B>>;
pub struct ImageViewMarker<B>(PhantomData<B>) where B: hal::Backend;

impl<B> HasKey for ImageViewMarker<B> where B: hal::Backend {
    type Key = Arc<ImageViewKey<B>>;
}
impl<B> HasValue for ImageViewMarker<B> where B: hal::Backend {
    type Value = ManagedImageViewData<B>;
}

pub struct ImageViewKey<B> where B: hal::Backend {
    pub image: ImageHandle<B>,
    pub info: ImageViewInfo,
}
impl<B: hal::Backend> PartialEq for ImageViewKey<B> {
    fn eq(&self, rhs: &Self) -> bool {
        self.image == rhs.image
            && self.info == rhs.info
    }
}
impl<B: hal::Backend> Eq for ImageViewKey<B> {}
impl<B: hal::Backend> Hash for ImageViewKey<B> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.image.hash(hasher);
        self.info.hash(hasher);
    }
}

pub struct ManagedImageViewData<B>
where
    B: hal::Backend,
{
    raw: B::ImageView,
    key: Arc<ImageViewKey<B>>,
}

impl<B> ManagedImageViewData<B>
where
    B: hal::Backend,
{

    pub fn create(
        device: &Device<B>,
        image: &ManagedImage<B>,
        key: Arc<ImageViewKey<B>>,
    ) -> Result<Self, CreationError<hal::image::ViewError>>
    {
        // TODO: assert device ownership
        // TODO: assert compatibility

        assert!(image.handle() == key.image);

        let view = unsafe {
            device
                .create_image_view(
                    image.raw(),
                    key.info.view_kind,
                    key.info.format,
                    key.info.swizzle,
                    SubresourceRange {
                        aspects: key.info.range.aspects.clone(),
                        layers: key.info.range.layers.clone(),
                        levels: key.info.range.levels.clone(),
                    },
                )
                .map_err(CreationError::Create)?
        };

        let data = Self {
            raw: view,
            key,
        };

        Ok(data)
    }

}

impl<B: hal::Backend> ManagedImageView<B> {

    pub fn raw(&self) -> &B::ImageView {
        &self.inner.value.raw
    }

}
