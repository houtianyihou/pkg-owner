# pkg-owner

用一行命令查询 macOS 图形应用和命令行软件的包管理来源。Python 3 标准库实现，无第三方依赖；所有查询只读。

在本目录运行：

```sh
./bin/pkg-owner python3
./bin/pkg-owner 'Visual Studio Code'
./bin/pkg-owner /opt/homebrew/bin/git
./bin/pkg-owner --list
./bin/pkg-owner --search chorme
./bin/pkg-owner --search python --json
./bin/pkg-owner --list --scan-dir ~/Tools --scan-dir /Volumes/Software
```

快捷参数与长参数等价，可以组合使用：

| 快捷参数 | 长参数 | 用途 |
| --- | --- | --- |
| `-s TEXT` | `--search TEXT` | 模糊搜索 |
| `-l` | `--list` | 列出全部 |
| `-j` | `--json` | JSON 输出 |
| `-d PATH` | `--scan-dir PATH` | 额外扫描目录，可重复 |
| `-h` | `--help` | 显示帮助 |

```sh
./bin/pkg-owner -s chrome
./bin/pkg-owner -s python -j
./bin/pkg-owner -l -d ~/Tools
```

`--search` 忽略大小写和标点，支持子串匹配及相似拼写（SequenceMatcher ≥ 0.72）。名称、别名、路径均参与搜索；位置参数使用精确匹配。多个安装来源或版本可能同时出现。

要在任意目录直接使用一行命令，可在 `~/.zshrc` 添加（替换实际插件路径）：

```sh
export PATH="$HOME/Project/pkg-owner/bin:$PATH"
```

重新打开终端后使用 `pkg-owner --search chrome`。也可始终用完整路径调用，不需要安装到系统目录。

## 识别范围

| 来源 | 依据 |
| --- | --- |
| Homebrew Formula / Cask | 当前 PATH 中 brew 的已安装 JSON 记录，Formula 文件路径和 Cask 应用目标 |
| npm 全局包 | 当前 npm 的全局包记录及模块路径，可追溯符号链接命令 |
| pipx | pipx list JSON 安装记录 |
| uv tool | uv 工具目录中的 uv-receipt.toml 安装记录 |
| Cargo | cargo install --list 安装记录 |
| MacPorts | port installed 中的 active 记录 |
| App Store | 应用内 `_MASReceipt/receipt` 存在 |
| macOS | 系统应用或系统命令目录 |
| Unknown | 已找到软件，但未关联受支持的管理器记录 |

应用扫描包含 `/Applications`、`~/Applications`、`/System/Applications`、Spotlight 索引中的其他 `.app` 和 `--scan-dir` 指定目录；命令扫描包含 PATH 下的可执行文件，也支持显式文件路径。扫描不会进入 `.app` 内部递归列出嵌套应用。

## 证据边界

- 管理器记录表示当前登记状态，不能证明最后一次覆盖安装的来源。已登记但被手动移除的软件仍可能出现在管理器结果中。
- 未索引的其他目录需要 `--scan-dir`。不进行全磁盘扫描。
- 只查询当前 PATH/环境所选择的各管理器实例；不枚举所有 Conda、venv、Node 版本和第二套 Homebrew。项目本地依赖与普通 pip 库不属于本次清单范围。
- pipx、uv、Cargo、MacPorts 提供包名登记结果；未关联路径的独立命令可能另列为 Unknown。不会仅凭同名就断言文件所有权。
- PKG 安装收据不等于持续管理来源，本版不将历史 PKG 收据推断为包管理器；也不根据扩展属性推断“官网安装”。
- 管理器异常或超时以警告输出；其余扫描继续。单条外部命令超时为 40 秒。Spotlight 不可用时回退目录扫描。
- `--json` 提供 `results`、`warnings`、`managers_checked`；每项含名称、来源、证据、路径和别名。退出码：成功 0，无匹配 1，参数错误 2。

此目录也包含 Codex 插件清单和技能，可作为插件源码使用；尚未写入个人 marketplace 或安装到 Codex。命令行功能可以直接独立运行。
