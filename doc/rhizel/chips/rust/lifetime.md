# Lifetime

## 问：rust的引用生命周期（lifetime）的几条规则是什么？给我详细讲解一下

ChatGPT答：

Rust 引用生命周期（Lifetime）详解
Rust 的**生命周期（lifetime）**是 Rust 借用检查器（borrow checker）用来回答一个核心问题：

> **一个引用 `&T` 在什么时间范围内是有效的？**
> 理解 lifetime 最重要的不是死记 `'a`，而是理解**“引用之间的生命周期关系”**。
> 本文将生命周期规则整理为 6 条核心规则：

1. 生命周期描述的是“引用有效的范围”，不是变量本身活多久
2. 引用不能活得比被引用对象更久
3. 生命周期参数 `'a` 表示“多个引用之间的关系”
4. 函数返回引用时，返回值生命周期必须和某个输入引用关联
5. 生命周期省略规则（Lifetime Elision）有 3 条
6. `'static` 表示整个程序运行期间有效，但不等于“所有权是 static”

---

一、先建立一个正确的生命周期模型
先看最简单的代码：

```rust
fn main() {
    let x = 10;

    {
        let r = &x;
        println!("{}", r);
    }
}
```

可以粗略理解成：

```text
x 的生命周期
┌──────────────────────────────┐
│                              │
│     r 的生命周期             │
│     ┌──────────────┐         │
│     │              │         │
│     └──────────────┘         │
│                              │
└──────────────────────────────┘
```

这里：

```rust
let x = 10;
let r = &x;
```

`r` 是一个引用，它借用了 `x`。
所以必须满足：

```text
r 的有效期 ⊆ x 的有效期
```

也就是说：

> **引用不能在被引用对象已经死亡之后继续存在。**

## 这是 lifetime 最核心的规则。

二、第一条：引用不能活得比被引用对象久
看一个经典例子：

```rust
fn main() {
    let r;

    {
        let x = 10;
        r = &x;
    }

    println!("{}", r);
}
```

这个代码不能编译。
为什么？

```text
main
│
├── r ────────────────────────────────┐
│                                     │
│   ┌── x ──────────┐                 │
│   │               │                 │
│   └───────────────┘                 │
│                                     │
└─────────────────────────────────────┘
```

`x` 在内部 `{}` 结束时死亡。
但是 `r` 还想继续使用：

```rust
println!("{}", r);
```

于是：

```text
r 的生命周期
────────────────────────────>

x 的生命周期
────────────>
```

出现：

```text
r > x
```

这是不允许的。
Rust 要保证：

```text
引用生命周期 <= 被引用对象生命周期
```

---

三、生命周期不是“变量存在时间”这么简单
这里非常容易产生一个误解。
很多初学者会认为：

> lifetime 就是变量的生命周期。
> 其实不准确。
> 例如：

```rust
let x = String::from("hello");

let r = &x;

println!("{}", r);
```

这里至少存在两个不同的生命周期：

```text
x
┌──────────────────────────────┐
│                              │
│  String 对象                  │
│                              │
└──────────────────────────────┘

r
      ┌──────────────────┐
      │      &x          │
      └──────────────────┘
```

`x` 是拥有数据的变量。
`r` 是借用 `x` 的引用。
所以：

> **lifetime 主要描述的是引用能够合法使用的区间。**

## 而变量自身的生命周期和引用的生命周期不是一个概念。

四、第二条：生命周期参数 `'a` 是“关系”，不是具体时间
这是 Rust lifetime 最重要的地方。
比如：

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
```

第一次看到：

```text
'a
```

很多人会想：

> `'a` 是不是一个变量？
> 不是。
> 也不是：
> `'a` = 10 秒
> 更准确地说：
> **`'a` 是一个生命周期参数，用来表达多个引用之间的约束关系。**
> 这个函数可以理解成：

```text
x: &'a str
y: &'a str
      ↓
return: &'a str
```

意思是：

> 返回的引用至少必须满足 `'a` 所代表的生命周期关系。

---

五、为什么 `longest` 必须有生命周期参数？
看：

```rust
fn longest(x: &str, y: &str) -> &str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
```

编译器不知道：

```text
返回值到底来自 x 还是 y？
```

可能：

```rust
return x;
```

也可能：

```rust
return y;
```

所以返回引用的生命周期必须同时受到输入引用约束。
于是：

```rust
fn longest<'a>(
    x: &'a str,
    y: &'a str,
) -> &'a str
```

表达：

```text
x ──────┐
        ├──> 返回值
y ──────┘
```

## 返回值不能保证比 `x` 或 `y` 中真正需要的生命周期更长。

六、这里有一个非常重要的概念：最短生命周期
例如：

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
```

调用：

```rust
let s1 = String::from("long string");

{
    let s2 = String::from("short");

    let result = longest(&s1, &s2);

    println!("{}", result);
}
```

可以想象：

```text
s1
┌────────────────────────────────────┐
│                                    │
└────────────────────────────────────┘

s2
       ┌──────────────────┐
       │                  │
       └──────────────────┘

result
       ┌──────────────────┐
       │                  │
       └──────────────────┘
```

这里 `'a` 不能简单理解成：

```text
s1 的生命周期
```

也不能简单理解成：

```text
s2 的生命周期
```

而是编译器根据实际使用情况确定一个满足约束的生命周期范围。
通常可以把它理解成：

```text
'a ≈ 两个输入引用生命周期的交集/较短部分
```

也就是：

```text
'a
       ┌──────────────┐
       │              │
       └──────────────┘
```

## 因此 `result` 不能保证比 `s2` 活得更久。

七、第三条：函数返回引用时，必须知道它和谁有关
考虑：

```rust
fn first(s: &str) -> &str {
    &s[..1]
}
```

这里没有写 `'a`：

```rust
fn first(s: &str) -> &str
```

但是 Rust 可以编译。
为什么？
因为 Rust 有一个非常重要的机制：

> **生命周期省略（Lifetime Elision）**
> 实际上可以理解成：

```rust
fn first<'a>(s: &'a str) -> &'a str {
    &s[..1]
}
```

也就是说：

```text
输入 s
   │
   │ 'a
   ↓
返回值
   │
   │ 'a
```

所以：

> 返回引用来自 `s`，那么返回引用的生命周期跟 `s` 关联。

---

八、生命周期省略规则——最重要的 3 条规则
规则 1：每个引用参数都有自己独立的生命周期参数
比如：

```rust
fn foo(x: &i32, y: &i32)
```

编译器可以理解成：

```rust
fn foo<'a, 'b>(x: &'a i32, y: &'b i32)
```

注意：

```text
x → 'a
y → 'b
```

默认不是同一个生命周期。
这是非常重要的。

---

九、为什么不是默认 `'a`？
假设：

```rust
fn foo(x: &i32, y: &i32)
```

你不能认为它默认就是：

```rust
fn foo<'a>(x: &'a i32, y: &'a i32)
```

实际上规则是：

```rust
fn foo<'a, 'b>(x: &'a i32, y: &'b i32)
```

因为两个引用可能来自完全不同的地方：

```text
x
┌───────────────┐
│               │
└───────────────┘

y
       ┌───────────────┐
       │               │
       └───────────────┘
```

## 它们生命周期没有理由天然相同。

十、规则 2：如果只有一个输入生命周期，那么输出生命周期就是它
例如：

```rust
fn foo(x: &i32) -> &i32 {
    x
}
```

自动推导成：

```rust
fn foo<'a>(x: &'a i32) -> &'a i32 {
    x
}
```

这是最常见的一种。
再比如：

```rust
fn first_word(s: &str) -> &str {
    &s[..1]
}
```

相当于：

```rust
fn first_word<'a>(s: &'a str) -> &'a str {
    &s[..1]
}
```

---

十一、规则 3：方法里如果有 `&self` / `&mut self`，返回值生命周期默认跟 `self` 绑定
例如：

```rust
struct User {
    name: String,
}

impl User {
    fn name(&self) -> &str {
        &self.name
    }
}
```

实际上类似：

```rust
impl User {
    fn name<'a>(&'a self) -> &'a str {
        &self.name
    }
}
```

也就是：

```text
User
┌──────────────────────┐
│ name                 │
└──────────────────────┘
          ↑
          │
        &self
          │
          ↓
       返回 &str
```

返回的 `&str` 借用了 `self`。
所以：

```rust
let name = user.name();
```

## `name` 不能比 `user` 活得更久。

十二、三个 lifetime elision 规则总结
规则 含义
1 每个引用参数获得一个独立生命周期
2 如果只有一个输入生命周期，它赋给所有省略的输出生命周期
3 如果有 `&self` / `&mut self`，输出生命周期绑定到 `self`
例如：

```rust
fn foo(x: &i32) -> &i32
```

推导：

```rust
fn foo<'a>(x: &'a i32) -> &'a i32
```

再看：

```rust
fn foo(x: &i32, y: &i32) -> &i32
```

这里就有问题。
因为：

```text
x → 'a
y → 'b
```

但是返回值：

```text
? → ?
```

编译器不知道应该跟：

```text
'a
```

还是：

```text
'b
```

绑定。
所以这种情况下通常必须显式写 lifetime：

```rust
fn foo<'a>(x: &'a i32, y: &'a i32) -> &'a i32
```

---

十三、这就是为什么 `longest` 要写 `'a`
再看：

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str
```

其实你可以把它翻译成人话：

> 我要求 `x` 和 `y` 都至少在 `'a` 这个范围内有效，并且我返回一个在 `'a` 范围内有效的引用。
> 所以：

```text
          'a
    ┌──────────────┐
    │              │
x ──┤              │
y ──┤              │
    │   return     │
    └──────────────┘
```

---

十四、第四条：生命周期参数不会延长对象的生命
这是特别容易犯的错误。
比如：

```rust
fn get<'a>() -> &'a String {
    let s = String::from("hello");
    &s
}
```

你可能会想：

> 我写了 `'a`，那是不是让 `s` 活到 `'a`？
> 不是。
> 生命周期参数是约束，不是续命。
> `s` 还是会在函数结束时销毁：

```text
get()
│
├── s 创建
│
├── &s
│
└── s 销毁
```

所以不能返回：

```rust
&s
```

因为：

```text
返回引用
──────────────────────────>

s
────────────>
```

## 引用比数据活得久。

十五、生命周期不会改变所有权
这也是核心思想。
例如：

```rust
fn foo<'a>(x: &'a String) -> &'a String {
    x
}
```

这里没有发生：

```text
复制 String
```

也没有：

```text
转移所有权
```

只是：

```text
调用者拥有 String
       │
       │ borrow
       ↓
      &String
```

生命周期只是在告诉 Rust：

> 这个 borrow 最多能持续多久。

---

十六、第五条：生命周期参数可以出现在 struct 中
例如：

```rust
struct User<'a> {
    name: &'a str,
}
```

这里：

```text
User<'a>
```

表示：

> `User` 内部保存了一个引用，这个引用必须至少在 `'a` 生命周期内有效。
> 例如：

```rust
let name = String::from("Gavin");

let user = User {
    name: &name,
};
```

生命周期关系：

```text
name
┌───────────────────────────────┐
│                               │
└───────────────────────────────┘

user
       ┌───────────────────┐
       │ &name             │
       └───────────────────┘
```

## `user` 不能活得比 `name` 久。

十七、为什么 `struct` 需要写 `'a`？
因为：

```rust
struct User {
    name: &str,
}
```

这是不允许的。
Rust 必须知道：

```text
这个引用至少要活多久？
```

所以：

```rust
struct User<'a> {
    name: &'a str,
}
```

相当于：

```text
User<'a>
   │
   └── name: &'a str
```

## 生命周期成为 `User` 类型的一部分。

十八、第六条：`'static`
你经常会看到：

```rust
&'static str
```

比如：

```rust
let s: &'static str = "hello";
```

字符串字面量：

```rust
"hello"
```

存储在程序的静态区域，所以整个程序运行期间都有效。
可以粗略理解：

```text
程序启动
│
├─────────────────────────────┐
│                             │
│      &'static str           │
│                             │
└─────────────────────────────┘
                              │
                           程序结束
```

所以：

```rust
fn hello() -> &'static str {
    "hello"
}
```

## 可以。

十九、但 `'static` 不等于“永远不能销毁”
这是一个非常重要的细节。
比如：

```rust
let s: &'static str = "hello";
```

这里说的是：

> **这个引用指向的数据至少可以存在整个程序运行期间。**
> 而不是：
> 引用本身一定必须使用到程序结束。
> 例如：

```rust
fn foo() {
    let s: &'static str = "hello";

    println!("{}", s);
}
```

`foo()` 结束后：

```text
s 这个引用变量
───────────────┐
               ↓
             销毁
```

但是：

```text
"hello"
```

对应的静态数据仍然存在。
所以：

```text
引用变量的生命周期
```

和：

```text
引用指向的数据生命周期
```

## 依然是两个概念。

二十、`'static` 最容易出现的误区
很多人看到：

```rust
T: 'static
```

就理解成：

> T 必须存在整个程序生命周期。
> 这个理解不完全正确。
> 例如：

```rust
fn foo<T: 'static>(x: T) {}
```

`T: 'static` 更准确地表示：

> `T` 中不能包含一个比 `'static` 更短的借用。
> 例如：

```rust
String
```

满足：

```text
String: 'static
```

因为 `String` 自己拥有数据，没有内部借用。
所以：

```rust
fn foo<T: 'static>(x: T) {}

let s = String::from("hello");

foo(s);
```

是可以的。
并不是说 `s` 必须活到程序结束。

---

二十一、生命周期其实是一个“约束系统”
这是非常建议建立起来的思维方式。
不要把：

```text
'a
```

想象成：

> 一个具体的时间。
> 而应该想成：
> 一个约束变量。
> 例如：

```rust
fn foo<'a>(x: &'a str) -> &'a str
```

表达的是：

```text
lifetime(return) = lifetime(x)
```

或者更准确地说：

```text
return 必须满足 'a
x       必须满足 'a
```

而：

```rust
fn longest<'a>(
    x: &'a str,
    y: &'a str,
) -> &'a str
```

表达的是：

```text
x ──┐
    ├── 'a ──> return
y ──┘
```

所以生命周期本质上是在描述：

> **这些引用之间有什么合法的存活关系。**

---

二十二、一个非常经典的例子
看：

```rust
fn main() {
    let x = String::from("hello");

    let r;

    {
        let y = String::from("world");

        r = longest(&x, &y);

        println!("{}", r);
    }
}
```

这里可以工作。
为什么？
因为 `r` 实际使用发生在：

```rust
println!("{}", r);
```

而这个位置还在 `y` 的生命周期里面。
所以：

```text
x
┌──────────────────────────────────┐
│                                  │
└──────────────────────────────────┘

y
       ┌────────────────────┐
       │                    │
       └────────────────────┘

r
             ┌──────────────┐
             │              │
             └──────────────┘
```

## 生命周期只需要覆盖 `r` 实际使用的范围。

二十三、这说明 Rust 的 lifetime 不是简单的“作用域”
这点很重要。
例如：

```rust
let x = String::from("hello");

let r = &x;

println!("{}", r);
```

`r` 可能在语法作用域里存在更长时间，但 Rust 的借用检查会关注：

> **引用实际上在哪里被使用。**
> 现代 Rust 的借用分析比简单的：

```text
变量从 { 到 }
```

更精确。
因此不要把：

```text
scope
```

和：

```text
lifetime
```

完全画等号。
可以先这样理解：

```text
scope
= 变量名字可以使用的范围

lifetime
= 某个引用/借用关系有效的范围
```

## 两者经常重叠，但不是完全一样。

二十四、`&T` 和 `&mut T` 的生命周期
生命周期不仅存在于：

```text
&T
```

也存在于：

```text
&mut T
```

例如：

```rust
fn modify<'a>(x: &'a mut String) {
    x.push_str(" world");
}
```

这里：

```text
x
│
└── &'a mut String
```

`&mut` 的借用同样有生命周期。
而且 `&mut` 还有一个重要规则：

> **同一时间只能存在一个有效的可变借用。**
> 例如：

```rust
let mut x = String::from("hello");

let a = &mut x;
let b = &mut x;
```

不允许。
因为两个可变引用同时存在，会破坏 Rust 的别名规则。

---

二十五、为什么这个限制和 lifetime 有关系？
例如：

```rust
let mut x = 10;

let a = &mut x;

*a += 1;

let b = &mut x;
*b += 1;
```

这个却可以。
为什么？
因为 `a` 的借用结束后，`b` 才开始。
所以可以把：

```text
&mut
```

理解成：

> 在某个生命周期范围内，独占访问这个对象。

---

二十六、一个非常重要的口诀
你可以先记：

> **引用不能超过数据。**
> 也就是：

```text
&data 的生命周期
        ≤
data 的生命周期
```

再加：

> **返回引用必须说明“它借了谁”。**
> 例如：

```rust
fn foo<'a>(x: &'a str) -> &'a str
```

就是：

```text
返回值 ← x
```

而：

```rust
fn longest<'a>(
    x: &'a str,
    y: &'a str,
) -> &'a str
```

就是：

```text
返回值 ← x 或 y
```

## 所以必须建立生命周期关系。

二十七、把所有规则放在一起
可以形成这样一个完整模型：

```text
                    Rust Lifetime
                         │
          ┌──────────────┼──────────────┐
          │              │              │
       引用有效期       生命周期参数     生命周期省略
          │              │              │
          │              │              ├── 每个输入引用独立 lifetime
          │              │              ├── 单输入 → 输出跟随输入
          │              │              └── &self → 输出跟随 self
          │
          ├── 引用不能比数据活得久
          │
          ├── 返回引用必须和输入建立关系
          │
          ├── struct 保存引用需要 lifetime
          │
          └── 'static = 数据至少可活整个程序
```

---

二十八、你现在最应该掌握的 5 个例子
例 1：普通借用

```rust
fn main() {
    let x = String::from("hello");
    let r = &x;

    println!("{}", r);
}
```

核心：

```text
r ≤ x
```

---

例 2：悬垂引用

```rust
fn foo() -> &String {
    let s = String::from("hello");
    &s
}
```

错误：

```text
s 死了
↓
引用还想活
```

---

例 3：单输入生命周期省略

```rust
fn foo(x: &str) -> &str {
    x
}
```

等价：

```rust
fn foo<'a>(x: &'a str) -> &'a str {
    x
}
```

---

例 4：多个输入，需要显式关系

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
```

核心：

```text
x ──┐
    ├── 'a ──> return
y ──┘
```

---

例 5：struct 持有引用

```rust
struct User<'a> {
    name: &'a str,
}
```

核心：

```text
User
 │
 └── name: &'a str
             │
             ↓
           外部数据
```

所以：

```text
User 不能比 name 所指向的数据活得更久。
```

---

二十九、生命周期学习路线
如果正在系统学习 Rust，可以按这个顺序深入：

```text
① &T / &mut T
        ↓
② 借用规则
        ↓
③ 生命周期是什么
        ↓
④ 生命周期参数 'a
        ↓
⑤ Lifetime Elision 三规则
        ↓
⑥ struct<'a>
        ↓
⑦ 多个生命周期 'a / 'b
        ↓
⑧ &'a mut T
        ↓
⑨ 'static
        ↓
⑩ T: 'a / T: 'static
        ↓
⑪ 生命周期子类型 / outlives
        ↓
⑫ HRTB：for<'a>
        ↓
⑬ async / Future 中的 lifetime
```

其中比较值得彻底吃透的是：

```text
⑦ 多个生命周期
⑩ T: 'static / T: 'a
```

尤其是：

```rust
fn foo<'a, 'b>(x: &'a str, y: &'b str)
```

和：

```rust
fn foo<T: 'static>(x: T)
```

这两个东西一旦真正理解，后面 Rust 的 trait bound、泛型、异步、`Send + 'static`、闭包以及 proc macro 生成的泛型代码都会好理解很多。
