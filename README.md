# pkg-owner

用一行命令查询 macOS 图形应用和命令行软件的包管理来源。Rust 实现，编译后的程序无需 Python 或 Rust 运行时。查询不会安装或更新软件。

## 构建与运行

需要 Rust/Cargo 工具链及 macOS 命令行开发工具。在项目目录执行：

```sh
cargo build --release --locked
./bin/pkg-owner -s claude
```

`bin/pkg-owner` 是兼容原使用方式的启动脚本，仅调用 `target/release/pkg-owner`。首次使用和修改源码后需要重新构建；脚本不会自动下载依赖或编译。也可直接调用二进制：

```sh
./target/release/pkg-owner python3
./target/release/pkg-owner /opt/homebrew/bin/git
```

全局安装（安装到 Cargo 的 bin 目录，通常是 `~/.cargo/bin`）：

```sh
cargo install --path . --locked
pkg-owner -s chrome
```

或继续使用原来的 PATH 设置：

```sh
export PATH="$HOME/Project/pkg-owner/bin:$PATH"
```

## 查询参数

| 快捷参数 | 长参数 | 用途 |
| --- | --- | --- |
| `-s TEXT` | `--search TEXT` | 模糊搜索 |
| `-l` | `--list` | 列出全部 |
| `-j` | `--json` | JSON 输出 |
| `-d PATH` | `--scan-dir PATH` | 额外扫描目录，可重复 |
| `-h` | `--help` | 显示帮助 |
| `-V` | `--version` | 显示版本 |

```sh
./bin/pkg-owner python3
./bin/pkg-owner 'Visual Studio Code'
./bin/pkg-owner -l
./bin/pkg-owner -s chorme
./bin/pkg-owner -s python -j
./bin/pkg-owner -l -d ~/Tools -d /Volumes/Software
```

长短参数等价；`--list`、`--search`、位置参数互斥。`--search` 忽略大小写和标点，支持子串与相似拼写；相似度使用最长公共匹配块递归计算，阈值 0.72，保留原版常规软件名称的匹配行为。名称、别名、路径及其分词参与搜索。位置参数在同样的字符归一化后精确匹配。多个安装来源或版本可能同时出现。

文本表格按 Unicode 显示宽度对齐，支持中文、组合字符和长应用名，并转义控制字符。超长路径保持完整，终端较窄时可能自然折行；机器读取请使用 `--json`。

## 识别范围

| 来源 | 依据 |
| --- | --- |
| Homebrew Formula / Cask | 当前 PATH 中 brew 的已安装 JSON 记录，Formula 文件路径和 Cask 应用目标 |
| npm 全局包 | 当前 npm 的全局包记录及模块路径，可追溯符号链接命令 |
| pipx | pipx list JSON 安装记录 |
| uv tool | uv 工具目录中的 uv-receipt.toml 安装记录，不调用可能创建缓存的 uv tool list |
| Cargo | cargo install --list 安装记录 |
| MacPorts | port installed 中的 active 记录 |
| App Store | 应用内 `_MASReceipt/receipt` 存在 |
| macOS | 系统应用或系统命令目录 |
| Unknown | 已找到软件，但未关联受支持的管理器记录 |

应用扫描包含 `/Applications`、`~/Applications`、`/System/Applications`、Spotlight 索引中的其他 `.app` 和 `--scan-dir` 指定目录；命令扫描包含 PATH 下的可执行文件，也支持显式文件路径。不会进入 `.app` 内部递归列出嵌套应用，也不递归跟随目录符号链接。

## 证据边界

- 管理器记录表示当前登记状态，不能证明最后一次覆盖安装的来源。已登记但被手动移除的软件仍可能出现在管理器结果中。
- 未索引的其他目录需要 `--scan-dir`。不进行全磁盘扫描。
- 只查询当前 PATH/环境所选择的各管理器实例；不枚举所有 Conda、venv、Node 版本和第二套 Homebrew。项目本地依赖与普通 pip 库不属于本次清单范围。
- pipx、Cargo、MacPorts 提供包名登记结果；未关联路径的独立命令可能另列为 Unknown。不会仅凭同名断言文件所有权。
- PKG 安装收据不等于持续管理来源，本版不将历史 PKG 收据推断为包管理器；不根据扩展属性推断“官网安装”。
- 管理器并发采集，异常或超时以警告输出，其余扫描继续。单条外部命令超时为 40 秒。Spotlight 不可用时回退目录扫描；不可读目录可能被跳过。
- `--json` 保留 `results`、`warnings`、`managers_checked`；每项含 `name`、`manager`、`evidence`、`paths` 和 `aliases`。退出码：成功 0，无匹配 1，参数错误 2。
- 当前面向 macOS，使用 Unix 文件权限和进程组；未声明 Windows 支持。

## 开发与验证

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

模块：`collectors.rs` 采集管理器记录，`inventory.rs` 扫描并关联文件，`process.rs` 管理外部命令及超时，`lib.rs` 搜索和表格渲染，`main.rs` 解析参数并输出。单元测试覆盖搜索、列宽、路径归属和超时；集成测试使用隔离的假管理器验证六类来源、符号链接、短参数、失败降级及自定义应用目录。

项目也包含 Codex 插件清单和技能；无需安装到 Codex 即可独立使用命令行。未自动写入个人 marketplace。
