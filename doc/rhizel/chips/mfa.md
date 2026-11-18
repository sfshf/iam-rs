# 问：给我详细讲解一下 Multi-Factor Authentication（多因素认证），并给我讲一讲如何用rust实现

ChatGPT 答：

Multi-Factor Authentication（MFA，多因素认证）详解与 Rust 实现
Multi-Factor Authentication（简称 MFA，多因素认证）是现代身份认证系统中非常重要的一项安全机制。

如果你正在使用 Rust 开发 Web 服务、RESTful API、用户认证系统，或者类似 OAuth2、JWT 的身份验证模块，那么 MFA 是非常值得掌握的技术。

这次我从基本原理、安全模型、常见实现方案、认证流程几个方面展开，然后使用 Rust 从零实现一个基于 TOTP 的 MFA 系统。

一、什么是 Multi-Factor Authentication？
1.1 基本概念
传统的身份认证通常采用单因素认证（Single-Factor Authentication，SFA）。

例如，一个用户登录网站：

1. 输入用户名。
2. 输入密码。
3. 服务端验证密码。
4. 验证成功，允许登录。
   这种方式只依赖一个认证因素：Something You Know（你知道的东西）。

问题在于，如果用户密码泄露，攻击者就可能直接登录用户账户。

MFA 的核心思想是：
不再仅仅依赖一个认证因素，而是要求用户提供两个或两个以上来自不同类别的认证因素。

通常分为三个类别：

1. Something You Know（你知道的）

知识因素（Knowledge Factor）

- 密码（Password）
- PIN 码
- 安全问题答案

2. Something You Have（你拥有的）

持有因素（Possession Factor）

- 手机
- 硬件安全密钥（YubiKey）
- 身份认证器（Authenticator）
- 智能卡

3. Something You Are（你本身具有的）

生物特征因素（Inherence Factor）

- 指纹
- 人脸识别
- 虹膜识别
- 声纹

例如：
认证方式 认证因素
用户名 + 密码 知识因素
密码 + 手机短信验证码 知识 + 持有（有条件地成立）
密码 + Google Authenticator 知识 + 持有
密码 + 指纹 知识 + 生物特征
密码 + 硬件安全密钥 知识 + 持有

这里有一个非常重要的概念：

MFA 不等于输入两次密码。

两个认证因素必须来自不同类别。例如密码加 PIN 码，本质上仍然属于知识因素，不能算作真正的双因素认证。
1.2 MFA、2FA、2SV 有什么区别？
这三个概念经常被混淆。
名称 含义
SFA Single-Factor Authentication，单因素认证
2FA Two-Factor Authentication，双因素认证
MFA Multi-Factor Authentication，多因素认证
2SV Two-Step Verification，两步验证

它们的区别：

- 2FA：必须使用两个不同类别的认证因素。
- MFA：使用两个或更多不同类别的认证因素。
- 2SV：只表示存在两个验证步骤，不保证这两个步骤属于不同因素类别。
  例如：

密码 + 邮箱验证码属于两步验证，但邮箱验证码是否构成独立的持有因素，需要结合具体实现和威胁模型判断。

二、MFA 常见的实现方式
实际开发中，MFA 有多种实现方案。
2.1 SMS OTP（短信验证码）

tescobank.com

这是最容易理解的一种方式。

认证流程：

1. 用户输入用户名和密码。
2. 服务端验证密码。
3. 服务端生成一个随机验证码，例如 834921。
4. 服务端通过短信发送给用户。
5. 用户输入验证码。
6. 服务端验证验证码。
7. 验证成功，完成登录。
   优点：

- 用户使用门槛低。
- 不需要安装额外的认证应用。
- 实现相对简单。
  缺点：
- 可能遭受 SIM Swap（SIM 卡交换攻击）。
- 短信可能被拦截。
- 依赖短信服务商。
- 有一定的运营成本。
  因此，短信验证码虽然常见，但不属于特别强的 MFA 方案。
  2.2 Email OTP（邮箱验证码）

help.smartling.com

流程与短信验证码类似，只是通过电子邮件发送验证码。

例如：
Your verification code is:

739251

This code expires in 5 minutes.

优点：

- 实现简单。
- 不需要短信服务。
- 用户通常已经拥有邮箱账户。
  缺点：
- 邮箱账户本身可能已经被攻击者控制。
- 邮件可能延迟。
- 邮箱密码和当前登录账户密码可能相同。
  需要注意：邮箱 OTP 是否能满足某个系统的 MFA 合规要求，取决于采用的认证标准和威胁模型，不能简单认为它一定是独立的第二因素。
  2.3 TOTP（基于时间的一次性密码）
  本文 Rust 实现的重点

TOTP 全称是：

Time-Based One-Time Password

它是一种基于时间生成动态验证码的算法。

常见的认证应用包括：

- Google Authenticator
- Microsoft Authenticator
- Aegis Authenticator
- 2FAS
  这些应用都可以生成 TOTP 验证码。

TOTP 有几个重要特征：

1. 用户和服务端共享一个密钥（Secret）。
2. 双方使用当前时间计算验证码。
3. 验证码通常每隔 30 秒变化一次。
4. 不需要通过网络向认证应用发送验证码。
5. 服务端可以独立验证用户提交的验证码。
   这是非常适合自己实现的一种 MFA 方案。
   2.4 HOTP（基于计数器的一次性密码）
   HOTP 全称：

HMAC-Based One-Time Password

它与 TOTP 非常相似，但是有一个重要区别：

- HOTP 使用计数器（Counter）。
- TOTP 使用时间（Time）。
  例如：
  HOTP:

Counter = 1 -> 123456
Counter = 2 -> 823471
Counter = 3 -> 293817

每次生成验证码，计数器都会递增。

HOTP 需要服务端维护计数器状态，因此在现代互联网应用中，TOTP 通常更加常见。
2.5 WebAuthn / Passkeys

kjell.com

WebAuthn 是一种基于公钥密码学的 Web 身份认证标准。

常见实现：

- Passkeys
- YubiKey
- Windows Hello
- Touch ID
  它与 TOTP 的原理完全不同：

TOTP 使用共享密钥生成动态验证码，而 WebAuthn 使用非对称密码学：

- 客户端保存私钥。
- 服务端保存公钥。
- 服务端发送 Challenge。
- 客户端使用私钥签名 Challenge。
- 服务端使用公钥验证签名。
  WebAuthn 具有抗钓鱼能力，是现代身份认证系统非常值得优先考虑的方案。

三、深入理解 TOTP 的工作原理
在 Rust 中实现 TOTP 之前，需要先理解它的算法。

TOTP 并不是简单地生成一个随机数，然后让客户端和服务端分别保存这个随机数。

它实际上是一种基于共享密钥和时间窗口的确定性密码算法。
3.1 TOTP 的核心原理
假设：

- 服务端和用户手机共享一个 Secret。
- 时间窗口为 30 秒。
- 验证码长度为 6 位。
- 使用 HMAC-SHA1 算法。
  例如：
  用户手机与服务端共享同一个 Secret
  SHARED SECRET
  JBSWY3DPEHPK3PXP

手机端

Authenticator
482913
动态生成

服务端

Rust Backend
482913
独立计算

相同的 Secret + 相同的时间窗口 = 相同的验证码

双方不需要互相通信，也不需要实时同步验证码。

只要满足以下条件，就能计算出相同的结果：

1. Secret 相同。
2. 当前时间相同（或者处于允许的时间窗口内）。
3. 算法参数相同。
   3.2 TOTP 的数学公式
   TOTP 的公式：
   \[
   TOTP = HOTP(K,T)
   \]

其中：

- \(K\)：共享密钥（Secret）。
- \(T\)：时间计数器（Time Counter）。
- HOTP：基于 HMAC 的一次性密码算法。
  时间计数器的计算公式：
  \[
  T = \left\lfloor\frac{CurrentTime-T_0}{X}\right\rfloor
  \]

其中：
参数 含义 默认值
CurrentTime 当前 Unix 时间戳 当前时间
T0 起始时间 0
X 时间窗口 30 秒
T 时间计数器 动态计算

例如，假设当前 Unix 时间戳是 1794650400：
CurrentTime = 1794650400
T0 = 0
X = 30

T = 1794650400 / 30
= 59821680

那么当前时间窗口对应的 HOTP Counter 就是：
59821680

接下来使用共享密钥和 Counter 计算 HMAC。
第一步：计算 HMAC
\[
HMAC = HMAC_SHA1(K,T)
\]

注意，传入 HMAC 的 Counter 必须是 8 字节的 Big Endian 整数。
第二步：Dynamic Truncation
HMAC-SHA1 会返回 20 字节的数据。

需要根据最后一个字节的低 4 位确定截取位置：
\[
offset = HMAC[19] \ \& \ 0x0F
\]

然后取连续 4 个字节：
\[
P = HMAC[offset..offset+4]
\]

将其转换为一个 31 位正整数：
\[
S = P \ \& \ 0x7FFFFFFF
\]

第三步：生成最终验证码
假设验证码长度是 6 位：
\[
OTP = S \bmod 10^6
\]

最终结果需要补齐 6 位。

例如：
计算结果：8231

最终 OTP：008231

这也是为什么 TOTP 验证码有时会以 0 开头。
3.3 为什么验证码每 30 秒变化？
因为 Counter 每 30 秒递增一次。
TOTP 时间窗口示意

1. 10:00:00 - 10:00:29
   482913
   Counter = N
2. 10:00:30 - 10:00:59
   193827
   Counter = N + 1
3. 10:01:00 - 10:01:29
   726105
   Counter = N + 2

需要注意：这里的时间是示意，实际时间窗口由 Unix 时间戳计算，而不是按用户本地时区计算。
3.4 为什么服务端允许前后 30 秒的时间误差？
实际环境中，用户手机和服务端的系统时间不一定完全同步。

例如：
手机时间：10:00:31
服务端时间：10:00:29

如果严格使用当前 Counter，双方可能生成不同的验证码。

因此，服务端通常允许一个小范围的时间偏移：
当前 Counter - 1
当前 Counter
当前 Counter + 1

也就是最多容忍前后一个时间窗口的偏差。

但窗口越大，攻击者猜中验证码的概率越高，因此不应该无限扩大允许范围。

以上算法和默认参数来自 IETF RFC 6238。

RFC Editor+1

四、Rust 实现 TOTP
下面开始进入代码实现。

我们不依赖第三方 MFA 服务，而是直接使用 Rust 密码学库实现 TOTP 的核心逻辑。
4.1 创建 Rust 项目
cargo new rust-mfa
cd rust-mfa

在 Cargo.toml 中添加：

[dependencies]
hmac = "0.12"
sha1 = "0.10"
rand = "0.8"
data-encoding = "2"
subtle = "2"
url = "2"

各个依赖的用途：
Crate 用途
hmac HMAC 消息认证码
sha1 TOTP 默认使用的 SHA-1
rand 生成随机 Secret
data-encoding Base32 编码和解码
subtle 常量时间比较，减少时序侧信道
url 构造 otpauth URI

这里使用 SHA-1 是为了兼容大量现有 Authenticator 应用。RFC 6238 也允许 SHA-256 和 SHA-512。
4.2 生成 TOTP Secret
首先需要给每个用户生成一个唯一的随机 Secret。

Secret 是 TOTP 最重要的安全凭据。

它必须满足：

- 随机生成。
- 不同用户之间不能共享。
- 长度足够。
- 不能泄露给其他用户。
- 服务端必须安全保存。
  建议使用 20 字节（160 bit）的随机数据。
  use rand::rngs::OsRng;
  use rand::RngCore;
  use data_encoding::BASE32_NOPAD;

pub fn generate_secret() -> String {
let mut secret = [0u8; 20];

    OsRng.fill_bytes(&mut secret);

    BASE32_NOPAD.encode(&secret)

}

生成结果类似：
JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP

这里使用 Base32 而不是 Base64，主要是因为 Google Authenticator 等认证应用广泛采用 Base32 格式表示 TOTP Secret。

注意：这个 Secret 不是用户的验证码，而是用于计算验证码的长期共享密钥。
4.3 实现 TOTP 算法
这是整个实现中最核心的部分。

use hmac::{Hmac, Mac};
use sha1::Sha1;
use data_encoding::BASE32_NOPAD;

type HmacSha1 = Hmac<Sha1>;

/// 根据 Secret 和 Unix 时间戳生成 TOTP
pub fn generate*totp(
secret: &str,
unix_time: u64,
) -> Result<String, &'static str> {
// 1. Base32 解码
let secret_bytes = BASE32_NOPAD
.decode(secret.as_bytes())
.map_err(|*| "invalid secret")?;

    // 2. 计算时间 Counter
    let counter = unix_time / 30;

    // 3. Counter 转换为 8 字节 Big Endian
    let counter_bytes = counter.to_be_bytes();

    // 4. HMAC-SHA1
    let mut mac = HmacSha1::new_from_slice(&secret_bytes)
        .map_err(|_| "invalid secret")?;

    mac.update(&counter_bytes);

    let hash = mac.finalize().into_bytes();

    // 5. Dynamic Truncation
    let offset = (hash[19] & 0x0f) as usize;

    let binary = ((hash[offset] & 0x7f) as u32) << 24
        | (hash[offset + 1] as u32) << 16
        | (hash[offset + 2] as u32) << 8
        | (hash[offset + 3] as u32);

    // 6. 生成 6 位验证码
    let otp = binary % 1_000_000;

    Ok(format!("{:06}", otp))

}

这段代码实现了 RFC 6238 的核心算法。

特别注意几个细节：

第一，Counter 必须使用 Big Endian。
let counter_bytes = counter.to_be_bytes();

第二，Dynamic Truncation 需要屏蔽最高位。
hash[offset] & 0x7f

这是为了得到 31 位正整数。

第三，必须使用字符串格式化补齐 6 位。
format!("{:06}", otp)

否则生成 001234 时，可能被错误地转换成 1234。
4.4 实现 TOTP 验证
生成验证码并不难，真正用于服务端的是验证码验证。

验证时需要考虑时间误差。
use subtle::ConstantTimeEq;

pub fn verify_totp(
secret: &str,
input_code: &str,
unix_time: u64,
) -> bool {
// 只允许 6 位数字
if input_code.len() != 6
|| !input_code.bytes().all(|b| b.is_ascii_digit())
{
return false;
}

    let current_counter = unix_time / 30;

    // 允许前后一个时间窗口
    for offset in -1i64..=1 {
        let counter = current_counter as i64 + offset;

        if counter < 0 {
            continue;
        }

        let timestamp = counter as u64 * 30;

        if let Ok(expected) = generate_totp(secret, timestamp) {
            if expected.as_bytes()
                .ct_eq(input_code.as_bytes())
                .into()
            {
                return true;
            }
        }
    }

    false

}

这里使用了 subtle 的常量时间比较：
expected.as_bytes().ct_eq(input_code.as_bytes())

相比普通字符串比较，它可以减少因比较提前退出而产生的时序信息泄露。

不过，完整的服务端验证还需要处理验证码重放问题，后面会讲。
4.5 测试 TOTP
可以在 main.rs 中进行测试：

fn main() {
let secret = generate_secret();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let code = generate_totp(&secret, now).unwrap();

    println!("Secret: {}", secret);
    println!("TOTP: {}", code);

    let valid = verify_totp(
        &secret,
        &code,
        now,
    );

    println!("Valid: {}", valid);

}

运行：
cargo run

输出示意：
Secret: JBSWY3DPEHPK3PXP...
TOTP: 482913
Valid: true

实际生成的 Secret 和验证码每次运行都可能不同。

五、让 Google Authenticator 能够识别你的 Rust 服务
前面我们已经实现了 TOTP 算法，但目前还存在一个问题：

用户如何把自己的 Secret 导入 Google Authenticator？

通常采用 QR Code（二维码）完成这个过程。

这里涉及一个非常重要的 URI 协议：otpauth://。
5.1 otpauth URI
认证应用普遍支持这种格式：
otpauth://totp/{ISSUER}:{ACCOUNT}
?secret={SECRET}
&issuer={ISSUER}
&algorithm=SHA1
&digits=6
&period=30

例如：
otpauth://totp/MyApp:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=MyApp&algorithm=SHA1&digits=6&period=30

各参数含义：
参数 说明
totp 使用 TOTP 算法
ISSUER 应用名称
ACCOUNT 用户账户标识
secret 用户的 TOTP Secret
algorithm HMAC 算法
digits 验证码位数
period 时间窗口，单位秒

其中 issuer 建议设置成你的应用名称，例如 MyApp。
5.2 Rust 生成 otpauth URI
利用之前添加的 url crate：

use url::Url;

pub fn generate*otpauth_uri(
issuer: &str,
account: &str,
secret: &str,
) -> Result<String, &'static str> {
let mut url = Url::parse("otpauth://totp/")
.map_err(|*| "invalid url")?;

    let label = format!("{}:{}", issuer, account);

    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "invalid path")?;

        segments.push(&label);
    }

    url.query_pairs_mut()
        .append_pair("secret", secret)
        .append_pair("issuer", issuer)
        .append_pair("algorithm", "SHA1")
        .append_pair("digits", "6")
        .append_pair("period", "30");

    Ok(url.to_string())

}

这样就可以得到一个标准的 TOTP URI。
5.3 生成二维码
接下来可以使用 Rust 的 qrcode crate。

在 Cargo.toml 中增加：
qrcode = "0.14"

生成二维码：
use qrcode::QrCode;
use qrcode::render::unicode;

fn print_qr_code(uri: &str) {
let code = QrCode::new(uri).unwrap();

    let image = code
        .render::<unicode::Dense1x2>()
        .build();

    println!("{}", image);

}

完整的注册过程：

1. 用户开启 MFA

   服务端为用户生成随机 Secret。

2. 生成 otpauth URI

   将 Secret、账户名称和算法参数组合起来。

3. 生成 QR Code

   向用户展示二维码。

4. 用户扫描二维码

   使用 Google Authenticator 等应用导入 Secret。

5. 用户输入验证码

   服务端验证用户手机生成的 TOTP。

6. 正式启用 MFA

   只有验证成功后，才将 MFA 状态设置为 Enabled。

重要安全要求
二维码中包含完整的 TOTP Secret。

因此：

- 不要将二维码 URI 写入日志。
- 不要将二维码保存到公开存储空间。
- 不要在用户尚未验证之前启用 MFA。
- 注册过程中，Secret 应处于 Pending 状态。
- 验证成功后，再将 Secret 标记为正式启用。

六、将 MFA 集成到实际 Rust Web 服务
到这里，我们实现的是 TOTP 算法。

但实际开发中，真正需要设计的是完整的 MFA 认证流程。

我以一个典型的 Rust RESTful API 服务为例。
6.1 用户数据库设计
首先需要为用户增加 MFA 相关字段。

例如：
ALTER TABLE users
ADD COLUMN mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE users
ADD COLUMN mfa_secret_encrypted TEXT;

ALTER TABLE users
ADD COLUMN mfa_pending_secret_encrypted TEXT;

ALTER TABLE users
ADD COLUMN mfa_last_accepted_step BIGINT;

字段解释：
字段 用途
mfa_enabled 是否启用 MFA
mfa_secret_encrypted 已启用的 TOTP Secret
mfa_pending_secret_encrypted 尚未完成验证的 Secret
mfa_last_accepted_step 最近一次成功使用的时间窗口

这里有个很重要的设计：

TOTP Secret 不应该像普通密码一样直接进行单向 Hash。

因为服务端需要使用它重新计算验证码，所以必须能够恢复原始 Secret。

正确方式是：

- 数据库中保存加密后的 Secret。
- 使用独立的密钥进行 AES-256-GCM 等认证加密。
- 加密密钥存储在 KMS、Secrets Manager 或独立密钥管理系统中。
- 不要把加密密钥和数据库放在一起。
  6.2 MFA 注册接口
  建议设计以下接口：
  HTTP Method Endpoint 功能
  POST /auth/mfa/setup 开始 MFA 注册
  POST /auth/mfa/verify 验证注册验证码
  POST /auth/mfa/disable 关闭 MFA
  POST /auth/mfa/challenge 登录时验证 TOTP
  GET /auth/mfa/status 查询 MFA 状态
  POST /auth/mfa/recovery 使用恢复码恢复账户

注意：这里是 API 设计建议，不是必须遵循的固定规范。
setup 接口
POST /auth/mfa/setup

服务端：

1. 检查用户是否已经登录。
2. 要求用户重新输入密码或进行其他强认证。
3. 生成随机 Secret。
4. 加密 Secret。
5. 保存到 mfa_pending_secret_encrypted。
6. 生成 otpauth URI。
7. 返回二维码或 URI。
   此时：
   mfa_enabled = false

verify 接口
POST /auth/mfa/verify

请求：
{
"code": "482913"
}

服务端：

1. 获取 Pending Secret。
2. 解密 Secret。
3. 验证 TOTP。
4. 验证成功后，将 Pending Secret 转为正式 Secret。
5. 设置 mfa_enabled = true。
6. 生成恢复码。
   伪代码：
   if verify_totp(&secret, &request.code, now) {
   // 将 pending secret 转为正式 secret
   // 设置 mfa_enabled = true
   // 生成并保存 recovery codes
   } else {
   // 返回验证码错误
   }

这里还应该在数据库事务中完成状态转换，避免并发请求导致状态不一致。
6.3 登录流程
这是 MFA 最重要的部分。

传统登录：
Client
|
| username + password
v
Server
|
| Verify Password
v
Authenticated
|
v
JWT

启用 MFA 后：
Client
|
| username + password
v
Server
|
| Verify Password
v
Check MFA Enabled
|
+------------------------+
| |
| Disabled | Enabled
v v
Authenticated MFA Challenge
| |
v | TOTP Code
JWT v
Verify TOTP
|
+-----+-----+
| |
Failed Success
| |
v v
Reject Authenticated
|
v
JWT

这里有一个容易被忽略的问题：

第一次验证密码成功，并不意味着用户已经完成认证。

如果用户启用了 MFA，那么在 TOTP 验证成功之前，不能签发具有完整访问权限的 JWT。

可以考虑先签发一个短时效、权限受限的 MFA Challenge Token：
{
"sub": "user-123",
"purpose": "mfa_challenge",
"exp": 1794650700
}

服务端验证时必须检查：

- Token 签名是否正确。
- 是否过期。
- purpose 是否为 mfa_challenge。
- 是否对应当前用户。
- 是否已经被使用。
  验证 TOTP 成功后，再签发正式 Access Token。

七、实际生产环境必须解决的安全问题
前面的代码只是核心算法，并不意味着直接部署就具备生产环境所需的安全性。

下面几个问题尤其重要。
7.1 防止 TOTP 重放攻击（Replay Attack）
这是最容易遗漏的问题。

假设当前时间窗口为：
10:00:00 - 10:00:29

用户生成验证码：
482913

用户第一次提交：
482913 -> 验证成功

攻击者如果获得了这个验证码，在同一个时间窗口内再次提交：
482913 -> 也可能验证成功

因为 TOTP 在一个时间窗口内不会变化。

因此，仅仅检查验证码是否正确是不够的。

我们需要保存：
mfa_last_accepted_step

例如：
last_accepted_step = 59821680

只有当新验证码对应的 Counter 大于上一次成功使用的 Counter 时，才允许通过。

实现逻辑：
if matched_step <= last_accepted_step {
return Err("OTP already used");
}

注意，这个检查必须和更新操作在数据库事务中原子完成。

例如 PostgreSQL 可以使用：
UPDATE users
SET mfa_last_accepted_step = $1
WHERE id = $2
AND (
mfa_last_accepted_step IS NULL
OR mfa_last_accepted_step < $1
)
RETURNING id;

如果没有返回记录，就说明该时间窗口已经被使用，或者并发请求已经抢先完成验证。

这也是为什么前面示例的 verify_totp 返回 bool 还不够完善。生产环境建议改成：
pub fn verify_totp(
secret: &str,
input_code: &str,
unix_time: u64,
) -> Option<u64>

返回匹配成功的 Counter，而不是只返回 true 或 false。
7.2 防止暴力破解
6 位 TOTP 只有：
\[
10^6 = 1,000,000
\]

种可能。

因此必须限制验证次数。

例如：
同一账户：

5 次失败
|
v
触发限流或短暂锁定
|
v
等待一段时间
|
v
重新允许验证

建议结合：

- 每账户限流。
- 每 IP 限流。
- 失败次数统计。
- 异常登录审计。
- 必要时增加验证码或风险验证。
  不要只按 IP 限制，因为攻击者可以切换 IP。
  7.3 Recovery Codes（恢复码）
  用户可能丢失手机、卸载认证应用，或者更换设备。

如果没有恢复机制，用户就可能永久失去账户访问权限。

因此，MFA 通常还需要 Recovery Codes。

例如：
A7K9-M3PX-8Q2L
B5D8-N7TW-4R6C
C2F9-H6JK-1M8P

用户启用 MFA 时，服务端生成若干恢复码。

恢复码应该：

1. 使用密码学安全随机数生成。
2. 每个恢复码只能使用一次。
3. 数据库只保存恢复码的安全 Hash。
4. 用户使用成功后立即标记为已消费。
5. 重新生成恢复码时，使旧恢复码全部失效。
   另外，恢复码的验证和消费也必须原子化，防止并发重放。
   7.4 MFA Secret 的安全存储
   这个问题值得特别强调。

例如数据库中保存：
user_id: 123
mfa_secret: JBSWY3DPEHPK3PXP

这是不安全的。

如果数据库泄露，攻击者就能复制用户的 TOTP Secret，生成正确的验证码。

建议使用信封加密（Envelope Encryption）：
Secret 加密存储流程
TOTP Secret（明文）

AES-256-GCM

使用 Data Encryption Key（DEK）加密

Ciphertext + Nonce + Authentication Tag

数据库存储加密结果
DEK 应由独立 KMS 管理的 KEK 保护，避免密钥和密文一起泄露。

加密时还需要考虑：

- 每次加密使用唯一随机 Nonce。
- 保存密钥版本，以便将来轮换密钥。
- 可以将 user_id 作为 Additional Authenticated Data（AAD），绑定密文所属用户。
- 不要记录明文 Secret、OTP 或完整 otpauth URI。
  7.5 TOTP 并不能防止钓鱼攻击
  这是一个非常重要的安全认知。

假设攻击者搭建了一个仿冒登录网站：
用户
|
| 输入用户名、密码和 TOTP
v
钓鱼网站
|
| 转发用户名、密码和 TOTP
v
真实网站
|
v
攻击者获取登录会话

因为 TOTP 验证码没有绑定特定网站或特定登录会话，所以攻击者可以实时转发验证码。

因此：

- TOTP 比单纯密码认证安全得多。
- TOTP 能有效抵御大量密码泄露攻击。
- TOTP 不能从协议层面抵御实时钓鱼攻击。
  如果你正在构建高安全等级的认证系统，建议考虑 WebAuthn / Passkeys。

NIST 也明确指出，手动输入 OTP 不具备抗钓鱼能力。

NIST Pages+2

八、推荐的 Rust 项目架构
如果你打算把 MFA 做成一个独立的 Rust 模块，我建议不要把 TOTP 算法、用户认证、数据库操作全部写在一起。

可以按照职责划分：
src/
├── main.rs
├── config.rs
│
├── auth/
│ ├── mod.rs
│ ├── password.rs
│ ├── jwt.rs
│ └── session.rs
│
├── mfa/
│ ├── mod.rs
│ ├── totp.rs
│ ├── secret.rs
│ ├── encryption.rs
│ ├── recovery.rs
│ └── service.rs
│
├── repository/
│ ├── mod.rs
│ └── user_repository.rs
│
└── api/
├── mod.rs
└── mfa_handler.rs

各模块职责：
模块 职责
totp.rs TOTP 算法及验证码验证
secret.rs Secret 生成、Base32 编解码
encryption.rs Secret 加密和解密
recovery.rs 恢复码生成、验证和消费
service.rs MFA 注册、启用、关闭、登录验证
mfa_handler.rs HTTP API 接口

如果使用你熟悉的 Rust 异步生态，可以采用：

- Axum：HTTP 服务。
- SQLx：数据库访问。
- Tokio：异步运行时。
- Redis：验证码限流、短期 Challenge 状态。
- aes-gcm：Secret 加密。
- rand：安全随机数。
- hmac + sha1：TOTP 算法。
  其中，Redis 适合保存短期登录 Challenge、限流计数等数据，但不要只依赖 Redis 的非原子读写来防止 OTP 重放。应通过数据库条件更新、事务或者 Redis Lua 脚本保证原子性。
  九、推荐阅读的技术规范
  如果想进一步理解 MFA，我建议阅读以下资料。

1. RFC 6238 — TOTP
   必读

这是 TOTP 算法的正式技术规范，包含算法定义、参数以及测试向量。
RFC 6238 官方文档 2. OWASP Multifactor Authentication Cheat Sheet
必读

介绍 MFA 的实现建议、验证码存储、恢复机制、攻击方式以及安全防护。
OWASP MFA 安全指南 3. NIST SP 800-63B-4

数字身份认证和认证器管理方面的重要指南，适合进一步研究 MFA 的安全等级和抗钓鱼认证。
NIST 官方文档 4. Rust totp-rs

一个 Rust TOTP 实现库，可以参考它的 API 设计和实际使用方式。
totp-rs GitHub 项目
十、总结：应该如何选择 MFA 技术？
最后把几种常见方案放在一起比较：
技术 安全性 实现复杂度 推荐程度
SMS OTP 较低 低 不建议作为首选
Email OTP 依赖邮箱安全性 低 适合低风险场景
TOTP 较高 中 推荐
HOTP 较高 中 特定场景
WebAuthn / Passkeys 很高 较高 优先推荐

我的建议是：

如果你想在 Rust 项目中第一次实现 MFA，可以按照以下顺序：

1. 实现 TOTP 算法。
2. 实现 Secret 生成和加密存储。
3. 实现 otpauth:// URI 和二维码生成。
4. 实现 MFA 注册、验证和启用流程。
5. 将 MFA 集成到 JWT 登录认证流程。
6. 实现 OTP 防重放、限流和 Recovery Codes。
7. 最后考虑 WebAuthn / Passkeys。
   特别强调： 上面的 Rust 代码实现了 TOTP 的核心算法，但生产环境还需要 RFC 测试向量验证、Secret 加密、原子防重放、限流和安全的账户恢复机制，不能直接把前面的几个函数当作完整的生产级 MFA 系统。

如果继续深入，我建议下一步直接使用 Axum + SQLx + Redis，实现一个完整的 Rust MFA 模块，包括数据库表结构、RESTful API、JWT 两阶段认证和 TOTP 防重放机制。
