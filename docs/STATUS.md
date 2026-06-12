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
- 样例英雄标签已拆到 `crates/leagueakari-probe/data/champion-tags.sample.json`。
- 项目级 Cargo 配置已切到 `rsproxy` 镜像源，改善国内依赖下载速度。
- 新增 `--watch` 实时监听模式，通过 LCU 本地 WebSocket 订阅 gameflow 和选人阶段变化。
- 新增敌我阵容对比分析字段，开始输出敌方威胁和本局取胜思路。
- 新增 `--json` 结构化事件输出，作为后续 Tauri UI 的数据通道。

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
```

当前单元测试数量：19。

## 需要回来后确认

- 用更多真实对局验证 `enemy_threats` 和 `win_conditions` 的解释是否符合“帮你理解这一局应该怎么赢”。
- 决定下一步先扩展完整英雄标签库，还是先搭 Tauri UI。

## 推荐下一步

优先搭 Tauri UI 骨架，同时继续扩展完整英雄标签库。

原因：实时数据流和第一版敌我分析已经接上，下一步需要把它变成你能看的客户端界面，同时继续补足阵容分析需要的英雄标签数据。
