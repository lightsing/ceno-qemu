use alloc::vec::Vec;
use core::arch::asm;
use core::ptr::NonNull;
use fdt_rs::base::DevTree;
use fdt_rs::prelude::*;
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};
use virtio_drivers::transport::{DeviceType, Transport};
use crate::println;

extern "C" {
    static _stext: u8;
    static _stack_start: u8;
    static _sheap: u8;
    static _heap_size: u8;
}

fn get_dtb_address() -> usize {
    let dtb_address: usize;
    unsafe {
        asm!("mv {}, a1", out(reg) dtb_address);
    }
    dtb_address
}

#[inline(always)]
pub fn text_start() -> usize {
    unsafe { &_stext as *const u8 as usize }
}

#[inline(always)]
pub fn heap_start() -> usize {
    unsafe { &_sheap as *const u8 as usize }
}

#[inline(always)]
pub fn heap_size() -> usize {
    unsafe { &_heap_size as *const u8 as usize }
}

#[inline(always)]
pub fn stack_start() -> usize {
    unsafe { &_stack_start as *const u8 as usize }
}

#[derive(Debug)]
pub struct DeviceInfo {
    pub physical_memory: PhysicalMemory,
    pub virtio_blk_device: VirtioBlkDevice,
}

#[derive(Debug, Default)]
pub struct PhysicalMemory {
    pub base_address: u64,
    pub size: u64,
}

#[derive(Debug)]
pub struct VirtioBlkDevice {
    pub mmio_transport: MmioTransport,
    pub size: usize
}



/// Detect devices from device tree
///
/// # Safety
///
/// This function must be called immediately after booting.
pub unsafe fn detect_devices(dtb_address: *const u8) -> DeviceInfo {
    println!("----------------------------------DEVICE INFO-----------------------------------");

    println!(".text: {:#x}", text_start());
    println!("stack top: {:#x}", stack_start());
    println!("heap: {:#x} - {:#x}", heap_start(), heap_start() + heap_size());

    println!("DTB address: {:#x}", dtb_address as usize);

    let mut physical_memory = PhysicalMemory::default();
    let mut virtio_blk_device = None;

    let tree = unsafe { DevTree::from_raw_pointer(dtb_address) }.expect("failed to parse dev tree");
    for node in tree.nodes().iterator() {
        let node = node.expect("dev tree error");
        let name = node.name().expect("failed to get dev node name");

        if name.starts_with("memory") {
            for prop in node.props().iterator() {
                let prop = prop.expect("failed to get prop");
                let name = prop.name().expect("failed to get prop name");
                if !name.starts_with("reg") {
                    continue;
                }

                let base = prop.u64(0).unwrap();
                let size = prop.u64(1).unwrap();
                println!("phy memory: {:#x} - {:#x}", base, base + size);
                physical_memory.base_address = base;
                physical_memory.size = size;
            }
        } else if name.starts_with("virtio_mmio") {
            for prop in node.props().iterator() {
                let prop = prop.expect("failed to get prop");
                let name = prop.name().expect("failed to get prop name");
                if !name.starts_with("reg") {
                    continue;
                }
                let base_address = prop.u64(0).expect("failed to read virtio_mmio base_address") as usize;
                let size = prop.u64(1).expect("failed to read virtio_mmio size") as usize;
                let header = NonNull::new(base_address as *mut VirtIOHeader).expect("base_address null pointer");
                if let Ok(transport) = unsafe { MmioTransport::new(header) } {
                    if transport.device_type() != DeviceType::Block {
                        continue; // we don't need other devices
                    }
                    virtio_blk_device = Some(VirtioBlkDevice {
                        mmio_transport: transport,
                        size
                    });
                    println!("virtio_mmio: {:#x} - {:#x}", base_address, base_address + size);
                }
            }
        }
    }
    println!("--------------------------------------------------------------------------------");

    if physical_memory.base_address == 0 || physical_memory.size == 0 {
        panic!("failed to detect memory size");
    }
    if virtio_blk_device.is_none() {
        panic!("none of virtio_blk_device found");
    }

    DeviceInfo {
        physical_memory,
        virtio_blk_device: virtio_blk_device.unwrap(),
    }
}