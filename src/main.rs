#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::arch::asm;
use virtio_drivers::device::blk::{VirtIOBlk, SECTOR_SIZE};

mod sbi;
mod info;
mod memory;
mod virtio;

#[riscv_rt::entry]
fn main() -> ! {
    let dtb_address: *const u8;
    unsafe {
        asm!("mv {}, a1", out(reg) dtb_address);
    }
    println!("-------------------------------CENO QEMU RISCV32--------------------------------");
    unsafe { memory::init_heap(); }

    let info = unsafe { info::detect_devices(dtb_address) };

    let mut blk = VirtIOBlk::<virtio::HalImpl, _>::new(info.virtio_blk_device.mmio_transport).unwrap();
    let sectors = blk.capacity();
    let mut buffer = Vec::with_capacity(sectors as usize * SECTOR_SIZE);
    buffer.resize(sectors as usize * SECTOR_SIZE, 0);
    blk.read_blocks(0, buffer.as_mut()).unwrap();
    println!("Read: {:?}", String::from_utf8(buffer).unwrap());

    sbi::shutdown();
}