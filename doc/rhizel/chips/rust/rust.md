# Rust

## 资料

- [rust language](https://doc.rust-lang.org/book/title-page.html)
- [rust nightly reference](https://doc.rust-lang.org/nightly/reference/introduction.html)
- [rust language（中文）](https://rustwiki.org/zh-CN/book/title-page.html)
- [cargo](https://doc.rust-lang.org/cargo/index.html)
- [宏小册](https://zjp-cn.github.io/tlborm/introduction.html)
- [Rust学习笔记](https://zjp-cn.github.io/rust-note/index.html)
- [rust相关资料](https://www.yuque.com/zhoujiping/programming/rust-materials)
- [rust course](https://beatai.org/rust-course/about-book)
- [Macro 宏编程](https://github.com/sunface/rust-course/blob/3df9ab18ef4b2e70700b04ec8507b273547c858f/src/advance/macro.md)
- [https://blog.turbo.fish/](https://blog.turbo.fish/)
- [https://serde.rs/)](https://serde.rs/)

## FQA （与chatgpt的对话）

### 问：rust语言的设计原理和理念是什么？其类型系统设计架构是怎样的？

#### 答：

如果从“为什么 Rust 要这样设计”而不是单纯学习语法的角度去理解，Rust 会突然变得非常清晰。

我会把它概括成一句话：

**Rust 的核心目标，是在不依赖 GC 的情况下，把内存安全、线程安全和零成本抽象尽可能地交给编译器证明。**

而它的类型系统，就是实现这个目标的核心基础设施。

##### 一、Rust 到底想解决什么问题？

Rust 并不是简单地想做一个“比 C++ 更现代的语言”。

它实际上在解决一个非常困难的三角矛盾：

```txt
                 高性能
                   ▲
                  / \
                 /   \
                /     \
               /       \
        内存安全 ──────── 无 GC

```

传统语言往往只能同时很好地满足其中两个。

例如：

C/C++

```txt
高性能 ✅
无 GC  ✅
内存安全 ❌
```

程序员拥有非常大的控制权，但也容易出现：

- use-after-free
- double free
- dangling pointer
- buffer overflow
- data race
- iterator invalidation

Java / Go / C#

```txt
内存安全 ✅
开发效率 ✅
高性能 ⚠️
```

通常通过 GC 来解决内存生命周期问题。

Rust

Rust 希望做到：

```txt
高性能    ✅
无 GC     ✅
内存安全  ✅
线程安全  ✅
```

因此 Rust 的设计重点不是：

“如何让程序员写起来更方便？”

而更像：

“如何让编译器能够证明这个程序是安全的，同时又不牺牲底层控制能力？”

##### 二、Rust 最核心的设计理念：让错误尽可能在编译期消失

Rust 非常强调：

**Compile-time guarantees**

例如：

```rs
let s = String::from("hello");

drop(s);

println!("{s}");
```

C/C++ 中类似问题可能在运行时爆炸。

Rust：

```txt
编译阶段
    ↓
发现 s 已经 moved
    ↓
拒绝编译
```

又例如：

```rs
let mut x = 10;

let a = &x;
let b = &mut x;
```

Rust 会拒绝：

```txt
immutable borrow
+
mutable borrow
+
同时存在
```

原因不是 Rust 认为这种代码“不优雅”。

而是：

这种别名关系可能破坏内存安全，因此类型系统直接禁止。

##### 三、Rust 的设计哲学其实可以拆成 6 个关键词

###### 1. Ownership

所有权：

```rs
let s1 = String::from("hello");
let s2 = s1;
```

之后：

```rs
println!("{s1}");
```

不允许。

因为：

```txt
s1
 │
 ▼
String
```

所有权移动到了：

```txt
s2
 │
 ▼
String
```

Rust 不需要 GC，也不需要引用计数，就知道什么时候释放：

```txt
s2 离开作用域
      ↓
drop(String)
```

##### 四、Borrowing：把“使用权”和“所有权”分离

例如：

```rs
fn print(s: &String) {
    println!("{s}");
}

fn main() {
    let s = String::from("hello");

    print(&s);

    println!("{s}");
}
```

这里：

```txt
String
  │
  └── owner: s

       │
       └── borrow → print()
```

print() 只有使用权，没有所有权。

因此函数执行完：

```txt
borrow 结束
     ↓
s 仍然存在
```

##### 五、Borrow Checker 真正解决的是什么？

很多人把 Rust 的 borrow checker 理解成：

“Rust 不允许同时有多个引用。”

这其实不准确。

Rust 允许：

```rs
let s = String::from("hello");

let a = &s;
let b = &s;
let c = &s;
```

多个不可变引用完全可以。

规则实际上更接近：

```txt
多个 &T
       ↓
      OK

一个 &mut T
       ↓
不能同时存在其他访问
```

也就是经典规则：

```txt
读可以多个
写必须独占
```

可以抽象成：

```txt
& T
│
├── Read
├── Read
└── Read

&mut T
│
└── Exclusive Write
```

这直接建立了：

Rust 的内存安全模型

##### 六、Rust 类型系统最重要的架构之一：Algebraic Data Types

Rust 的类型系统不是简单的：

```txt
int
float
char
class
```

它大量建立在 Algebraic Data Types（ADT） 上。

最重要的两个组成部分：

```txt
Product Type
Sum Type
```

##### 七、Product Type：struct / tuple

例如：

```rs
struct User {
    name: String,
    age: u32,
}
```

可以理解成：

```txt
User
 =
String × u32
```

也就是：

一个 User 同时拥有 name 和 age。

所以叫：

Product Type

因为：

```txt
字段数量组合
```

例如：

```rs
struct Point {
    x: f64,
    y: f64,
}
```

数学上可以理解成：

```txt
Point = f64 × f64
```

##### 八、Sum Type：enum

例如：

```rs
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

它表示：

```txt
Result<T, E>
=
T + E
```

也就是说：

```txt
要么是 T
要么是 E
```

这就是：

Sum Type

Rust 的 enum 实际上非常强大。

例如：

```rs
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
}
```

实际上是：

```txt
Message
   │
   ├── Quit
   │
   ├── Move
   │     ├── x
   │     └── y
   │
   └── Write
         └── String
```

这和传统 C/C++ 的 enum 有非常大的区别。

##### 九、Option<T> 是 Rust 类型系统思想的典型体现

Rust 没有传统意义上的：

```txt
null
```

而是：

```rs
Option<T>
```

本质：

```rs
enum Option<T> {
    Some(T),
    None,
}
```

也就是：

```txt
Option<T>
=
T + None
```

所以：

```rs
let value: Option<String>;
```

意味着：

```txt
value
 │
 ├── Some(String)
 │
 └── None
```

而不是：

```txt
String
 │
 └── 可能偷偷是 null
```

这就是 Rust 一个非常重要的理念：

把程序状态编码进类型。

##### 十、这也是 Rust 类型系统最漂亮的地方之一

比如函数：

```rs
fn find_user(id: u64) -> Option<User>
```

函数签名已经告诉你：

```txt
可能找到 User
也可能找不到
```

调用者必须处理：

```rs
match find_user(10) {
    Some(user) => {}
    None => {}
}
```

而不是：

```txt
user = findUser(10);

user.name // 可能突然 null pointer
```

Rust 把：

运行时可能发生的状态

变成：

编译期必须处理的类型。

##### 十一、Result<T, E> 也是一样

```rs
fn read_file() -> Result<String, Error>
```

类型告诉你：

```txt
成功 → String
失败 → Error
```

因此：

```rs
match read_file() {
    Ok(content) => {}
    Err(error) => {}
}
```

错误不是一个隐藏的控制流，而是函数类型的一部分。

这也是为什么 Rust 特别强调：

Make invalid states unrepresentable

即：

让非法状态无法被类型系统表示。

这是理解 Rust 的一个非常重要的思想。

##### 十二、Trait：Rust 没有传统 OOP 的继承体系

Rust 类型系统另外一个核心组件是：

```txt
trait
```

例如：

```rs
trait Animal {
    fn speak(&self);
}
```

然后：

```rs
struct Dog;

impl Animal for Dog {
    fn speak(&self) {
        println!("wang");
    }
}
```

这里不是：

```txt
Dog extends Animal
```

而是：

```txt
Dog
 │
 └── implements Animal
```

也就是：

行为和数据类型解耦。

##### 十三、Trait 本质上类似“能力约束”

例如：

```rs
fn print<T: Display>(value: T) {
    println!("{value}");
}
```

这不是说：

```txt
T 必须继承某个父类。
```

而是：

```txt
只要 T 实现了 Display，它就可以进入这个函数。
```

因此：

```txt
T
│
├── i32
├── String
├── MyStruct
└── ...
        ↓
    Display
```

##### 十四、Generics + Trait Bounds 构成 Rust 的抽象机制

例如：

```rs
fn max<T: Ord>(a: T, b: T) -> T {
    if a > b { a } else { b }
}
```

这里：

```txt
T
│
└── Ord
```

意味着：

T 必须具备比较能力。

这和 C++ template 有一点相似，但 Rust 的 trait 系统把这种约束显式地纳入类型系统。

##### 十五、Rust 类型系统还有一个非常关键的概念：生命周期

例如：

```rs
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() > b.len() {
        a
    } else {
        b
    }
}
```

这里：

```txt
'a
```

不是运行时的数据。

它是：

编译期描述引用有效范围的类型系统信息。

可以粗略理解成：

```txt
'a
│
├── a 至少活这么久
└── b 至少活这么久
```

因此返回值：

```rs
&'a str
```

保证：

返回的引用不会比 'a 活得更久。

##### 十六、所以 Rust 类型系统其实在描述很多东西

普通语言的类型可能主要描述：

```txt
这个东西是什么？
```

Rust 的类型系统还在描述：

```txt
它是什么？
+
它拥有谁？
+
谁可以访问？
+
能不能修改？
+
什么时候有效？
+
是否可能为空？
+
是否可能失败？
+
是否可以跨线程？
+
支持哪些行为？
```

因此可以把 Rust 类型系统粗略画成：

```txt
                    Rust Type System
                           │
          ┌────────────────┼────────────────┐
          │                │                │
       Data             Behavior         Lifetime
          │                │                │
   ┌──────┴──────┐      Trait          'a
   │             │
 struct         enum
   │             │
Product        Sum
 Type           Type
   │             │
   └──────┬──────┘
          │
       Generic
          │
      Trait Bounds
          │
     ┌────┴────┐
     │         │
Ownership   Borrowing
     │         │
     └────┬────┘
          │
     Borrow Checker
          │
          ▼
   Compile-time Safety
```

##### 十七、再往上还有 Send / Sync

这是 Rust 类型系统特别厉害的地方。

例如：

```rs
std::thread::spawn(...)
```

跨线程传递的数据必须满足相应的线程安全约束。

核心 trait：

```rs
Send
Sync
```

粗略理解：

```txt
Send
 ↓
这个类型的所有权可以安全地转移到另一个线程

Sync
 ↓
这个类型可以安全地被多个线程通过引用访问
```

于是 Rust 能把：

```txt
线程安全问题
```

也部分转化为：

```txt
类型检查问题
```

这就是所谓：

**Fearless Concurrency**

##### 十八、Rust 的 unsafe 反而说明了它的设计边界

Rust 并不是说：

“世界上不存在不安全的代码。”

而是把系统划分成：

```txt
Safe Rust
     │
     ▼
编译器能够证明安全
```

和：

```txt
Unsafe Rust
     │
     ▼
程序员承担证明责任
```

例如：

```rs
unsafe {
    // raw pointer
}
```

unsafe 并不是：

“这里一定不安全。”

而更准确地说：

“编译器无法仅凭 Rust 的安全类型系统证明这里安全，因此把证明责任交给程序员。”

这其实是非常重要的语言设计哲学。

##### 十九、为什么 Rust 能做到这些？

因为 Rust 把很多传统运行时问题提前到了编译器。

可以对比：

```txt
传统 C/C++

源代码
  ↓
编译器
  ↓
机器码
  ↓
运行
  ↓
可能发现：
use-after-free
double free
data race
null pointer
```

Rust：

```txt
源代码
  ↓
Parser
  ↓
Type Checker
  ↓
Ownership Analysis
  ↓
Borrow Checker
  ↓
Trait Resolution
  ↓
Lifetime Checking
  ↓
MIR
  ↓
LLVM
  ↓
机器码
```

大量错误在：

运行之前

就已经被消灭。

##### 二十、Rust 的“零成本抽象”也是核心理念

Rust 并不是为了安全而接受巨大性能损失。

它希望：

**高级抽象在编译之后尽可能消失。**

例如：

```rs
let values = vec![1, 2, 3];

for value in &values {
    println!("{value}");
}
```

这里用了：

- iterator
- borrowing
- generics
- trait
- abstraction

但是经过优化后，很多抽象层并不会产生对应的运行时成本。

所以 Rust 的一个经典理念就是：

**What you don't use, you don't pay for.**

##### 二十一、为什么 Rust 学起来会感觉“很难”？

因为 Rust 实际上把很多原来由程序员运行时承担的问题，搬到了编译期类型系统。

C/C++：

```txt
程序员负责：
    ↓
内存生命周期
    ↓
指针有效性
    ↓
线程同步
    ↓
错误处理
```

Rust：

```txt
程序员表达意图
       ↓
类型系统
       ↓
Borrow Checker
       ↓
编译器验证
       ↓
机器码
```

所以 Rust 的学习曲线实际上来自：

你必须学会用类型系统表达资源、生命周期、所有权和状态。

##### 二十二、如果你正在学习 Rust，我建议建立这张“心智模型”

不要把 Rust 学成：

```txt
let 怎么写
match 怎么写
trait 怎么写
lifetime 怎么写
macro 怎么写
```

而应该建立：

```txt
                Rust
                  │
        ┌─────────┴─────────┐
        │                   │
      Value               Type
        │                   │
    Ownership           Trait
        │                   │
    Move/Borrow        Generic
        │                   │
    Lifetime           Enum/Struct
        │                   │
        └─────────┬─────────┘
                  │
             Type System
                  │
        ┌─────────┴─────────┐
        │                   │
 Memory Safety        Thread Safety
        │                   │
        └─────────┬─────────┘
                  │
           Compile-time
              Guarantee
                  │
                  ▼
          Zero-cost abstraction
```

##### 二十三、而你最近学习的过程宏，实际上正好处于这个体系的更上层

你最近在学习：

```rs
#[derive(Builder)]
struct Command {
    executable: String,
    args: Vec<String>,
}
```

然后用：

```txt
syn
quote
proc_macro
```

分析 AST，再生成 Rust 代码。

这实际上是：

```txt
Rust 源代码
     ↓
     syn
     ↓
AST
     ↓
你的宏逻辑
     ↓
quote!
     ↓
Rust TokenStream
     ↓
Rust 编译器
     ↓
类型系统
     ↓
Borrow Checker
```

所以你现在遇到的：

```txt
LitStr
Ident
TokenStream
MetaNameValue
quote!
syn
```

并不是一些孤立的“宏 API”。

它们实际上是在接触 Rust 编译器体系的：

**语法层 → 类型层 → 语义层**

这也是为什么学 proc-macro 会让你对 Rust 的整体设计理解得更深。

如果继续往这个方向深入，下一步非常值得理解的是 “Rust 编译器从源码 → AST → HIR → 类型检查 → MIR → Borrow Check → LLVM”这一整条编译链。理解这条链之后，ownership / lifetime / trait / async / proc-macro 会开始串成一个整体。
