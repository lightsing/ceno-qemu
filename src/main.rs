#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use core::arch::asm;
use core::ptr;
use riscv::interrupt::{Exception, Interrupt, Trap};
use riscv::register::stvec::TrapMode;
use virtio_drivers::device::blk::SECTOR_SIZE;

mod info;
mod memory;
mod sbi;
mod virtio;

static mut ROOT_PAGE_TABLE: *mut usize = ptr::null_mut();

#[riscv_rt::entry]
fn main() -> ! {
    let dtb_address: *const u8;
    unsafe {
        asm!("mv {}, a1", out(reg) dtb_address);
    }
    println!("-------------------------------CENO QEMU RISCV32--------------------------------");
    unsafe {
        memory::init_memory();
    }

    let mut devices = unsafe { info::detect_devices(dtb_address) };

    println!("--------------------------------------ELF---------------------------------------");
    let sectors = devices.elf_blk_device.capacity();
    let mut elf_buffer = Vec::with_capacity(sectors as usize * SECTOR_SIZE);
    elf_buffer.resize(sectors as usize * SECTOR_SIZE, 0);
    devices
        .elf_blk_device
        .read_blocks(0, elf_buffer.as_mut())
        .unwrap();
    let elf_buffer: &'static [u8] = elf_buffer.leak();
    let elf = goblin::elf::Elf::parse(elf_buffer).expect("Failed to parse ELF file");
    assert_eq!(elf.header.e_ident[goblin::elf::header::EI_CLASS], goblin::elf::header::ELFCLASS32, "Not a 32-bit ELF file");
    assert_eq!(elf.header.e_machine, goblin::elf::header::EM_RISCV, "Not a RISC-V ELF file");
    assert_eq!(elf.header.e_type, goblin::elf::header::ET_EXEC, "Not an executable ELF file");

    println!("Entry Point: {:#x}", elf.entry);

    unsafe { ROOT_PAGE_TABLE = memory::allocate_page() as *mut usize; }
    let root_table = unsafe { ROOT_PAGE_TABLE };

    for ph in elf.program_headers.iter().filter(|ph| ph.p_type == goblin::elf::program_header::PT_LOAD) {
        println!("vaddr: {:#x}, paddr: {:#x}, filesz: {:#x}, memsz: {:#x}", ph.p_vaddr, ph.p_paddr, ph.p_filesz, ph.p_memsz);
        let pages = ph.p_memsz as usize / memory::PAGE_SIZE + 1;
        let vaddr = ph.p_vaddr as usize;

        let mut flags = memory::PTE_U;
        if ph.is_read() {
            flags |= memory::PTE_R;
        }
        if ph.is_executable() {
            flags |= memory::PTE_X;
        }
        if ph.is_write() {
            flags |= memory::PTE_W;
        }

        let file_end = ph.p_offset as usize + ph.p_filesz as usize;
        for i in 0..pages {
            let page = memory::allocate_page();
            let file_offset = ph.p_offset as usize + i * memory::PAGE_SIZE;
            if file_offset < file_end {
                let end = core::cmp::min(file_offset + memory::PAGE_SIZE, file_end);
                let file_page = &elf_buffer[file_offset..end];
                unsafe {
                    ptr::copy_nonoverlapping(file_page.as_ptr(), page, file_page.len());
                }
            }
            unsafe { memory::map_region(root_table, vaddr + i * memory::PAGE_SIZE, page as usize, flags); }
        }
    }
    println!("mapping done");

    unsafe {
        riscv::register::stvec::write(handle_exception as usize, TrapMode::Direct);
        riscv::register::satp::set(riscv::register::satp::Mode::Sv32, 0, root_table as usize >> 12);
    }

    riscv::asm::sfence_vma_all();
    println!("page table enabled");
    riscv::register::sepc::write(elf.entry as usize);
    unsafe {
        riscv::register::sstatus::set_spp(riscv::register::sstatus::SPP::User);
        asm!("mv sp, {0}", in(reg) 0x80000000usize);
        asm!("sret");
    }
    sbi::shutdown();
}

#[no_mangle]
fn handle_exception() {
    println!("Trap!");
    let raw_trap = riscv::register::scause::read().cause();
    let standard_trap: Trap<Interrupt, Exception> = raw_trap.try_into().unwrap();
    let fault_addr = riscv::register::stval::read();

    match standard_trap {
        Trap::Exception(Exception::LoadFault) | Trap::Exception(Exception::StoreFault)=> {
            let page = memory::allocate_page();
            let vaddr = fault_addr & !(memory::PAGE_SIZE - 1);
            unsafe {
                memory::map_region(ROOT_PAGE_TABLE, vaddr, page as usize, memory::PTE_U | memory::PTE_R | memory::PTE_W);
            }
        }
        _ => {
            println!("Unhandled trap: {:?}", standard_trap);
        }
    }
}

