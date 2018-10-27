
use std::ffi::{CStr, c_void};

use ash::{
    version::{EntryV1_0, InstanceV1_0},
    vk::{
        SurfaceKHR,
        MacOSSurfaceCreateInfoMVK,
    },
    extensions::MacOSSurface,
};

use failure::Error;
use objc::runtime::{Object, BOOL, YES};
use winit::{Window, os::macos::WindowExt};

pub struct NativeSurface(MacOSSurface);

impl NativeSurface {
    pub fn name() -> &'static CStr {
        MacOSSurface::name()
    }

    pub fn new(entry: &impl EntryV1_0, instance: &impl InstanceV1_0) -> Result<Self, Vec<&'static str>> {
        MacOSSurface::new(entry, instance)
            .map(NativeSurface)
    }

    pub fn create_surface(&self, window: &Window) -> Result<SurfaceKHR, Error> {
        let surface = unsafe {
            let nsview = window.get_nsview();

            if nsview.is_null() {
                bail!("Window does not have a valid contentView");
            }

            put_metal_layer(nsview);

            self.0.create_mac_os_surface_mvk(
                &MacOSSurfaceCreateInfoMVK::builder()
                    .view(&*nsview)
                    .build(),
                None,
            )
        }?;

        trace!("Surface {:p} created", surface);
        Ok(surface)
    }
}

unsafe fn put_metal_layer(nsview: *mut c_void) {
    let class = class!(CAMetalLayer);
    let view: cocoa::base::id = ::std::mem::transmute(nsview);
    
    let is_layer: BOOL = msg_send![view, isKindOfClass:class];
    if is_layer == YES {
        return;
    }

    let layer: *mut Object = msg_send![view, layer];
    if !layer.is_null() && msg_send![layer, isKindOfClass:class] {
        return;
    }

    let layer: *mut Object = msg_send![class, new];
    msg_send![view, setLayer: layer];
    msg_send![view, retain];
}
