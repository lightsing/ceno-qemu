use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use virtio_drivers::{BufferDirection, Hal, PhysAddr, PAGE_SIZE};
use crate::memory;

pub struct HalImpl;

unsafe impl Hal for HalImpl {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        let layout = Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).expect("invalid layout");
        let ptr = unsafe { memory::HEAP.alloc_zeroed(layout) };
        (ptr as PhysAddr, NonNull::new(ptr).expect("invalid pointer"))
    }

    unsafe fn dma_dealloc(paddr: PhysAddr, vaddr: NonNull<u8>, pages: usize) -> i32 {
        let layout = Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).expect("invalid layout");
        memory::HEAP.dealloc(vaddr.as_ptr(), layout);
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        NonNull::new(paddr as _).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        buffer.as_ptr() as *mut u8 as PhysAddr
    }

    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {}
}