use crate::{info, println};

#[global_allocator]
pub static HEAP: embedded_alloc::TlsfHeap = embedded_alloc::TlsfHeap::empty();

pub const PAGE_SIZE: usize = 4096;

const ENTRY_COUNT: usize = 1024;

const PTE_V: usize = 1 << 0; // Valid
pub const PTE_U: usize = 1 << 8; // User
pub const PTE_R: usize = 1 << 1; // Read
pub const PTE_W: usize = 1 << 2; // Write
pub const PTE_X: usize = 1 << 3; // Execute

static mut CURRENT_PPN: usize = 0;

#[inline(always)]
pub fn allocate_page() -> *mut u8 {
    unsafe {
        let ppn = CURRENT_PPN;
        CURRENT_PPN += 1;
        let addr = ((ppn << 12) as *mut u8);
        addr.write_bytes(0, PAGE_SIZE);
        addr
    }
}

/// Initialize the memory related.
///
/// # Safety
///
/// This function must be called exactly ONCE.
#[inline(always)]
pub unsafe fn init_memory() {
    unsafe {
        HEAP.init(info::heap_start(), info::heap_size());
        CURRENT_PPN = info::stack_start().next_multiple_of(PAGE_SIZE) >> 12;
    }
}

/// Map a virtual address to a physical address with flags.
pub unsafe fn map_region(root_table: *mut usize, vaddr: usize, paddr: usize, flags: usize) {
    let vpn1 = (vaddr >> 22) & 0x3ff; // VPN[1]
    let vpn0 = (vaddr >> 12) & 0x3ff; // VPN[0]

    let l1_entry = unsafe { root_table.add(vpn1) };
    if *l1_entry & PTE_V == 0 {
        let ppn = allocate_page() as usize;
        *l1_entry = ppn | PTE_V;
    }
    let l2_table = (*l1_entry & !0xfff) as *mut usize;
    let l2_entry = unsafe { l2_table.add(vpn0) };
    assert_eq!(paddr & 0xfff, 0, "paddr must be page aligned");
    *l2_entry = paddr | flags | PTE_V;
}