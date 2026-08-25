# LavaXEmu

LavaXEmu 是一个用 Rust 编写的跨平台 LavaX 虚拟机，目标是运行 `.lav` 游戏并同时提供：

- 独立桌面模拟器；
- 可由 RetroArch 等前端加载的 libretro 核心；
- 无窗口、确定性的核心库，便于测试和移植。

项目目前处于早期开发阶段。LAV 文件头解析已经完成，字节码解释器、系统 API、图形和两个前端正在逐步接入。

## 工程结构

```text
crates/
├── lavaxemu-core/      # 平台无关的 LAV 加载器和虚拟机
├── lavaxemu/           # 独立桌面前端
└── lavaxemu-libretro/  # RetroArch/libretro 核心
```

## 构建

```bash
# 默认构建 libretro 核心
cargo build --release

# 构建独立模拟器
cargo build --release -p lavaxemu

# 运行测试
cargo test --workspace
```

## 许可证

项目采用 `GPL-2.0-or-later`。格式研究结论见 [docs/lavax-format.md](docs/lavax-format.md)。本地测试资源由 Git 忽略，不进入源码或发行包。

libretro API 是 MIT 许可的开放接口；RetroArch 是使用该接口的 GPLv3 前端，两者不是同一项目。
