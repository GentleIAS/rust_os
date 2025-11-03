学习rust最小系统，学习地址：https://os.phil-opp.com/zh-CN/

# 独立式可执行程序

## no_std属性
#![no_std] //禁用标准库

## panic 处理函数
`panic_handler`属性定义一个函数，它会在一个 panic 发生时被调用

```rust
#![no_std] //禁用标准库

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```