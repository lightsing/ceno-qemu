use core::panic::PanicInfo;
use sbi_rt::Physical;

#[derive(Copy, Clone)]
pub struct DebugConsole;

impl core::fmt::Write for DebugConsole {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut slice = s.as_bytes();
        while !slice.is_empty() {
            match sbi_rt::console_write(Physical::new(slice.len(),slice.as_ptr() as usize, 0)).into_result() {
                Ok(n) => {
                    slice = &slice[n..];
                }
                Err(_) => {
                    return Err(core::fmt::Error);
                }
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let _ = write!(crate::sbi::DebugConsole, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($($arg:tt)*) => {
        {
            $crate::print!("{}\n", format_args!($($arg)*));
        }
    };
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("-------------------------------------PANIC--------------------------------------");
    println!("{}", info);
    println!("--------------------------------SYSTEM ABORTED----------------------------------");


    sbi_rt::system_reset(sbi_rt::Shutdown, sbi_rt::SystemFailure).unwrap();
    unreachable!()
}

#[inline(always)]
pub fn shutdown() -> ! {
    println!("--------------------------------SYSTEM SHUTDOWN---------------------------------");
    sbi_rt::system_reset(sbi_rt::Shutdown, sbi_rt::NoReason).unwrap();
    unreachable!()
}