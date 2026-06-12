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
```

运行探针：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-probe
```

默认输出会隐藏账号标识、token 和原始 session，只打印连接状态、当前阶段和标准化 `DraftState`。

进入选人阶段后，探针会从 LCU 本地 game-data 读取英雄摘要数据，把公开 pick 和 ban 的 `championId` 尽量转换成英雄名称。

探针也会输出一个本地规则模型的 `composition analysis`。当前规则库只覆盖少量样例英雄，用来验证分析管线，不能当成正式胜率预测。

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
5. 搭建 Tauri UI 骨架，把 `DraftState` 和 `composition analysis` 显示到桌面客户端里。
