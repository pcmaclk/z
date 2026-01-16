# zedit

> **轻量 · 精致 · 大文件友好 · 单文件发布**

一个现代化的文本编辑器，定位为系统记事本的现代替代品，在性能与功能上超越 Notepad2。

## 项目定位

- **单文件分发**：最终产物为单个可执行文件
- **小体积**：Release 模式 <5MB，压缩后 <2MB
- **高性能**：支持大文件（100MB+）流畅编辑与高效撤销
- **现代外观**：符合各平台设计规范
- **跨平台**：Windows、macOS、Linux 原生支持

## 技术栈

- **编程语言**: Rust 1.75+
- **渲染层**: Slint 1.5+（仅作为渲染和事件捕获层）
- **文本核心**: Piece Table 缓冲区（为虚拟化与大文件编辑优化）
- **编辑模型**: 事务式状态机
- **架构模式**: 单向数据流 + 分层设计

## 架构

```
Input → Action → Editor Core → Transaction
     → Viewport → ViewModel → Render Adapter
```

详细架构文档请参考 [docs/03 ARCHITECTURE.md](docs/03 ARCHITECTURE.md)

## 开发路线

- **Phase 1 (v0.1.0)**: MVP 核心功能
- **Phase 2 (v0.2.0)**: 完整编辑器
- **Phase 3 (v0.3.0)**: 效率增强

详见 [docs/06 zedit 分阶段开发路线图.md](docs/06 zedit 分阶段开发路线图.md)

## 构建

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行
cargo run
```

## 测试

```bash
# 运行所有测试
cargo test

# 运行单元测试
cargo test --lib

# 运行集成测试
cargo test --test '*'

# 性能测试
cargo bench
```

## 贡献

欢迎贡献！请先阅读文档了解架构设计。

## 许可证

MIT License
