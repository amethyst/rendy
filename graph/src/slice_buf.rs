use std::cell::UnsafeCell;
use std::alloc::{alloc, dealloc, Layout};
use std::ops::Drop;

pub struct SliceBuf<T> {
    segment_capacity: usize,
    inner: UnsafeCell<SliceBufInner<T>>,
}

pub struct SliceBufInner<T> {
    segments: Vec<Segment<T>>,
    current: Option<CurrentState>,
}

struct Segment<T> {
    data: *mut T,
    capacity: usize,
    used: usize,
}

struct CurrentState {
    segment_idx: usize,
    items: usize,
}

impl<T> SliceBufInner<T> {

    /// If there is already a partial segment, drop items in it.
    pub fn drop_current(&mut self) {
        if let Some(current) = self.current.take() {
            let segment = &mut self.segments[current.segment_idx];

            let mut curr_item = current.items;
            for _ in 0..current.items {
                unsafe {
                    let ptr = segment.data.add(curr_item);
                    std::ptr::drop_in_place(ptr);
                }
                curr_item += 1;
            }
        }
    }

}

impl<T> SliceBuf<T> {

    pub fn new() -> Self {
        SliceBuf::with_segment_capacity(1024)
    }

    pub fn with_segment_capacity(segment_capacity: usize) -> Self {
        Self {
            segment_capacity,
            inner: UnsafeCell::new(SliceBufInner {
                segments: Vec::new(),
                current: None,
            }),
        }
    }

    pub fn start(&self) {
        self.start_with_capacity(32)
    }

    pub fn start_with_capacity(&self, capacity: usize) {
        let inner = unsafe { &mut *self.inner.get() };
        inner.drop_current();

        let segments_len = inner.segments.len();
        if let Some(last) = inner.segments.last_mut() {
            if (last.capacity - last.used) >= capacity {
                inner.current = Some(CurrentState {
                    segment_idx: segments_len - 1,
                    items: 0,
                });
                return;
            }
        }

        let mut to_create_segment_size = self.segment_capacity;
        while to_create_segment_size < capacity {
            to_create_segment_size *= 2;
        }

        let layout = Layout::array::<T>(to_create_segment_size).unwrap();
        let ptr = unsafe {
            alloc(layout) as *mut T
        };
        inner.segments.push(Segment {
            data: ptr,
            capacity: to_create_segment_size,
            used: 0,
        });

        inner.current = Some(CurrentState {
            segment_idx: segments_len,
            items: 0,
        });
    }

    pub fn push(&self, item: T) {
        let inner = unsafe { &mut *self.inner.get() };

        if let Some(current) = &mut inner.current {
            let segment = &mut inner.segments[current.segment_idx];
            let idx = segment.used + current.items;

            // TODO expand into new segment if too large
            assert!(idx < segment.capacity);

            unsafe {
                std::ptr::write(segment.data.add(idx), item);
            }
            current.items += 1;
        } else {
            panic!();
        }
    }

    pub fn commit<'a>(&'a self) -> &'a [T] {
        let inner = unsafe { &mut *self.inner.get() };
        let current = inner.current.take().unwrap();
        let segment = &mut inner.segments[current.segment_idx];

        unsafe {
            let base_ptr = segment.data.add(segment.used);
            let size = current.items;
            let slice = std::ptr::slice_from_raw_parts(base_ptr, size);

            segment.used += size;
            &*slice
        }
    }

    pub fn clear(&mut self) {
        let inner = unsafe { &mut *self.inner.get() };
        inner.drop_current();

        for segment in inner.segments.iter_mut() {
            for idx in 0..segment.used {
                unsafe {
                    let ptr = segment.data.add(idx);
                    std::ptr::drop_in_place(&mut *ptr);
                }
            }

            let layout = Layout::array::<T>(segment.capacity).unwrap();
            unsafe {
                dealloc(segment.data as *mut u8, layout);
            }
        }
    }

}

impl<T> Drop for SliceBuf<T> {
    fn drop(&mut self) {
        self.clear()
    }
}
