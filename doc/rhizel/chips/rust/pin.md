# 《The Rust Programming Language》里的 17.5 A Closer Look at the Traits for Async

ChatGPT 答：

这篇《The Rust Programming Language》里的 **17.5 A Closer Look at the Traits for Async**，核心其实是在回答一个问题：

> **为什么 Rust 的 `Future::poll()` 要接收 `Pin<&mut Self>`，而不是普通的 `&mut Self`？`Pin` 和 `Unpin` 到底是干什么的？**

你现在正在学习 Rust async，这一章非常关键。尤其你之前已经在看 `Future`、`JoinHandle`、Tokio，那么理解这里之后，很多类似：

```text
the trait `Unpin` is not implemented
consider using `Box::pin`
consider using the `pin!` macro
```

的编译错误就会突然变得很好理解。

我按照**“先理解现象 → 再理解原理 → 最后看代码”**来讲。

---

# 1. 先给你一个最重要的结论

先记住这张图：

```text
async fn
   │
   ▼
Future
   │
   ▼
Future::poll()
   │
   │ 需要
   ▼
Pin<&mut Self>
   │
   │ 防止 Future 在内存中移动
   ▼
因为某些 Future 可能是 self-referential
   │
   ▼
所以需要 Pin
   │
   └── 如果类型实现 Unpin
          │
          └── 就表示：这个类型其实不怕移动
```

所以：

> **Pin = “这个对象不能随便移动了”**

> **Unpin = “这个对象其实可以随便移动，不需要 Pin 带来的限制”**

这就是整篇文章最核心的思想。官方文档也明确指出，绝大多数 Rust 类型都是 `Unpin`；只有真正需要地址稳定性的类型才需要依赖 pinning 保证。[Rust Documentation](https://doc.rust-lang.org/std/pin/index.html?utm_source=chatgpt.com)

---

# 2. 先回顾一下 Future

你之前应该已经看到过：

```rust
trait Future {
    type Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output>;
}
```

这里最值得注意的是：

```rust
self: Pin<&mut Self>
```

如果你刚开始学 Rust，第一反应通常是：

> 为什么不是：

```rust
fn poll(
    &mut self,
    cx: &mut Context<'_>,
) -> Poll<Self::Output>
```

为什么非要：

```rust
Pin<&mut Self>
```

？

这就是这一篇文章真正想解释的东西。

---

# 3. Future 本质上是什么？

先看一个简单的：

```rust
async fn foo() {
    println!("hello");
}
```

你调用：

```rust
let f = foo();
```

**`f` 并没有马上执行。**

它实际上是一个 Future。

可以粗略理解成：

```text
foo()
 │
 ▼
Future
 │
 ▼
poll()
 │
 ├── Pending
 │
 ├── Pending
 │
 └── Ready(())
```

runtime，比如 Tokio，会不断 poll：

```text
       ┌─────────────┐
       │    Future   │
       └──────┬──────┘
              │
            poll()
              │
       ┌──────┴──────┐
       │             │
   Pending        Ready
       │             │
       │             ▼
       │            完成
       ▼
  以后再 poll
```

官方文档也是这样描述的：runtime 会 poll future；如果得到 `Pending`，之后再次检查；得到 `Ready` 就表示完成。[Rust Documentation](https://doc.rust-lang.org/book/ch17-05-traits-for-async.html?highlight=combinator&utm_source=chatgpt.com)

---

# 4. 问题来了：async Future 是一个“状态机”

这是理解 `Pin` 最重要的一步。

比如：

```rust
async fn foo() {
    let x = String::from("hello");

    some_async_operation().await;

    println!("{}", x);
}
```

表面看起来很简单。

但 Rust 编译器不会把它简单编译成：

```text
调用 some_async_operation()
等待
继续执行
```

而是会把 async 函数转换成一个类似**状态机**的东西。

粗略想象：

```rust
enum FooFuture {
    State1 {
        x: String,
    },

    State2 {
        x: String,
        // 这里可能还有一些状态
    },

    Done,
}
```

大概类似：

```text
             Future
                │
        ┌───────┴───────┐
        │               │
      State1          State2
        │               │
        │ await         │
        ▼               ▼
   正在等待        await 完成
                        │
                        ▼
                      Done
```

---

# 5. 更关键的问题：Future 里面可能存在“自己指向自己”的引用

这是 `Pin` 存在的根本原因。

假设某种 Future 内部有：

```text
Future
┌──────────────────────┐
│ data                 │
│                      │
│ pointer ─────────────┼──────┐
│                      │      │
│ ...                  │      │
└──────────────────────┘      │
                              │
                              ▼
                         Future 内部
```

也就是：

```text
Future 自己
   │
   └──────► Future 自己内部的数据
```

这种结构叫：

> **self-referential type（自引用类型）**

问题来了。

假设 Future 原来在：

```text
地址 1000
```

内部有：

```text
pointer = 1008
```

那么：

```text
1000 ┌──────────────────┐
     │ Future           │
     │                  │
     │ pointer ─────────┼──► 1008
     │                  │
1008 │ data             │
     └──────────────────┘
```

现在如果把整个 Future 移动：

```rust
let future2 = future1;
```

假设它从：

```text
1000
```

移动到了：

```text
2000
```

但是内部 pointer 如果没有同步修改：

```text
2000 ┌──────────────────┐
     │ Future           │
     │                  │
     │ pointer ─────────┼──► 1008  ❌
     │                  │
     │ data             │
     └──────────────────┘
```

pointer 就指向旧地址了。

这就是问题。

---

# 6. 为什么普通 Rust 代码很少遇到这个问题？

因为 Rust 通常禁止这种危险情况。

例如：

```rust
let x = String::from("hello");

let y = x;
```

Rust 可以安全地把 `x` 移动到 `y`。

因为：

```text
String
├── pointer
├── length
└── capacity
```

它没有：

```text
指向自己内部的引用
```

所以：

```text
移动 String
    ↓
没问题
```

但是：

```text
自引用结构
    ↓
移动
    ↓
内部引用可能失效
```

就不一样了。

---

# 7. async Future 为什么特别容易遇到这个问题？

因为：

```rust
async fn
```

会产生一个状态机。

尤其是：

```rust
async fn foo() {
    let x = ...;

    some_future().await;

    use(x);
}
```

`await` 前后的变量需要被保存下来。

所以编译器生成的 Future 可能类似：

```text
Future
┌─────────────────────┐
│ x                   │
│                     │
│ state               │
│                     │
│ future state        │
└─────────────────────┘
```

在复杂情况下，这个状态机可能存在地址敏感的数据。

所以 Rust 的 `Future` trait 直接规定：

```rust
fn poll(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
)
```

而不是：

```rust
fn poll(
    &mut self,
    cx: &mut Context<'_>,
)
```

官方 `Pin` 文档也明确说明，`async fn` 产生的 Future 需要这种保证，因此 `Future::poll` 使用 `Pin<&mut Self>`。[Rust Documentation](https://doc.rust-lang.org/stable/std/pin/struct.Pin.html?utm_source=chatgpt.com)

---

# 8. Pin 到底是什么？

先看：

```rust
Pin<&mut T>
```

不要把它理解成：

> “一种特殊的引用”

更准确地说：

> **Pin 是一个对“移动”进行限制的包装器。**

例如：

```rust
let mut x = String::from("hello");

let p = Pin::new(&mut x);
```

概念上：

```text
p
│
│
▼
┌──────────────┐
│ String       │
│ "hello"      │
└──────────────┘
```

`Pin` 的目的就是告诉 Rust：

> 我需要保证这里面的 `T` 不会被随便移动。

注意：

### `Pin` 本身可以移动

这是非常容易搞错的一点。

例如：

```text
Pin<Box<Future>>
```

移动：

```text
Pin
  ↓
Box
  ↓
Future
```

可以：

```text
之前：

p1 ──► Box ──► Future @ 1000


之后：

p2 ──► Box ──► Future @ 1000
```

**Box 可以移动。**

但：

```text
Future @ 1000
```

这个 Future 本体不能被移动到：

```text
Future @ 2000
```

这正是我们想要的。

官方文档也特别强调：`Pin<Box<T>>` 锁定的是 `Box` 指向的 `T`，而不是 `Box` 指针本身。[Rust Documentation](https://doc.rust-lang.org/book/ch17-05-traits-for-async.html?highlight=combinator&utm_source=chatgpt.com)

---

# 9. 那 Unpin 又是什么？

这是第二个最容易混淆的概念。

```rust
trait Unpin {}
```

`Unpin` 是一个 **marker trait**。

它本身基本没有方法。

它表达的是：

> **这个类型不需要 pinning 保证，可以安全移动。**

例如：

```rust
String
Vec<T>
i32
bool
```

这些通常都是：

```rust
T: Unpin
```

也就是说：

```rust
Pin<&mut String>
```

实际上并不会给 `String` 带来特别大的限制。

因为：

```text
String
   ↓
Unpin
   ↓
不怕移动
```

官方文档把 `Unpin` 描述成一种“取消 Pin 限制”的机制：如果 `T: Unpin`，那么 `Pin<Box<T>>` 在行为上基本就和普通 `Box<T>` 一样。[Rust Documentation](https://doc.rust-lang.org/std/pin/index.html?utm_source=chatgpt.com)

---

# 10. 所以 Pin + Unpin 的关系其实非常简单

你可以记成：

```text
Pin
│
│ “不要移动这个东西”
│
▼
T

但是：

T: Unpin
│
▼
“放心吧，我不怕移动”
│
▼
Pin 的限制基本不起作用
```

所以：

```text
T: Unpin
```

意味着：

> 即使你把 T 放进 Pin，我也允许你把它当作普通可移动对象使用。

---

# 11. 为什么 Future 经常是 !Unpin？

这里是重点。

你可能会看到：

```text
dyn Future<Output = ()> cannot be unpinned
```

这个错误。

意思不是：

> “Future 坏了”

而是：

> **这个 Future 类型不能保证自己随便移动是安全的。**

所以 runtime/API 需要：

```rust
Pin<...>
```

来保证它的位置稳定。

特别是：

```rust
async {
    ...
}
```

生成的匿名 Future 类型，经常是：

```text
!Unpin
```

也就是：

```text
async block
    ↓
匿名 Future
    ↓
可能 !Unpin
    ↓
需要 Pin
```

---

# 12. 为什么 `.await` 的时候我们通常感觉不到 Pin？

这是非常好的问题。

比如你写：

```rust
foo().await;
```

你根本没有写：

```rust
Pin
```

为什么？

因为：

> **编译器和 async 语法帮你处理了很多 pinning 细节。**

所以日常代码：

```rust
async fn main() {
    foo().await;
    bar().await;
}
```

通常完全不需要你手动：

```rust
Pin::new(...)
```

官方文档也指出，直接 `.await` 一个 Future 时，Future 会被隐式 pin，因此你平时不会到处看到 `pin!`。[Rust Documentation](https://doc.rust-lang.org/book/ch17-05-traits-for-async.html?highlight=combinator&utm_source=chatgpt.com)

---

# 13. 真正容易遇到 Pin 的地方：动态 Future

这也是这篇文章为什么举：

```rust
Vec<Box<dyn Future<Output = ()>>>
```

这个例子。

假设：

```rust
let futures: Vec<Box<dyn Future<Output = ()>>> = vec![
    Box::new(future1),
    Box::new(future2),
    Box::new(future3),
];
```

看起来非常合理。

但是：

```text
dyn Future
```

本身可能：

```text
!Unpin
```

于是你可能得到：

```text
dyn Future<Output = ()> cannot be unpinned
```

---

# 14. 为什么 Box 还不够？

你可能会想：

> Future 都已经放进 Box 了，不就不会移动了吗？

这里一定要区分：

```text
Box
```

和：

```text
Box 里面的对象
```

假设：

```text
Box
 │
 ▼
┌───────────────┐
│    Future     │
└───────────────┘
```

移动 Box：

```text
Box1 ──► Future
```

变成：

```text
Box2 ──► Future
```

Future 仍然可以被移动。

所以：

```rust
Box<T>
```

不等于：

```rust
Pin<Box<T>>
```

真正表达：

> “这个 Future 的内存位置固定”

的是：

```rust
Pin<Box<T>>
```

---

# 15. `Box::pin()` 是干什么的？

例如：

```rust
let future = Box::pin(async {
    println!("hello");
});
```

得到：

```rust
Pin<Box<SomeFuture>>
```

概念上：

```text
Pin
 │
 ▼
Box
 │
 ▼
Future
```

其中：

```text
Box 可以移动
```

但是：

```text
Future 本身不能被移动
```

因此：

```rust
Box::pin(async { ... })
```

非常常见。

官方文档也把 `Box::pin` 作为需要把 pinned value 带到当前作用域之外时的典型方式。[Rust Documentation](https://doc.rust-lang.org/book/ch17-05-traits-for-async.html?highlight=combinator&utm_source=chatgpt.com)

---

# 16. `pin!` 又是什么？

还有：

```rust
use std::pin::pin;

let future = pin!(async {
    println!("hello");
});
```

它和：

```rust
Box::pin(...)
```

有一个重要区别。

### `pin!`

通常是：

```text
本地 pin
```

而：

```rust
Box::pin()
```

是：

```text
heap allocation + pin
```

官方文档明确指出，`pin!` 不会创建新的 heap allocation，而是创建 `Pin<&mut T>`。[Rust Documentation](https://doc.rust-lang.org/core/pin/macro.pin.html?utm_source=chatgpt.com)

所以：

```rust
let f = pin!(async {
    ...
});
```

大致得到：

```rust
Pin<&mut Future>
```

而：

```rust
let f = Box::pin(async {
    ...
});
```

得到：

```rust
Pin<Box<Future>>
```

---

# 17. 两者怎么选择？

简单记：

```text
pin!
 │
 └── 我就在当前作用域里用
     不需要 heap allocation

Box::pin
 │
 └── 我需要一个拥有所有权的
     heap allocated Future
```

例如：

```rust
let f = pin!(async {
    foo().await;
});

f.await;
```

这种很适合 `pin!`。

而：

```rust
fn create_future() -> Pin<Box<dyn Future<Output = ()>>> {
    Box::pin(async {
        foo().await;
    })
}
```

这种场景就非常适合：

```rust
Box::pin
```

---

# 18. 回头看文章里的 `join_all`

文章中真正让人头疼的代码类似：

```rust
let futures: Vec<Box<dyn Future<Output = ()>>> = vec![
    Box::new(future1),
    Box::new(future2),
    Box::new(future3),
];

join_all(futures).await;
```

问题是：

```text
Box<dyn Future>
```

无法满足所需要的 `Unpin` 条件。

于是改成：

```rust
let future1 = pin!(async {
    ...
});

let future2 = pin!(async {
    ...
});

let future3 = pin!(async {
    ...
});

let futures: Vec<Pin<&mut dyn Future<Output = ()>>> =
    vec![future1, future2, future3];
```

这时候：

```text
future1
   │
   ▼
Pin<&mut Future>

future2
   │
   ▼
Pin<&mut Future>

future3
   │
   ▼
Pin<&mut Future>
```

于是 `join_all` 就知道：

> 这些 Future 已经被 pin 住了，可以安全地 poll。

这正是文章 Listing 17-24 的核心。[Rust Documentation](https://doc.rust-lang.org/book/ch17-05-traits-for-async.html?highlight=combinator&utm_source=chatgpt.com)

---

# 19. 你可以把整个过程想象成“地址合同”

我觉得这是理解 Pin 最好的一种方式。

普通：

```rust
let x = SomeFuture;
```

相当于：

> “x，你以后爱搬家就搬家。”

而：

```rust
let x = pin!(SomeFuture);
```

相当于：

> “从现在开始，Future 本体的地址稳定下来。”

于是：

```text
普通 Future：

1000 ── Future
          │
          ▼
        可以移动
          │
          ▼
2000 ── Future


Pinned Future：

1000 ── Future
          │
          │
          X 不能随便移动
```

---

# 20. `Unpin` 可以理解成“这个类型签了豁免协议”

假设：

```rust
String: Unpin
```

意思就是：

> “我虽然被放进 Pin 里面，但你不用担心我移动。我没有那种依赖自身地址的内部状态。”

所以：

```rust
Pin<&mut String>
```

和：

```rust
&mut String
```

实际使用体验差别很小。

但是某个：

```rust
SomeSelfReferentialFuture: !Unpin
```

就意味着：

> “别移动我，我内部可能依赖自己的地址。”

因此：

```text
Pin
+
!Unpin
=
真正需要保证不能移动
```

---

# 21. 为什么 `Unpin` 是“默认”的？

这一点非常重要。

不要反过来理解：

```text
所有类型默认都不能移动
```

实际上恰恰相反。

绝大多数 Rust 类型：

```rust
T: Unpin
```

例如：

```rust
i32
String
Vec<T>
struct Foo { ... }
```

通常都是 `Unpin`。

只有涉及地址敏感状态的类型才需要：

```text
!Unpin
```

官方 `std::pin` 文档也强调，绝大多数 Rust 类型没有 address-sensitive state，因此实现 `Unpin`。[Rust Documentation](https://doc.rust-lang.org/std/pin/index.html?utm_source=chatgpt.com)

所以你日常写：

```rust
struct User {
    name: String,
}
```

基本完全不用考虑：

```text
Pin
Unpin
```

---

# 22. 这也解释了你以后经常看到的类型

你以后写 Tokio/Rust async，很容易看到：

```rust
Pin<Box<dyn Future<Output = ()> + Send>>
```

刚开始非常吓人。

其实拆开：

```text
Pin
 │
 └── Box
      │
      └── dyn Future
```

意思就是：

> 一个放在 heap 上、地址稳定、可以动态派发的 Future。

再加：

```text
+ Send
```

表示：

> 可以在线程之间安全转移。

所以：

```rust
Pin<Box<dyn Future<Output = ()> + Send>>
```

可以粗略翻译成人话：

> **一个放在堆上、不能随便移动、可以在线程间传递的动态 Future。**

这在 async Rust 里非常常见。

---

# 23. 最后，文章后半部分的 Stream

这部分其实就比较容易了。

文章接着讲：

```rust
trait Stream {
    type Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>>;
}
```

你可以把：

```rust
Future
```

和：

```rust
Stream
```

对比：

### Future

只能产生一个最终结果：

```text
Future

Pending
Pending
Pending
Ready(value)
```

### Stream

可以产生很多个结果：

```text
Stream

Pending
Pending
Ready(Some(A))
Pending
Ready(Some(B))
Ready(Some(C))
Ready(None)
```

所以：

```rust
Future
```

相当于：

```text
异步版的一次性结果
```

而：

```rust
Stream
```

相当于：

```text
异步版 Iterator
```

可以理解成：

```text
Iterator
    │
    │ next()
    ▼
Option<Item>


Future
    │
    │ poll()
    ▼
Poll<Output>


Stream
    │
    │ poll_next()
    ▼
Poll<Option<Item>>
```

所以：

```rust
Poll<Option<T>>
```

其实非常有意义：

```text
Poll
│
├── Pending
│
└── Ready(...)
       │
       ├── Some(T)   → 还有数据
       │
       └── None      → Stream 结束
```

---

# 24. 把整篇文章浓缩成一张图

你可以把这篇文章记成：

```text
                         async fn
                            │
                            ▼
                         Future
                            │
                            ▼
                    ┌───────────────┐
                    │ Future::poll  │
                    └───────┬───────┘
                            │
                            │
                     Pin<&mut Self>
                            │
                            ▼
                    Future 不能随便移动
                            │
                            │
              ┌─────────────┴─────────────┐
              │                           │
          T: Unpin                    T: !Unpin
              │                           │
              ▼                           ▼
        可以安全移动                 必须稳定地址
              │                           │
              ▼                           ▼
       Pin 限制基本无影响                 Pin
                                          │
                                  ┌───────┴───────┐
                                  │               │
                               pin!()         Box::pin()
                                  │               │
                            Pin<&mut T>      Pin<Box<T>>
```

---

# 25. 对你现在学 Rust，我建议这样记

你暂时**不要深入研究 `Pin` 的所有 unsafe 细节**。

现在只需要牢牢记住下面 6 条：

### ① Future 是状态机

```rust
async fn foo() { ... }
```

实际上会生成一个 Future 状态机。

---

### ② runtime 通过 `poll()` 驱动 Future

```rust
poll()
```

返回：

```rust
Poll::Pending
```

或者：

```rust
Poll::Ready(value)
```

---

### ③ `Future::poll()` 为什么需要 Pin？

因为某些 Future 可能存在**地址敏感 / 自引用状态**。

所以：

```rust
Pin<&mut Self>
```

保证 Future 在 poll 的过程中不会被非法移动。

---

### ④ `Pin` 的核心含义

> **保证被 pin 的对象不会随便移动。**

不是：

> Pin 自己不能移动。

而是：

```text
Pin<Box<T>>
       │
       ▼
      T
       ↑
       │
   T 的地址稳定
```

---

### ⑤ `Unpin` 的核心含义

> **这个类型不需要地址稳定，可以安全移动。**

所以：

```rust
T: Unpin
```

基本可以理解成：

```text
“我不怕移动。”
```

---

### ⑥ 日常 async 代码通常不用手动管 Pin

```rust
foo().await;
```

绝大多数时候不用你考虑。

只有当你开始做：

```text
动态 Future
Future 集合
trait object
手动 poll
自定义 Future
async runtime
底层 async library
```

的时候，`Pin` / `Unpin` 才会大量出现。

---

## 和你现在的 Rust 学习路线连接起来

你前面已经在学习：

```text
async/await
    ↓
Future
    ↓
poll
    ↓
Pin / Unpin   ← 现在这篇
    ↓
Stream
    ↓
Task / Executor
    ↓
Tokio runtime
```

所以我建议你下一步**不要马上去钻 `unsafe Pin` 的实现细节**。

更值得理解的是一个非常关键的问题：

> **`async fn` 到底是怎么被编译器转换成 Future 状态机的？以及 `.await` 到底是怎么变成 `poll()` + `Pending/Ready` 的？**

一旦这个过程真正理解了，`Pin` 为什么存在会变得非常直观。
