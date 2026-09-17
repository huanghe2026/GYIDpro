# Harness Engineering Rules

## 编译工具链规则

### 规则 1: 禁止使用 Visual Studio 编译此项目

**描述**: 此项目使用 MinGW-w64 (GCC) 工具链编译，不依赖 Visual Studio。

**原因**:
- 项目已配置 GNU 工具链 (`stable-x86_64-pc-windows-gnu`)
- 使用 MSYS2 + MinGW-w64 环境
- Rust 依赖使用 bundled C 库 (如 rusqlite)，无需 Windows SDK
- 网络请求使用 rustls-tls，无需 OpenSSL

**例外**: 无

**违反后果**: N/A (软规则，VS 可作为备用 MSVC 工具链，但非必需)

---

### 规则 2: 使用 GNU 工具链优先

**描述**: 默认使用 GCC (MinGW-w64) 而非 MSVC 编译 Rust 项目。

**命令**:
```powershell
# 确认当前工具链
rustup show

# 切换到 GNU 工具链
rustup default stable-x86_64-pc-windows-gnu
```

---

*创建时间: 2026-04-05*
