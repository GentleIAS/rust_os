///VGA相关的类型定义

///颜色枚举
#[allow(dead_code)]   //对 Color 枚举类型禁用未使用的变量发出警告
//生成（derive）了 Copy、Clone、Debug、PartialEq 和 Eq 这几个 trait：
//这让我们的类型遵循复制语义（copy semantics），也让它可以被比较、被调试和打印。
#[core::prelude::rust_2024::derive(core::fmt::Debug, core::clone::Clone, core::marker::Copy, core::cmp::PartialEq, core::cmp::Eq)]
#[repr(u8)]   //注记标注的枚举类型，都会以一个 u8 的形式存储
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

///颜色代码结构体
#[core::prelude::rust_2024::derive(core::fmt::Debug, core::clone::Clone, core::marker::Copy, core::cmp::PartialEq, core::cmp::Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

///屏幕字符结构体
#[core::prelude::rust_2024::derive(core::fmt::Debug, core::clone::Clone, core::marker::Copy, core::cmp::PartialEq, core::cmp::Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

///屏幕字符写入器结构体
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

///屏幕字符写入器实现
impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col] = ScreenChar {
                    ascii_character: byte,
                    color_code,
                };
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {/* TODO */}
}

///屏幕字符串写入器实现
impl Writer {
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                //可以是能打印的 ASCII 码字节，也可以是换行符
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                //不包含在上述范围之内的字节
                _ => self.write_byte(0xfe),
            }

        }
    }
}

///打印"Hello World!"
pub fn print_something() {
    let mut writer = Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    };

    writer.write_byte(b'H');
    writer.write_string("ello ");
    writer.write_string("Wörld!");
}