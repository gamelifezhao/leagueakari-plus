# 当前状态

更新日期：2026-06-12

## 已完成

- 创建 Rust workspace。
- 新增 `leagueakari-probe` 命令行探针。
- 支持标准 LCU `lockfile` 连接发现。
- 支持国服 / WeGame 通过 `LeagueClientUx*.log` 发现 LCU 端口和 token。
- 默认隐藏账号标识、token 和原始 session。
- 支持 `--raw` 参数打印原始 LCU JSON 方便调试。
- 成功验证国服客户端能读取当前召唤师和 `ChampSelect` 阶段。
- 能解析双方公开 pick 和 ban。
- 能从 LCU 本地 game-data 读取英雄摘要，将 `championId` 转为英雄名称。
- 新增本地规则模型骨架，输出基础 `composition analysis`。
- 英雄机制标签已升级到 `crates/leagueakari-probe/data/champion-tags.v1.json`。
- 项目级 Cargo 配置已切到 `rsproxy` 镜像源，改善国内依赖下载速度。
- 新增 `--watch` 实时监听模式，通过 LCU 本地 WebSocket 订阅 gameflow 和选人阶段变化。
- 新增敌我阵容对比分析字段，开始输出敌方威胁和本局取胜思路。
- 新增 `--json` 结构化事件输出，作为后续 Tauri UI 的数据通道。
- 新增零依赖 UI 原型，用样例 JSON 事件展示连接状态、双方阵容和阵容分析。
- 新增最小 Tauri 桌面壳，直接嵌入当前静态 UI 原型。
- 新增 Tauri 到 probe 的 JSON 事件桥接：前端准备好后启动只读 probe，并接收实时 LCU 事件。
- 桌面壳支持 `LEAGUEAKARI_PROBE_PATH` 覆盖 probe 路径，并会在 probe 异常退出时把最后一条错误显示到 UI。
- 已确认 `leagueakari-app` 开发构建可生成 `target\debug\leagueakari-app.exe`。
- 新增 `champion-tags.v1.json` 英雄机制标签库，覆盖真实 BP 样例和一批 OP.GG 当前榜单英雄。
- 新增 OP.GG 公开页面统计快照缓存，并把胜率、选率、禁率作为阵容解释的辅助信息。
- 新增 `tools/opgg-exporter`，用于从已打开的 OP.GG 公开页面导出本地统计快照。
- 新增 `leagueakari-probe --validate-data`，可在不启动英雄联盟客户端的情况下检查英雄标签和 OP.GG 快照。
- 新增 `tools/opgg-exporter/import-opgg-snapshot.js`，刷新 OP.GG 快照前可先做字段、重复项和标签覆盖检查。
- 新增 OP.GG 单英雄 build 快照样例与导入工具，开始覆盖召唤师技能、符文、技能加点和装备路径。

## 安全边界

当前实现只读 LCU 本地接口，不做：

- 游戏内自动操作
- 内存读取或内存修改
- 反作弊绕过
- 自动秒退
- 自动写符文
- 读取客户端不可见信息

`--watch` 也只订阅本地事件，不会向客户端写入配置或执行游戏操作。

## 已验证命令

```powershell
C:\Users\admin\.cargo\bin\cargo.exe fmt --all --check
C:\Users\admin\.cargo\bin\cargo.exe test -p leagueakari-probe
C:\Users\admin\.cargo\bin\cargo.exe check -p leagueakari-probe
C:\Users\admin\.cargo\bin\cargo.exe test -p leagueakari-app
C:\Users\admin\.cargo\bin\cargo.exe check -p leagueakari-app
C:\Users\admin\.cargo\bin\cargo.exe build -p leagueakari-app
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-probe -- --validate-data
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check tools/opgg-exporter/import-opgg-snapshot.js
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-snapshot.js crates/leagueakari-probe/data/opgg-champion-stats.sample.json --dry-run
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check tools/opgg-exporter/extract-opgg-champion-build.js
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check tools/opgg-exporter/import-opgg-build.js
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-build.js crates/leagueakari-probe/data/opgg-champion-builds.sample.json --dry-run
```

当前单元测试数量：`leagueakari-probe` 21 个，`leagueakari-app` 3 个。

## 需要回来后确认

- 用更多真实对局验证 `enemy_threats` 和 `win_conditions` 的解释是否符合“帮你理解这一局应该怎么赢”。
- 决定下一步先扩展完整英雄标签库，还是先搭 Tauri UI。

## 推荐下一步

优先在真实选人阶段验证 `leagueakari-app` 的实时 UI，同时继续扩展完整英雄标签库。

原因：Tauri 壳已经能启动 probe 并转发事件，下一步要在真实 BP 流程里验证 UI 刷新体验，同时继续补足阵容分析需要的英雄标签数据。
