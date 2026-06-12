# 开发说明

## 当前阶段

当前实现的是 M0：`leagueakari-probe`。

它是一个 Rust 命令行探针，用来验证 League Client / LCU 本地接口能否稳定读取：

- 客户端 `lockfile`
- 国服 / WeGame 的 `LeagueClientUx*.log` 启动参数
- 当前召唤师资料
- 当前游戏阶段
- 选人阶段 session
- 标准化后的 `DraftState`

这个阶段只读取客户端公开给本地应用的数据，不做游戏内自动操作，不读取内存，不绕过反作弊。

## 环境

本机已安装：

- Rust stable MSVC toolchain
- Visual Studio 2022 Build Tools C++ 组件
- `rustfmt`

仓库内 `.cargo/config.toml` 配置了 Cargo 镜像源，用于改善依赖下载速度。

## 常用命令

格式检查：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe fmt --all --check
```

单元测试：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe test -p leagueakari-probe
```

编译检查：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe check -p leagueakari-probe
C:\Users\admin\.cargo\bin\cargo.exe check -p leagueakari-app
```

运行探针：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-probe
```

默认输出会隐藏账号标识、token 和原始 session，只打印连接状态、当前阶段和标准化 `DraftState`。

进入选人阶段后，探针会从 LCU 本地 game-data 读取英雄摘要数据，把公开 pick 和 ban 的 `championId` 尽量转换成英雄名称。

探针也会输出一个本地规则模型的 `composition analysis`。当前规则库只覆盖少量样例英雄，用来验证分析管线，不能当成正式胜率预测。

当前 `composition analysis` 会同时输出我方维度、敌方维度、敌方威胁和本局取胜思路。它的目标不是告诉你“必胜/必输”，而是把这一局的主要风险、强点和打法重点说清楚。

样例英雄标签位于 `crates/leagueakari-probe/data/champion-tags.sample.json`，后续会扩展成完整英雄标签库。

如果需要调试原始 LCU JSON，可以显式加 `--raw`：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-probe -- --raw
```

如果需要实时监听选人变化，可以显式加 `--watch`：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-probe -- --watch
```

`--watch` 只订阅 LCU 本地 WebSocket 事件，不会写入客户端配置。

如果需要给未来 Tauri UI 或其他前端进程消费，可以加 `--json`，输出会变成一行一个 JSON 事件：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-probe -- --watch --json
```

结构化事件会隐藏 token/password、puuid、summonerId 等账号字段，只暴露连接状态、gameflow、标准化 `DraftState` 和阵容分析结果。

后续 UI 进程建议直接启动 `target\debug\leagueakari-probe.exe --watch --json`，这样 stdout 会是纯 JSON 行；如果用 `cargo run`，请加 `--quiet`，避免 Cargo 编译日志混入输出。

Tauri 桌面壳会在前端准备好后启动同目录下的 `leagueakari-probe --watch --json`，并把每一行 JSON 事件转发给前端。开发时请先确保 probe 已经编译过：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe build -p leagueakari-probe
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-app
```

开发构建生成的桌面程序位于：

```text
target\debug\leagueakari-app.exe
```

默认情况下，桌面壳会优先查找同目录下的 `leagueakari-probe.exe`。如果需要临时指定另一个探针路径，可以使用环境变量：

```powershell
$env:LEAGUEAKARI_PROBE_PATH="C:\Users\admin\Documents\Codex\2026-06-11\leagueakari\target\debug\leagueakari-probe.exe"
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-app
```

如果 probe 因为客户端未启动、LCU 超时或其他原因退出，桌面 UI 会显示最后一条 probe 错误，方便判断是客户端状态问题还是桥接启动问题。

如果 LCU 短时间内没有返回当前召唤师资料，探针会继续使用已经可达的 gameflow 和 WebSocket 数据，不会因为召唤师摘要超时而退出。

如果英雄联盟客户端没有启动，预期输出是：

```text
Error: League Client lockfile was not found. Start the League client, then run the probe again.
```

如果客户端已启动，探针会读取 `lockfile`，隐藏 token/password，并请求：

- `/lol-summoner/v1/current-summoner`
- `/lol-gameflow/v1/gameflow-phase`
- `/lol-champ-select/v1/session`，仅在 `ChampSelect` 阶段读取

## 国服 / WeGame 兼容

国服客户端可能出现传统 `LeagueClient:pid:port:password:protocol` 格式的 `lockfile` 为空文件。

这种情况下，探针会继续从最新的 `LeagueClientUx*.log` 里读取：

- `--app-port`
- `--remoting-auth-token`

探针只会在控制台显示端口和来源路径，不会打印 token。

探针会从最新的少量日志里提取候选端口，并逐个做短超时连接测试。旧端口不可用时会自动跳过，避免在历史日志里长时间等待。

当前已验证的国服路径示例：

```text
F:\WeGameApps\英雄联盟\LeagueClient
```

## 下一步

1. 启动英雄联盟客户端后运行 `leagueakari-probe`。
2. 确认能读到当前召唤师和 gameflow。
3. 进入自定义房间或正常选人阶段，确认能读到 champ select session。
4. 使用 `--watch` 做一次实机验证，确认选人变化能稳定推送。
5. 打开 `apps/leagueakari-ui-prototype/index.html` 查看零依赖 UI 原型。
6. 运行 `leagueakari-app` 桌面壳，确认静态 UI 能在 Tauri WebView 中打开。
7. 在真实选人阶段运行 `leagueakari-app`，确认前端能消费实时 `DraftState`、敌我阵容对比和 `composition analysis`。
