#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use core::arch::asm;
use virtio_drivers::device::blk::SECTOR_SIZE;
use virtio_drivers::transport::Transport;

mod info;
mod memory;
mod sbi;
mod virtio;

#[riscv_rt::entry]
fn main() -> ! {
    let dtb_address: *const u8;
    unsafe {
        asm!("mv {}, a1", out(reg) dtb_address);
    }
    println!("-------------------------------CENO QEMU RISCV32--------------------------------");
    unsafe {
        memory::init_heap();
    }

    let mut devices = unsafe { info::detect_devices(dtb_address) };

    let sectors = devices.elf_blk_device.capacity();
    let mut elf_buffer = Vec::with_capacity(sectors as usize * SECTOR_SIZE);
    elf_buffer.resize(sectors as usize * SECTOR_SIZE, 0);
    devices
        .elf_blk_device
        .read_blocks(0, elf_buffer.as_mut())
        .unwrap();

    sbi::shutdown();
}
