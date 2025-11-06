#![no_std]    //不链接 Rust 标准库
#![no_main]   //禁用所有 Rust 层级的入口点

//这个函数将在 panic 时被调用
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

static HELLO: &[u8] = b"Hello World!";

#[unsafe(no_mangle)]   //不重整函数名
pub extern "C" fn _start() -> ! {
    //因为编译器会寻找一个名为 `_start` 的函数，所以这个函数就是入口点
    //默认命名为 `_start`

    let vga_buffer = 0xb8000 as *mut u8;    //将0xb8000转换为裸指针，VGA字符缓冲区的物理地址

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {}
}