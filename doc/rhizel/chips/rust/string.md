# String 与 &str

## rust的&str与String的内存结构有什么区别?

ChatGPT 答：

可以把它们先记成一句话：

**String = 拥有一块堆内存的“胖指针”**
**&str = 借用一段字符串的“胖指针”**

两者在内存结构上都不是单纯的指针，都可以理解成 **指针 + 长度**，但 `String` 还额外保存 **capacity**。

### 1. String 的结构

可以粗略理解为：

```rs
struct String {
    ptr: *mut u8,
    len: usize,
    capacity: usize,
}
```

例如：

```rs
let s = String::from("hello");
```

大致是：

```txt
栈 Stack
┌────────────────────┐
│ ptr ───────────────┼─────────────┐
│ len = 5            │             │
│ capacity = 5       │             ▼
└────────────────────┘        堆 Heap
                              ┌─────────────┐
                              │ h e l l o   │
                              └─────────────┘
```

所以在 64 位系统上，`String` 本身通常占：

```txt
8 bytes ptr
8 bytes len
8 bytes capacity
----------------
24 bytes
```

可以验证：

```rs
println!("{}", std::mem::size_of::<String>());
```

通常输出：

```rs
24
```

### 2. `&str` 的结构

&str 是一个 **DST（Dynamically Sized Type，动态大小类型）** 的引用。

因此：

```rs
let s = String::from("hello");
let x: &str = &s;
```

x 可以粗略理解为：

```txt
┌──────────────────┐
│ ptr ─────────────┼─────────────┐
│ len = 5          │             │
└──────────────────┘             ▼
                            ┌─────────────┐
                            │ h e l l o   │
                            └─────────────┘
```

也就是说：

```txt
&str
┌──────────────┐
│ pointer      │
│ length       │
└──────────────┘
```

它没有 `capacity`。

因此 64 位系统上：

```rs
println!("{}", std::mem::size_of::<&str>());
```

通常是：

```txt
16
```

所以：

```txt
String = 24 bytes
&str = 16 bytes
```

### 3. 为什么 `&str` 需要 length？

这是理解 Rust 字符串非常重要的一点。

普通引用：

```rs
&i32
```

只需要一个地址：

```txt
&i32
    │
    ▼
┌───────┐
│ 123   │
└───────┘
```

但是 `str` 本身大小是不确定的。

例如：

```txt
"hello"
"hello world"
"abcdef..."
```

编译器不能仅仅通过地址知道字符串有多长。

所以 `&str` 必须同时携带：

```txt
地址 + 长度
```

这就是为什么 `&str` 是 **fat pointer（胖指针）**。

### 4. String 和 &str 的关系

例如：

```rs
let s = String::from("hello");
let x = &s[..];
```

内存可以理解成：

```txt
             Stack
        ┌───────────────┐
s ────> │ ptr ──────────┼────────────┐
        │ len = 5       │            │
        │ capacity = 5  │            │
        └───────────────┘            │
                                     ▼
                                  Heap
                              ┌───────────┐
                              │ h e l l o │
                              └───────────┘

        ┌───────────────┐
x ────> │ ptr ──────────┼────────────┘
        │ len = 5       │
        └───────────────┘
```

注意：

**`s` 和 `x` 指向的是同一块字符串数据。**

没有发生字符串复制。

### 5. 最大的区别其实是“所有权”

这比内存大小更加重要。

```rs
let s = String::from("hello");
let x: &str = &s;
```

这里：

```txt
String
  │
  │ owns
  ▼
Heap data

&str
  │
  │ borrows
  ▼
Heap data
```

`String`：

**拥有这块内存**

`&str`：

**只是借用这块内存**

所以：

```rs
fn foo() -> String {
    String::from("hello")
}
```

可以。

但是：

```rs
fn foo() -> &str {
    let s = String::from("hello");
    &s
}
```

不可以。

因为：

```txt
foo()
 │
 ├── s ──> Heap "hello"
 │
 └── &s ──> Heap "hello"
       ↑
       │
       想返回
       │
foo() 结束
 │
 └── s 被销毁
      │
      └── Heap 被释放
```

如果允许返回，就会产生悬垂引用。

### 6. 这也解释了你上一条代码的问题

你之前：

```rs
let token = opt.token.unwrap_or(&self.random_token().unwrap());
```

假设：

```rs
opt.token: Option<&str>
```

那么 `unwrap_or()` 需要：

```rs
&str
```

但是：

```rs
self.random_token().unwrap()
```

产生的是：

```rs
String
```

你又取：

```rs
&String
```

最终得到：

```rs
&str
```

虽然类型上可以转换，但问题是这个 `String` 是一个**临时拥有者**：

```txt
temporary String
      │
      ▼
    Heap
      │
      ▲
      │
     &str
```

表达式结束后 `String` 就销毁了，`&str` 就没有合法的数据来源了。

所以更合理的是：

```rs
let token = match opt.token {
    Some(token) => token.to_owned(),
    None => self.random_token().unwrap(),
};
```

最终：

```rs
token: String
```

### 7. 可以用一个表记住

|                            | String         | &str           |
| -------------------------- | -------------- | -------------- |
| 所有权                     | 拥有           | 借用           |
| 数据通常在哪里             | Heap           | 指向已有数据   |
| 自己管理内存               | ✅             | ❌             |
| ptr                        | ✅             | ✅             |
| len                        | ✅             | ✅             |
| capacity                   | ✅             | ❌             |
| 64 位通常大小              | 24 bytes       | 16 bytes       |
| 可以修改字符串             | ✅             | 通常 `&str` ❌ |
| 可以返回函数内部创建的数据 | ✅             | ❌             |
| 是否会复制字符串           | 本身不代表复制 | 本身不复制     |

最重要的是：

```txt
String
   = ptr + len + capacity
   = 我拥有这块字符串

&str
   = ptr + len
   = 我借用这块字符串
```

另外还有一个容易混淆的点：**`&str` 不一定指向 `String` 的堆内存**。它也可以指向字符串字面量：

```rs
let s: &'static str = "hello";
```

这里 `"hello"` 通常来自**程序的只读静态数据区**，而不是 `String` 的 heap。
