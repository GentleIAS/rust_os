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

## 禁用栈展开
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

## 标准Rust程序的启动流程
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

## 重写入口点
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

## 编译为裸机目标
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

## 以本地操作系统为目标进行编译
```bash
# Linux
cargo rustc -- -C link-arg=-nostartfiles
# Windows
cargo rustc -- -C link-args="/ENTRY:_start /SUBSYSTEM:console"
# macOS
cargo rustc -- -C link-args="-e __start -static -nostartfiles"
```


# 最小内核