# pkg-owner

**查询 macOS 上的应用和命令由哪个包管理器管理，并查看判断依据。**

同一个软件可能通过 Homebrew、npm、App Store 等方式安装。`pkg-owner` 汇总当前环境中的管理器记录、应用目录和 PATH 命令，帮助你在更新、卸载或排查多版本问题前确认来源。

- 同时查询 `.app` 应用、命令名和文件路径。
- 支持模糊搜索、拼写近似匹配和全量列表。
- 提供文本表格及包含证据、路径、别名和警告的 JSON 输出。
- 使用 Rust 实现，编译后的程序无需 Python 或 Rust 运行时。

查询只读取安装信息，不执行软件安装、更新或卸载。**识别结果反映当前记录，不能证明软件最后一次被谁安装或覆盖。**

## 快速开始

### 从源码安装

需要 macOS、Rust/Cargo 工具链及 macOS 命令行开发工具。

```sh
git clone https://github.com/houtianyihou/pkg-owner.git
cd pkg-owner
cargo install --path . --locked
pkg-owner --help
```

Cargo 默认将可执行文件安装到 `~/.cargo/bin`。如果提示找不到 `pkg-owner`，请将 Cargo 的 bin 目录加入 PATH。

### 在项目目录运行

不做全局安装也可以使用：

```sh
cargo build --release --locked
./bin/pkg-owner -s chrome
```

`bin/pkg-owner` 是启动脚本，调用项目中的 `target/release/pkg-owner`；也可以直接运行该二进制。首次使用和修改源码后都需要手动构建，启动脚本不会自动下载依赖或编译。

## 常用查询

```sh
# 按命令名或完整应用名查询
pkg-owner python3
pkg-owner 'Visual Studio Code'

# 按路径查询；路径含空格时加引号
pkg-owner /usr/bin/git
pkg-owner '/Applications/Visual Studio Code.app'

# 不确定完整名称或拼写时，使用模糊搜索
pkg-owner -s chrome
pkg-owner -s chorme

# 列出当前发现的全部软件与命令
pkg-owner -l

# 补充应用扫描目录，可指定多个
pkg-owner -l -d ~/Tools -d /Volumes/Software

# 输出 JSON，适合脚本处理
pkg-owner -s python -j
```

位置参数按归一化后的名称、别名或路径精确匹配；`--search` 还支持子串和近似拼写匹配。两者均忽略大小写与标点，模糊搜索也会匹配名称和路径中的分词。同名软件或多个安装来源可能返回多条结果。

### 参数

| 参数 | 含义 |
| --- | --- |
| `[APPLICATION]` | 软件名、命令名或路径 |
| `-s, --search TEXT` | 模糊搜索 |
| `-l, --list` | 列出全部发现结果 |
| `-d, --scan-dir PATH` | 额外递归扫描应用的目录，可重复使用 |
| `-j, --json` | 输出 JSON，包含证据与扫描警告 |
| `-h, --help` | 显示帮助 |
| `-V, --version` | 显示版本 |

位置参数、`--search` 和 `--list` 互斥。不带查询参数时显示帮助。

## 如何解读结果

默认表格显示「名称」「管理来源」「路径 / 证据」。每条记录最多展示前三个路径；没有路径时显示判断依据。表格支持中文显示宽度对齐，长路径可能随终端宽度折行。需要完整路径和证据时使用 `--json`。

**`Unknown` 表示找到了软件，但没有关联到受支持的管理器记录，不等于“手动安装”或“官网安装”。** 包管理器登记的包与未能关联路径的命令也可能分别出现。

### JSON 结构

以下仅演示字段结构，不代表某台机器的实际扫描结果：

```json
{
  "results": [
    {
      "name": "example-tool",
      "manager": "Unknown",
      "evidence": "PATH 或指定路径；无已关联管理记录",
      "paths": ["/usr/local/bin/example-tool"],
      "aliases": []
    }
  ],
  "warnings": [],
  "managers_checked": []
}
```

| 字段 | 含义 |
| --- | --- |
| `results` | 匹配的记录；每项包含名称、来源、证据、路径和别名 |
| `warnings` | 管理器采集失败或超时的信息 |
| `managers_checked` | 在当前 PATH 中发现并尝试采集的管理器命令名，不表示采集全部成功 |

文本模式将管理器警告写入标准错误；JSON 模式将其放在 `warnings` 中。单个管理器失败不会阻止其他结果输出，因此有结果不等于扫描完整。

退出码：`0` 表示查询有匹配，或正常执行列表、帮助等操作；`1` 表示查询无匹配或发生非管道关闭的输出错误；`2` 表示参数错误。`--list` 即使结果为空也返回 `0`。脚本若要求完整采集，还应检查 `warnings`。

## 支持的来源

| 来源 | 识别依据 |
| --- | --- |
| Homebrew Formula | `brew info --json=v2 --installed` 记录及 Cellar 路径 |
| Homebrew Cask | Homebrew 安装记录中的应用或命令目标路径 |
| npm 全局包 | `npm ls -g` 记录及全局模块目录，可关联符号链接命令 |
| pipx | `pipx list --json` 中的包名与命令别名 |
| uv tool | 工具目录中的 `uv-receipt.toml` 文件；不调用 `uv tool list` |
| Cargo | `cargo install --list` 中的包名记录 |
| MacPorts | `port installed` 中标记为 active 的记录 |
| App Store | 应用包含 `Contents/_MASReceipt/receipt` |
| macOS | 系统应用目录或系统命令目录 |
| Unknown | 已发现应用或文件，但没有关联的受支持记录 |

无需安装所有管理器；程序只采集当前 PATH 中可找到的 `brew`、`npm`、`pipx`、`uv`、`cargo` 和 `port`。管理器并发采集，单条外部命令的超时为 40 秒，整个扫描可能包含多条命令。

### 扫描范围与限制

- **应用：** 扫描 `/Applications`、`~/Applications`、`/System/Applications`、Spotlight 索引中的其他应用，以及 `--scan-dir` 指定目录。目录递归扫描遇到 `.app` 后停止深入，不递归跟随目录符号链接。
- **命令：** 扫描 PATH 各目录下的可执行文件，也接受显式文件路径。路径可与管理器记录关联时，会补充到对应记录中。
- **非全盘扫描：** Spotlight 不可用时仍执行目录扫描；不可读目录可能被跳过。其他未索引目录需要通过 `--scan-dir` 补充。
- **环境范围：** 只查询当前环境选择的管理器实例，不枚举所有 Conda、venv、Node 版本或第二套 Homebrew，也不列出项目本地依赖与普通 pip 库。
- **归属边界：** 管理器登记不保证文件仍存在。pipx、Cargo、MacPorts 的包名记录不一定能关联到独立命令路径，程序不会仅凭同名判定文件归属。
- **历史来源：** 不使用历史 PKG 安装收据推断持续管理来源，也不根据扩展属性判断“官网安装”。

当前面向 macOS，使用 Unix 文件权限和进程组，未声明 Windows 支持。

## 开发

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

| 文件 | 职责 |
| --- | --- |
| [src/main.rs](src/main.rs) | 参数解析、输出与退出码 |
| [src/collectors.rs](src/collectors.rs) | 管理器安装记录采集 |
| [src/inventory.rs](src/inventory.rs) | 应用和命令扫描、路径归属关联 |
| [src/lib.rs](src/lib.rs) | 搜索匹配与表格渲染 |
| [src/process.rs](src/process.rs) | 外部命令执行、超时与进程组清理 |
| [tests/cli.rs](tests/cli.rs) | 使用隔离的模拟管理器验证命令行行为 |

单元测试覆盖模糊搜索、Unicode 列宽、路径归属、目录扫描和进程超时；集成测试覆盖六类管理器、符号链接、长短参数、失败降级与自定义应用目录。模拟管理器测试不等于验证了所有真实管理器版本。

## Codex 集成

仓库附带 [插件清单](.codex-plugin/plugin.json) 和 [pkg-owner 技能](skills/pkg-owner/SKILL.md)，供 Codex 集成使用。命令行工具可以独立运行，无需安装 Codex 插件；构建和 Cargo 安装不会自动注册插件或写入个人 marketplace。

## 许可证

[MIT](LICENSE)
