use alloc::boxed::Box;
use core::alloc::Layout;
use core::ptr::{read_volatile, write_volatile};
use riscv::register::satp;
use sbi_rt::Physical;

#[global_allocator]
pub static HEAP: embedded_alloc::TlsfHeap = embedded_alloc::TlsfHeap::empty();

const PAGE_SIZE: usize = 4096;
const L1_PAGE_SIZE: usize = 2 * 1024 * 1024; // 2 MiB
const L2_PAGE_SIZE: usize = 1 * 1024 * 1024 * 1024; // 1 GiB

const ENTRY_COUNT: usize = 512;

const PTE_V: u64 = 1 << 0; // Valid
const PTE_R: u64 = 1 << 1; // Read
const PTE_W: u64 = 1 << 2; // Write
const PTE_X: u64 = 1 << 3; // Execute

#[repr(align(4096))]
struct PageTable([u64; ENTRY_COUNT]);

/// Initialize the heap allocator.
///
/// # Safety
///
/// This function must be called exactly ONCE.
#[inline(always)]
pub unsafe fn init_heap() {
    unsafe { HEAP.init(crate::info::heap_start(), crate::info::heap_size()) }
}

fn allocate_page_table() -> &'static mut PageTable {
    Box::leak(Box::new(PageTable([0; ENTRY_COUNT])))
}

fn map_1gib_region(root_table: &mut PageTable, vaddr: usize, paddr: u64, flags: u64) {
    let vpn2 = vaddr >> 30 & 0x1FF;
    let l2_entry = &mut root_table.0[vpn2];
    *l2_entry = paddr | flags | PTE_V;
}

fn map_2mib_region(root_table: &mut PageTable, vaddr: usize, paddr: u64, flags: u64) {
    let vpn1 = vaddr >> 21 & 0x1FF;
    let vpn2 = vaddr >> 30 & 0x1FF;

    let l2_entry = &mut root_table.0[vpn2];
    if *l2_entry & PTE_V == 0 {
        *l2_entry = allocate_page_table() as *mut PageTable as u64 | PTE_V;
    }
    let l1_table = unsafe { &mut *((*l2_entry & 0xFFFF_FFFF_F000) as *mut PageTable) };

    let l1_entry = &mut l1_table.0[vpn1];
    *l1_entry = paddr | flags | PTE_V;
}

fn map_4k_region(root_table: &mut PageTable, vaddr: usize, paddr: u64, flags: u64) {
    let vpn = [
        (vaddr >> 12) & 0x1FF, // VPN[0]
        (vaddr >> 21) & 0x1FF, // VPN[1]
        (vaddr >> 30) & 0x1FF, // VPN[2]
    ];

    let l2_entry = &mut root_table.0[vpn[2]];
    if *l2_entry & PTE_V == 0 {
        *l2_entry = allocate_page_table() as *mut PageTable as u64 | PTE_V;
    }
    let l1_table = unsafe { &mut *((*l2_entry & 0xFFFF_FFFF_F000) as *mut PageTable) };

    let l1_entry = &mut l1_table.0[vpn[1]];
    if *l1_entry & PTE_V == 0 {
        *l1_entry = allocate_page_table() as *mut PageTable as u64 | PTE_V;
    }
    let l0_table = unsafe { &mut *((*l1_entry & 0xFFFF_FFFF_F000) as *mut PageTable) };

    let l0_entry = &mut l0_table.0[vpn[0]];
    *l0_entry = paddr | flags | PTE_V;
}

fn map_region(root_table: &mut PageTable, vaddr: usize, paddr: u64, flags: u64, size: usize) {
    let mut vaddr = vaddr;
    let mut paddr = paddr;
    let mut size = size;

    assert_eq!(size % PAGE_SIZE, 0, "size must be page aligned");

    while size > 0 {
        if vaddr % L2_PAGE_SIZE == 0 && paddr % L2_PAGE_SIZE as u64 == 0 && size >= L2_PAGE_SIZE {
            map_1gib_region(root_table, vaddr, paddr, flags);
            vaddr += L2_PAGE_SIZE;
            paddr += L2_PAGE_SIZE as u64;
            size -= L2_PAGE_SIZE;
        } else if vaddr % L1_PAGE_SIZE == 0
            && paddr % L1_PAGE_SIZE as u64 == 0
            && size >= L1_PAGE_SIZE
        {
            map_2mib_region(root_table, vaddr, paddr, flags);
            vaddr += L1_PAGE_SIZE;
            paddr += L1_PAGE_SIZE as u64;
            size -= L1_PAGE_SIZE;
        } else {
            map_4k_region(root_table, vaddr, paddr, flags);
            vaddr += PAGE_SIZE;
            paddr += PAGE_SIZE as u64;
            size -= PAGE_SIZE;
        }
    }
}

pub fn create_guest_page_table(mut paddr_start: u64) {
    let mut root_table = allocate_page_table();

    // 0x8000_0000-0xFFFF_0000 2GiB
    const GUEST_MEM_SIZE: usize = 0xFFFF_0000 - 0x8000_0000;
    let ram: Physical<&mut [u8]> = Physical::new(
        GUEST_MEM_SIZE,
        paddr_start as usize,
        (paddr_start >> 32) as usize,
    );
    map_region(
        &mut root_table,
        0x8000_0000,
        paddr_start,
        PTE_R | PTE_W,
        GUEST_MEM_SIZE,
    );
    paddr_start += GUEST_MEM_SIZE as u64;

    // ROM 256MiB 0x2000_0000..0x3000_0000
    const GUEST_ROM_SIZE: usize = 0x3000_0000 - 0x2000_0000;
    let rom: Physical<&mut [u8]> = Physical::new(
        GUEST_ROM_SIZE,
        paddr_start as usize,
        (paddr_start >> 32) as usize,
    );
    map_region(
        &mut root_table,
        0x2000_0000,
        paddr_start,
        PTE_R,
        GUEST_ROM_SIZE,
    );
}
