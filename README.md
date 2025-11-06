学习rust最小系统，学习地址：https://os.phil-opp.com/zh-CN/

# 独立式可执行程序

## no_std属性
#![no_std] //禁用标准库

## panic 处理函数
`panic_handler`属性定义一个函数，它会在一个` panic `发生时被调用

```rust
#![no_std] //禁用标准库

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

## eh_personality 语言项
` eh_personality `语言项标记的函数，将被用于实现栈展开`（stack unwinding）`
在使用标准库的情况下，当` panic `发生时，` Rust `将使用栈展开，来运行在栈上所有活跃的变量的析构函数`（destructor）`,这确保了所有使用的内存都被释放，允许调用程序的父进程`（parent thread）`捕获` panic `，处理并继续运行。
但是，栈展开是一个复杂的过程，如` Linux `的` libunwind `或` Windows `的结构化异常处理`（structured exception handling, SEH）`，通常需要依赖于操作系统的库；所以不在自己编写的操作系统中使用它

### 禁用栈展开
` Rust `提供了在` panic `时中止`（abort on panic）`的选项
```rust
//Cargo.toml
[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```
这些选项能将` dev `配置`（dev profile）`和` release `配置`（release profile）`的` panic `策略设为` abort `。
` dev `配置适用于` cargo build `，而` release `配置适用于` cargo build --release `

## start 语言项
通常认为，当运行一个程序时，首先被调用的是` main `函数。但是，大多数语言都拥有一个运行时系统`（runtime system）`，它通常为垃圾回收`（garbage collection）`或绿色线程`（software threads，或 green threads）`服务，这个运行时系统需要在 main 函数前启动，因为它需要让程序初始化

在一个典型的使用标准库的` Rust `程序中，程序运行是从一个名为` crt0 `的运行时库开始的。` crt0 `意为` C runtime zero `，它能建立一个适合运行` C `语言程序的环境，这包含了栈的创建和可执行程序参数的传入。在这之后，这个运行时库会调用` Rust `的运行时入口点，这个入口点被称作` start语言项（“start” language item）`。` Rust `只拥有一个极小的运行时，它被设计为拥有较少的功能，如爆栈检测和打印栈轨迹`（stack trace）`。这之后，这个运行时将会调用` main `函数

### 标准Rust程序的启动流程
在使用标准库的` Rust `程序中，启动流程是这样的：

- 1. 操作系统加载程序 → 跳转到` crt0 (C runtime zero) `
- 2. ` crt0 `执行基本初始化 → 调用` C `运行时入口点
- 3. ` C `运行时 → 调用` Rust `运行时入口点 ( start language item)
- 4. ` Rust `运行时 → 初始化` Rust `特定环境（如栈溢出检测等）
- 5. ` Rust `运行时 → 调用` main `函数
- 
Rust程序借用了C语言的启动基础设施 ，但并不是从C语言入口启动的，而是：
- 借用了` C `语言的启动环境初始化（通过` crt0 `）
- 然后进入` Rust `自己的运行时
- 最后才到` Rust `的` main `函数

### 重写入口点
我们的独立式可执行程序并不能访问` Rust `运行时或` crt0 `库，所以我们需要定义自己的入口点。只实现一个` start `语言项并不能帮助我们，因为这之后程序依然要求` crt0 `库。所以，需要直接重写整个` crt0 `库和它定义的入口点

` #![no_main] `属性:
- 禁用` main `函数的默认实现
- 这将告诉` Rust `编译器，我们不希望它生成` main `函数的默认实现，而是自己实现一个` _start `函数作为入口点

移除` main `函数的默认实现后，我们需要自己实现一个` _start `函数作为入口点
```rust
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    loop {}
}

/// 这个函数将在 panic 时被调用
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
```
使用` no_mangle `标记这个函数，来对它禁用名称重整`（name mangling）`，确保` Rust `编译器输出一个名为` _start `的函数；否则，编译器可能最终生成名为` _ZN3blog_os4_start7hb173fedf945531caE `的函数，无法让链接器正确辨别出这个函数的入口点

将函数标记为` extern "C" `，告诉编译器这个函数应当使用 C 语言的调用约定，而不是 Rust 语言的调用约定。函数名为` _start `，是因为大多数系统默认使用这个名字作为入口点名称

## 链接器错误
链接器`（linker）`是一个程序，它将生成的目标文件组合为一个可执行文件。不同的操作系统如` Windows、macOS、Linux `，规定了不同的可执行文件格式，因此也各有自己的链接器，抛出不同的错误；但这些错误的根本原因还是相同的：链接器的默认配置假定程序依赖于C语言的运行时环境，但我们的程序并不依赖于它

为了解决这个错误，我们需要告诉链接器，它不应该包含` C `语言运行环境。我们可以选择提供特定的链接器参数`（linker argument）`，也可以选择编译为裸机目标`（bare metal target）`

### 编译为裸机目标
在默认情况下，` Rust `尝试适配当前的系统环境，编译可执行程序。举个例子，如果你使用` x86_64 `平台的` Windows `系统，` Rust `将尝试编译一个扩展名为` .exe `的` Windows `可执行程序，并使用` x86_64 `指令集。这个环境又被称作为你的宿主系统`（“host” system）`

为了描述不同的环境，` Rust `使用一个称为目标三元组`（target triple）`的字符串。要查看当前系统的目标三元组，我们可以运行` rustc --version --verbose `
```bash
rustc 1.90.0 (1159e78c4 2025-09-14)
binary: rustc
commit-hash: 1159e78c4747b02ef996e55082b704c09b970588
commit-date: 2025-09-14
host: x86_64-pc-windows-msvc
release: 1.90.0
LLVM version: 20.1.8
```
上面这段输出来自一个 x86_64 平台下的` Windows `系统。我们能看到，host 字段的值为三元组` x86_64-pc-windows-msvc `，它包含了 CPU 架构` x86_64 `、供应商` pc `、操作系统` windows `和二进制接口` msvc `

在默认情况下，` Rust `编译器尝试为当前系统的三元组编译，并假定底层有一个类似于` Windows `或` Linux `的操作系统提供` C `语言运行环境——然而这将导致链接器错误。所以，为了避免这个错误，我们可以另选一个底层没有操作系统的运行环境

这样的运行环境被称作裸机环境，例如目标三元组` thumbv7em-none-eabihf `描述了一个` ARM `嵌入式系统`（embedded system）`。这个环境底层没有操作系统，这是由三元组中的` none `描述的。要为这个目标编译，我们需要使用` rustup `添加它：
```bash
rustup target add thumbv7em-none-eabihf
```
这行命令将为目标下载一个标准库和` core `库。这之后，我们就能为这个目标构建独立式可执行程序了：
```bash
cargo build --target thumbv7em-none-eabihf
```
我们传递了 `--target` 参数，来为裸机目标系统交叉编译`（cross compile）`我们的程序。我们的目标并不包括操作系统，所以链接器不会试着链接` C `语言运行环境，因此构建过程会成功完成，不会产生链接器错误。

我们将使用这个方法编写自己的操作系统内核。我们不会编译到` thumbv7em-none-eabihf `目标，而是使用描述` x86_64 `环境的自定义目标（custom target）

### 以本地操作系统为目标进行编译
```bash
# Linux
cargo rustc -- -C link-arg=-nostartfiles
# Windows
cargo rustc -- -C link-args="/ENTRY:_start /SUBSYSTEM:console"
# macOS
cargo rustc -- -C link-args="-e __start -static -nostartfiles"
```


# 最小内核

## 安装 Nightly Rust
` Rust `语言有三个发行频道`（release channel）`，分别是` stable、beta 和 nightly `
- ` stable `是默认的发行频道，它包含了最新的稳定版本的` Rust `
- ` beta `频道包含了最新的测试版本
- ` nightly `频道包含了最新的开发版本

` rustup `是` Rust `语言的工具链管理器，它可以帮助我们安装和管理不同的` Rust `版本和工具链
在当前目录使用` nightly `版本的` Rust `
```bash
rustup override add nightly
```

` Nightly `版本的编译器允许我们在源码的开头插入特性标签`（feature flag）`，来自由选择并使用大量实验性的功能。举个例子，要使用实验性的内联汇编`（asm!宏）`，我们可以在` main.rs `的顶部添加` #![feature(asm)] `。要注意的是，这样的实验性功能不稳定`（unstable）`，意味着未来的` Rust `版本可能会修改或移除这些功能，而不会有预先的警告过渡。因此我们只有在绝对必要的时候，才应该使用这些特性

### 目标配置清单
为了编写我们的目标系统，并且鉴于我们需要做一些特殊的配置（比如没有依赖的底层操作系统），已经支持的目标三元组都不能满足我们的要求。幸运的是，只需使用一个` JSON `文件，` Rust `便允许我们定义自己的目标系统；这个文件常被称作目标配置清单`（target specification）`。比如，一个描述` x86_64-unknown-linux-gnu `目标系统的配置清单大概长这样：
```json
{
    "llvm-target": "x86_64-unknown-linux-gnu",
    "data-layout": "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
    "arch": "x86_64",
    "target-endian": "little",
    "target-pointer-width": 64,
    "target-c-int-width": 32,
    "os": "linux",
    "executables": true,
    "linker-flavor": "gcc",
    "pre-link-args": ["-m64"],
    "morestack": false
}
```

一个配置清单中包含多个配置项`（field）`。大多数的配置项都是` LLVM `需求的，它们将配置为特定平台生成的代码。打个比方，` data-layout `配置项定义了不同的整数、浮点数、指针类型的长度；另外，还有一些` Rust `用作条件编译的配置项，如` target-pointer-width `。还有一些类型的配置项，定义了这个包该如何被编译，例如，` pre-link-args `配置项指定了应该向链接器`（linker）`传入的参数

内核编译到` x86_64 `架构：创建一个名为` x86_64-rust_os.json `的文件并写入：
```json
{
    "llvm-target": "x86_64-unknown-none",
    "data-layout": "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
    "arch": "x86_64",
    "target-endian": "little",
    "target-pointer-width": 64,
    "target-c-int-width": 32,
    "os": "none",
    "executables": true,
    "linker-flavor": "ld.lld",
    "linker": "rust-lld",
    "panic-strategy": "abort",
    "disable-redzone": true,
    "features": "-mmx,-sse,+soft-float",
    "rustc-abi": "x86-softfloat"
}
```

要在裸机`（bare metal）`上运行内核，修改了` llvm-target `的内容，并将` os `配置项的值改为` none `
与编译相关的配置项：

```json
"linker-flavor": "ld.lld",
"linker": "rust-lld",
```
不使用平台默认提供的链接器，因为它可能不支持` Linux `目标系统。为了链接内核，使用跨平台的` LLD链接器（LLD linker）`，它是和` Rust `一起打包发布的

```json
"panic-strategy": "abort",
```
这个配置项的意思是，我们的编译目标不支持` panic `时的栈展开`（stack unwinding）`，所以我们选择直接在` panic `时中止`（abort on panic）`。这和在` Cargo.toml `文件中添加` panic = "abort" `选项的作用是相同的，所以我们可以不在这里的配置清单中填写这一项

```json
"disable-redzone": true,
```
我们正在编写一个内核，所以我们迟早要处理中断。要安全地实现这一点，我们必须禁用一个与红区`（redzone）`有关的栈指针优化：因为此时，这个优化可能会导致栈被破坏

```json
"features": "-mmx,-sse,+soft-float",
```
` features `配置项被用来启用或禁用某个目标` CPU `特征`（CPU feature）`。通过在它们前面添加` - `号，我们将` mmx `和` sse `特征禁用；添加前缀` + `号，我们启用了` soft-float `特征

` mmx `和` sse `特征决定了是否支持单指令多数据流`（Single Instruction Multiple Data，SIMD）`相关指令，这些指令常常能显著地提高程序层面的性能。然而，在内核中使用庞大的` SIMD `寄存器，可能会造成较大的性能影响：因为每次程序中断时，内核不得不储存整个庞大的` SIMD `寄存器以备恢复——这意味着，对每个硬件中断或系统调用，完整的` SIMD `状态必须存到主存中。由于` SIMD `状态可能相当大`（512~1600 个字节）`，而中断可能时常发生，这些额外的存储与恢复操作可能显著地影响效率。为解决这个问题，我们对内核禁用` SIMD（但这不意味着禁用内核之上的应用程序的 SIMD 支持）`

禁用` SIMD `产生的一个问题是，` x86_64 `架构的浮点数指针运算默认依赖于` SIMD `寄存器。我们的解决方法是，启用` soft-float `特征，它将使用基于整数的软件功能，模拟浮点数指针运算

```json
"rustc-abi": "x86-softfloat"
```
由于我们希望使用` soft-float `特性，还需要告诉` Rust `编译器` rustc `使用对应的` ABI `。我们可以通过将` rustc-abi `字段设置为` x86-softfloat `来实现这一点

### 编译内核
无论你开发使用的是哪类操作系统，你都需要将入口点命名为` _start `

` core crate `以预编译库`（precompiled library）`的形式与` Rust `编译器一同发布，` core crate `只对支持的宿主系统有效，而对我们自定义的目标系统无效。如果我们想为其它系统编译代码，我们需要为这些系统重新编译整个` core crate `

#### build-std 选项
此时就到了` cargo `中` build-std `特性，该特性允许按照自己的需要重编译` core `等标准` crate `，而不需要使用` Rust `安装程序内置的预编译版本。但是该特性是全新的功能，到目前为止尚未完全完成，所以它被标记为` “unstable” `且仅被允许在` Nightly Rust `编译器环境下调用

要启用该特性，你需要创建一个` cargo `配置 文件，即` .cargo/config.toml `并写入以下语句：
```toml
[unstable]
build-std = ["core", "compiler_builtins"]
```

该配置会告知` cargo `需要重新编译` core `和` compiler_builtins `这两个` crate `，其中` compiler_builtins `是` core `的必要依赖。 另外重编译需要提供源码，我们可以使用` rustup component add rust-src `命令来下载它们。

通过把` JSON `文件名传入` --target `选项，我们现在可以开始编译我们的内核了：
```bash
cargo build --target x86_64-rust_os.json
```

#### 内存相关函数
` Rust `编译器假定所有内置函数`（built-in functions）`在所有系统内都是存在且可用的。事实上这个前提只对了一半， 绝大多数内置函数都可以被` compiler_builtins `提供，而这个` crate `刚刚已经被我们重编译过了，然而部分内存相关函数是需要操作系统相关的标准` C `库提供的。` compiler_builtins `事实上自带了所有相关函数的实现，只是在默认情况下，出于避免和标准C库发生冲突的考量被禁用掉了，此时我们需要将` build-std-features `配置项设置为` ["compiler-builtins-mem"] `来启用这个特性。如同` build-std `配置项一样，该特性可以使用` -Z `参数启用，也可以在` .cargo/config.toml `中使用` unstable `配置集启用。现在我们的配置文件中的相关部分是这样子的：
```toml
[unstable]
build-std-features = ["compiler-builtins-mem"]
build-std = ["core", "compiler_builtins"]
```

该参数为` compiler_builtins `启用了` mem `特性，在内部通过` #[unsafe(no_mangle)] `向链接器提供了` memcpy `等函数的实现

#### 设置默认编译目标
每次调用` cargo build `命令都需要传入` --target `参数，可以在` .cargo/config.toml `中加入` cargo `配置，使用` x86_64-rust_os.json `这个文件作为默认的` --target `参数：
```toml
[build]
target = "x86_64-rust_os.json"
```

### 向屏幕打印字符
最简单的方式是写入` VGA `字符缓冲区`（VGA text buffer）`：这是一段映射到` VGA `硬件的特殊内存片段，包含着显示在屏幕上的内容。通常情况下，它能够存储` 25 `行、` 80 `列共` 2000 `个字符单元`（character cell）`；每个字符单元能够显示一个` ASCII `字符，也能设置这个字符的前景色`（foreground color）`和背景色`（background color）`

####  VGA 字符缓冲区
- 1.定义：VGA 字符缓冲区是 x86 机器在“文本模式”下的显存映射区域，CPU 直接往这块内存写数据，屏幕就会显示对应字符
- 2.物理地址：常见为 0xB8000（段:偏移通常写作 B800:0000）
- 3.尺寸：默认 80 列 × 25 行，共 2000 个字符单元；每个单元占 2 字节，总计 4000 字节
- 4.内存布局（每单元 2 字节）：
 - 第 1 字节：ASCII 字符码
 - 第 2 字节：属性字节（颜色/闪烁）
 - 低 4 位：前景色
 - 高 3 位：背景色
 - 最高位：闪烁位（某些模式下可作为“高亮背景”位）
 - 颜色编码（常见 4 位）：0-黑, 1-蓝, 2-绿, 3-青, 4-红, 5-品红, 6-棕/黄, 7-亮灰, 8-暗灰, 9-亮蓝, …, 15-白
- 工作方式：这是“内存映射 I/O”。向 0xB8000 开始的内存写入对应字节，VGA 控制器会把它渲染到屏幕上；无需调用 BIOS 或操作系统 API。
典型用途：在自制内核早期阶段打印调试文本（例如在 Rust no_std/no_main 环境下），不用依赖串口或图形模式

```rust
static HELLO: &[u8] = b"Hello World!";

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {}
}
```

在这段代码中，我们预先定义了一个字节字符串`（byte string）`类型的静态变量`（static variable）`，名为` HELLO `。我们首先将整数` 0xb8000 `转换`（cast）`为一个裸指针`（raw pointer）`。这之后，我们迭代` HELLO `的每个字节，使用` enumerate `获得一个额外的序号变量` i `。在` for `语句的循环体中，我们使用` offset `偏移裸指针，解引用它，来将字符串的每个字节和对应的颜色字节` 0xb `代表淡青色——写入内存位置

要注意的是，所有的裸指针内存操作都被一个` unsafe `语句块`（unsafe block）`包围。这是因为，此时编译器不能确保我们创建的裸指针是有效的；一个裸指针可能指向任何一个你内存位置；直接解引用并写入它，也许会损坏正常的数据。使用` unsafe `语句块时，程序员其实在告诉编译器，自己保证语句块内的操作是有效的。事实上，` unsafe `语句块并不会关闭` Rust `的安全检查机制；它允许你多做的事情只有四件

使用` unsafe `语句块要求程序员有足够的自信，所以必须强调的一点是，肆意使用` unsafe `语句块并不是` Rust `编程的一贯方式。在缺乏足够经验的前提下，直接在` unsafe `语句块内操作裸指针，非常容易把事情弄得很糟糕；比如，在不注意的情况下，我们很可能会意外地操作缓冲区以外的内存

在这样的前提下，我们希望最小化` unsafe `语句块的使用。使用` Rust `语言，我们能够将不安全操作将包装为一个安全的抽象模块。举个例子，我们可以创建一个` VGA `缓冲区类型，把所有的不安全语句封装起来，来确保从类型外部操作时，无法写出不安全的代码：通过这种方式，我们只需要最少的` unsafe `语句块来确保我们不破坏内存安全`（memory safety）`

## 启动内核

### 创建引导映像
要将可执行程序转换为可引导的映像`（bootable disk image）`，我们需要把它和引导程序链接。这里，引导程序将负责初始化` CPU `并加载我们的内核。编写引导程序并不容易，所以我们不编写自己的引导程序，而是使用已有的` bootloader `包；无需依赖于` C `语言，这个包基于` Rust `代码和内联汇编，实现了一个五脏俱全的` BIOS `引导程序。为了用它启动我们的内核，我们需要将它添加为一个依赖项，在` Cargo.toml `中添加下面的代码：

```toml
[dependencies]
bootloader = "0.9"    #必须 0.9 ，高了环境不支持，缺少配置
```

添加引导程序为依赖项，并不足以创建一个可引导的磁盘映像；我们还需要内核编译完成之后，将内核和引导程序组合在一起。然而，截至目前，原生的` cargo `并不支持在编译完成后添加其它步骤

为了解决这个问题，我们建议使用` bootimage `工具——它将会在内核编译完毕后，将它和引导程序组合在一起，最终创建一个能够引导的磁盘映像。我们可以使用下面的命令来安装这款工具：
```bash
cargo install bootimage
```

为了运行` bootimage `以及编译引导程序，我们需要安装` rustup `模块` llvm-tools-preview `，安装这个工具我们可以使用：
```bash
rustup component add llvm-tools-preview
```

成功安装` bootimage `后，创建一个可引导的磁盘映像：
```bash
cargo bootimage
```

` bootimage `工具开始使用` cargo build `编译你的内核，所以它将增量编译我们修改后的源码。在这之后，它会编译内核的引导程序，这可能将花费一定的时间；但和所有其它依赖包相似的是，在首次编译后，产生的二进制文件将被缓存下来——这将显著地加速后续的编译过程。最终，` bootimage `将把内核和引导程序组合为一个可引导的磁盘映像

运行这行命令之后，我们应该能在` target/x86_64-rust_os/debug `目录内找到我们的映像文件` bootimage-rust_os.bin `。我们可以在虚拟机内启动它，也可以刻录到` U 盘 `上以便在真机上启动。（需要注意的是，因为文件格式不同，这里的` bin `文件并不是一个光驱映像，所以将它刻录到光盘不会起作用。）

在这行命令背后，` bootimage `工具执行了三个步骤：
- 编译我们的内核为一个` ELF（Executable and Linkable Format）`文件
- 编译引导程序为独立的可执行文件
- 将内核` ELF `文件按字节拼接`（append by bytes）`到引导程序的末端

当机器启动时，引导程序将会读取并解析拼接在其后的` ELF `文件。这之后，它将把程序片段映射到分页表`（page table）`中的虚拟地址`（virtual address）`，清零` BSS段（BSS segment）`，还将创建一个栈。最终它将读取入口点地址`（entry point address）`，我们程序中` _start `函数的位置——并跳转到这个位置

## 在 QEMU 中启动内核

### 安装 qemu
我们可以从` qemu `的官方网站`（https://www.qemu.org/）`下载最新版本的` qemu `
` windows `使用`msys2 的 pacman `安装` qemu `
```bash
pacman -Syyu   #更新软件包数据库

pacman -S mingw-w64-x86_64-qemu    #安装 qemu，

#安装完成后将 qemu 添加到环境变量中
```

### 启动内核
在` qemu `目录下，打开` PowerShell `窗口，执行下面的命令来启动内核：
```bash
.\qemu-system-x86_64.exe -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_os.bin
```

### 在真机上运行内核(未测试)
我们也可以使用` dd 工具 `把内核写入` U 盘 `，以便在真机上启动。可以输入下面的命令：
```bash
dd if=target/x86_64-blog_os/debug/bootimage-blog_os.bin of=/dev/sdX && sync
```

` sdX `是` U盘 `的设备名`（device name）`。请注意，在选择设备名的时候一定要极其小心，因为目标设备上已有的数据将全部被擦除

写入到` U 盘 `之后，你可以在真机上通过引导启动你的系统。视情况而定，你可能需要在` BIOS `中打开特殊的启动菜单，或者调整启动顺序。需要注意的是，` bootloader `包暂时不支持` UEFI `，所以我们并不能在` UEFI `机器上启动

### 使用 cargo run
要让在` QEMU `中运行内核更轻松，我们可以设置在` cargo `配置文件中设置` runner `配置项：
```toml
[target.'cfg(target_os = "none")']
runner = "bootimage runner"
```

` target.'cfg(target_os = "none")' `筛选了三元组中宿主系统设置为` "none" `的所有编译目标——这将包含我们的` x86_64-rust_os.json `目标。另外，` runner `的值规定了运行` cargo run `使用的命令；这个命令将在成功编译后执行，而且会传递可执行文件的路径为第一个参数

命令` bootimage runner `由` bootimage `包提供，参数格式经过特殊设计，可以用于` runner `命令。它将给定的可执行文件与项目的引导程序依赖项链接，然后在` QEMU `中启动它
