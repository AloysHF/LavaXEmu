# LavaXEmu

LavaXEmu 是一个用 Rust 编写的跨平台 LavaX 虚拟机，目标是运行 `.lav` 游戏并同时提供：

- 独立桌面模拟器；
- 可由 RetroArch 等前端加载的 libretro 核心；
- 无窗口、确定性的核心库，便于测试和移植。

项目目前处于早期开发阶段。LAV 文件头、117 个语言字节码、87 个系统 API 分派、索引色图形、输入、确定性时钟和内存虚拟文件系统已经接入，前端正在逐步完善。

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

## 独立模拟器

直接打开游戏：

```bash
cargo run --release -p lavaxemu -- game.lav
```

窗口支持整数初始缩放和自由调整大小。`F9` 暂停或继续，`F10` 复位，`F12` 退出。程序所在目录中的资源会装入虚拟文件系统，退出时只回写来宾修改过的文件。

无窗口运行、只读测试和截图：

```bash
cargo run --release -p lavaxemu -- game.lav --headless --frames 600 --read-only
cargo run --release -p lavaxemu -- game.lav --screenshot frame.png --frames 300 --read-only
```

游戏参数放在 `--` 之后，例如 `lavaxemu game.lav -- first second`。

## RetroArch 核心

默认 release 构建会生成 libretro 动态库：

```bash
cargo build --release
```

将生成的动态库改名为平台对应的 `lavaxemu_libretro` 文件名并放入 RetroArch 的核心目录，同时把 `crates/lavaxemu-libretro/lavaxemu_libretro.info` 放入 `info` 目录。核心接受 `.lav` 内容，输出 60 Hz RGB565 画面并支持即时存档。

手柄方向键对应方向键；A/B/X/Y 分别对应 LavaX 的 B/N/M/G 键，Select/Start 对应 H/J，L/R 对应 Page Up/Page Down。

## 许可证

项目采用 `GPL-2.0-or-later`。格式研究结论见 [docs/lavax-format.md](docs/lavax-format.md)。本地测试资源由 Git 忽略，不进入源码或发行包。

libretro API 是 MIT 许可的开放接口；RetroArch 是使用该接口的 GPLv3 前端，两者不是同一项目。
