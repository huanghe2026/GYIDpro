# 贡献指南 - GyID

感谢你对 GyID 项目感兴趣！本文档描述如何参与贡献。

---

## 目录

1. [行为准则](#行为准则)
2. [开始之前](#开始之前)
3. [开发环境搭建](#开发环境搭建)
4. [代码规范](#代码规范)
5. [提交流程](#提交流程)
6. [测试要求](#测试要求)
7. [模块说明](#模块说明)

---

## 行为准则

- 尊重所有贡献者，欢迎建设性讨论
- 提交前请确保代码编译通过、测试全绿
- 保持提交信息清晰简洁

---

## 开始之前

### 前置条件

- Rust `stable` 工具链（推荐 `stable-x86_64-pc-windows-gnu`）
- Windows 用户需安装 MSYS2（`C:\msys64\mingw64\bin` 需在 PATH 中）
- 可选：Dioxus CLI（`cargo install dioxus-cli`，用于 GUI 开发）

### 克隆仓库

```bash
git clone https://github.com/your-org/gyid.git
cd gyid
```

---

## 开发环境搭建

```bash
# 安装工具链（项目已有 rust-toolchain.toml，自动选择）
rustup show

# 编译所有模块
cargo build --workspace

# 运行测试
cargo test --workspace

# Clippy 检查（必须零警告）
cargo clippy --workspace -- -D warnings

# 格式化代码
cargo fmt --all
```

---

## 代码规范

### 必须遵守

1. **零 Clippy 警告** - `cargo clippy --workspace -- -D warnings` 必须通过
2. **代码格式** - 提交前运行 `cargo fmt --all`
3. **无 `unsafe` 裸代码** - 凡涉及 unsafe 块，必须附注释说明原因
4. **错误传播** - 使用 `?` 传播错误，避免 `unwrap()`（测试除外）

### 命名约定

| 场景 | 约定 | 示例 |
|------|------|------|
| 类型/结构体 | `PascalCase` | `GyIdGenerator` |
| 函数/方法 | `snake_case` | `generate_gyid` |
| 常量 | `SCREAMING_SNAKE_CASE` | `DEFAULT_GEO_LEVEL` |
| 模块 | `snake_case` | `geo/ip.rs` |

### 注释规范

- 公开 API 必须有文档注释（`///`）
- 复杂逻辑需行内注释说明意图
- TODO 格式：`// TODO(your-name): description`

---

## 提交流程

### 分支策略

```
main          ← 稳定版本
dev           ← 开发分支（PR 目标）
feature/xxx   ← 功能分支
fix/xxx       ← 修复分支
```

### Commit 消息规范

格式：`<type>(<scope>): <message>`

| type | 说明 |
|------|------|
| `feat` | 新功能 |
| `fix` | 修复 bug |
| `docs` | 文档变更 |
| `refactor` | 代码重构 |
| `test` | 测试相关 |
| `chore` | 构建/CI 相关 |

示例：
```
feat(geo): 添加 WiFi BSSID 离线降级支持
fix(storage): 修复 LocalStorage::new 路径参数不一致问题
docs(api): 完善 ChainAnchor 链上锚定 API 说明
```

### Pull Request

1. 从 `dev` 分支创建功能分支
2. 在 PR 描述中说明：改动内容、测试方式、影响范围
3. 确保 CI 检查全部通过
4. 至少一位 reviewer 审核后合并

---

## 测试要求

### 测试覆盖

每个新功能或 bug 修复都需要对应的测试：

```bash
# 运行所有测试
cargo test --workspace

# 运行特定模块测试
cargo test -p gyid-core

# 运行集成测试
cargo test --test '*'

# 显示测试输出
cargo test -- --nocapture
```

### 测试规范

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_功能名称_场景描述() {
        // 三段式：arrange / act / assert
        let input = ...;        // arrange
        let result = func(input); // act
        assert_eq!(result, expected); // assert
    }

    // 需要网络/环境变量的测试用 #[ignore]
    #[tokio::test]
    #[ignore]
    async fn test_network_dependent() { ... }
}
```

### 禁止提交带有失败测试的代码

```bash
# 提交前必须通过
cargo test --workspace 2>&1 | tail -5
# 应看到：test result: ok. X passed; 0 failed
```

---

## 模块说明

### gyid-core

核心库，包含所有身份生成逻辑：

| 模块 | 路径 | 说明 |
|------|------|------|
| fingerprint | `src/fingerprint/` | 硬件指纹采集（MAC/CPU/Board/Disk） |
| geo | `src/geo/` | 地理位置（IP/WiFi/GPS/H3） |
| avatar | `src/avatar/` | 头像处理与 BLAKE3 哈希 |
| crypto | `src/crypto/` | BLAKE3 哈希工具 + Base58 编码 |
| identity | `src/identity/` | 生成器/验证器/配置 |
| device | `src/device/` | 设备关联（主/从设备） |
| storage | `src/storage/` | 本地 SQLite + 链上锚定 |

### gyid-cli

命令行工具，依赖 `gyid-core`。添加新命令步骤：
1. 在 `gyid-cli/src/commands/` 新建 `xxx.rs`
2. 在 `mod.rs` 中注册
3. 在 `main.rs` 的 CLI 路由中添加分支

### gyid-gui

Dioxus GUI 应用。页面位于 `src/pages/`，组件位于 `src/components/`。
状态管理使用 `src/state/mod.rs` 中的 `AppState`。

### gyid-sdk

跨平台 SDK，提供 WASM / Android JNI / iOS Swift / C FFI 绑定。
统一 API 入口位于 `src/api.rs`。

---

## 发布流程

1. 更新 `Cargo.toml` 版本号（workspace 根 + 各 crate）
2. 更新 `DEVELOPMENT_PLAN.md` 和 `README.md` 版本号
3. 运行完整测试套件
4. 打 git tag：`git tag -a v0.x.y -m "Release v0.x.y"`
5. 推送 tag 触发 CI 发布

---

*文档版本: v1.0*  
*创建日期: 2026-04-09*
