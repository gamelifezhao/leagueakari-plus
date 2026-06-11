# Rust / Tauri 技术规划

## 目标

LeagueAkari Plus 的第一版建议采用：

```text
Tauri + Rust + Web 前端
```

Rust 负责客户端连接、LCU 数据读取、WebSocket 实时监听、阵容分析、数据缓存和本地安全边界。前端负责界面展示、交互、图表和用户确认流程。

这个选择适合长期维护：核心逻辑轻、快、稳定，打包体积比 Electron 更小，也更适合做 Windows 桌面工具。

## 可实现性结论

使用 Rust 实现 MVP 是可行的，难点不在 Rust 语言本身，而在 LCU 细节和国服兼容。

高可行功能：

- 自动发现 League 客户端 `lockfile`
- 读取 LCU 端口、token、协议
- 连接本地 LCU HTTPS 接口
- 获取当前召唤师资料
- 监听游戏阶段变化
- 进入选人阶段后实时读取双方已公开英雄
- 读取 ban 位、pick 动作、倒计时
- 将选人状态推送给前端
- 基于规则模型做阵容评分和解释

需要谨慎处理：

- 国服客户端路径和 WeGame 环境
- LCU 自签证书
- WebSocket 断线重连
- 敌方位置推断不确定
- LCU 字段变化导致的兼容问题

不做：

- 游戏内自动操作
- 内存读取或内存修改
- 反作弊绕过
- 获取客户端不可见的信息

## 外部依据和约束

- Tauri 官方文档定位就是用 Web 前端构建桌面体验，同时把 Rust 用作后端逻辑，适合 LeagueAkari Plus 这种“本地客户端 + 可视化界面”的形态。
- Riot 官方开发者文档明确提到 League Client API 是客户端本地通信的一部分，但第三方应用使用它不属于官方支持范围，也不保证完整文档、稳定性或变更通知。
- Riot Data Dragon 提供英雄、物品、召唤师技能、图标等静态数据和资源，可以作为英雄基础资料、头像和版本数据的主要来源。

参考：

- Tauri: <https://tauri.app/start/>
- Riot League Client API: <https://developer.riotgames.com/docs/lol#league-client-api>
- Riot Data Dragon: <https://developer.riotgames.com/docs/lol#data-dragon>

因此技术策略是：先验证 LCU，再搭 UI；先规则模型，再数据增强；所有不确定数据都显示可信度，不伪装成确定事实。

## 第一阶段技术验证：LCU Probe

在正式做 UI 之前，先实现一个最小 Rust 命令行探针。

目标：

```text
leagueakari-probe
```

功能：

- 自动寻找 `lockfile`
- 解析端口、token、protocol
- 请求 `/lol-summoner/v1/current-summoner`
- 请求 `/lol-gameflow/v1/gameflow-phase`
- 连接 LCU WebSocket
- 监听 `/lol-gameflow/v1/gameflow-phase`
- 监听 `/lol-champ-select/v1/session`
- 如果进入选人阶段，打印我方、敌方已公开英雄、ban 位和当前 action

成功标准：

- 客户端未启动时能给出清晰错误
- 客户端启动后能识别账号
- 进入选人阶段后能实时输出选人变化
- WebSocket 断开后能自动重连或给出明确状态

## LCU 连接设计

### lockfile

League 客户端启动后会在安装目录生成 `lockfile`。

常见格式：

```text
LeagueClient:pid:port:password:protocol
```

我们需要解析：

- `port`
- `password`
- `protocol`

认证使用 Basic Auth：

```text
username: riot
password: lockfile password
```

HTTP Header：

```text
Authorization: Basic base64("riot:{password}")
```

### HTTPS

LCU 使用本地自签证书，所以 Rust HTTP client 需要允许本地无效证书。

```rust
reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()?;
```

这个设置只用于 `127.0.0.1` 的 LCU 连接，不用于外部网络请求。

### WebSocket

选人阶段实时性通过 LCU WebSocket 完成。

需要监听：

- `/lol-gameflow/v1/gameflow-phase`
- `/lol-champ-select/v1/session`

WebSocket 事件进入后，统一转换为内部状态：

```text
LcuEvent -> DraftState -> CompositionAnalysis -> FrontendEvent
```

## Rust 模块拆分

建议目录：

```text
src-tauri/
  src/
    main.rs
    commands.rs
    lcu/
      mod.rs
      lockfile.rs
      auth.rs
      client.rs
      websocket.rs
      gameflow.rs
      champ_select.rs
      models.rs
    analysis/
      mod.rs
      champion_tags.rs
      composition.rs
      recommendation.rs
      confidence.rs
    data/
      mod.rs
      champions.rs
      cache.rs
      patches.rs
    app_state.rs
    error.rs
```

职责：

- `lcu/lockfile.rs`：发现和解析 lockfile
- `lcu/client.rs`：REST 请求封装
- `lcu/websocket.rs`：WebSocket 连接和事件订阅
- `lcu/champ_select.rs`：解析选人 session
- `analysis/composition.rs`：阵容评分
- `analysis/recommendation.rs`：根据阵容短板和英雄池给建议
- `data/champions.rs`：英雄 ID、名称、头像、位置
- `commands.rs`：暴露给前端的 Tauri command

## 推荐依赖

Rust：

```toml
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
base64 = "0.22"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tauri = "2"
```

前端：

```text
Vue 或 React 均可
```

建议优先：

```text
Tauri + React + TypeScript + Vite
```

原因：

- 生态成熟
- 图表和状态管理选择多
- 和 Figma UI 转前端更顺

## 前端数据模型

Rust 后端推给前端的状态应该尽量干净。

示例：

```ts
type DraftState = {
  connected: boolean
  gameflow: 'None' | 'Lobby' | 'Matchmaking' | 'ChampSelect' | 'InProgress' | 'EndOfGame'
  localPlayerCellId?: number
  myTeam: DraftPlayer[]
  theirTeam: DraftPlayer[]
  bans: DraftBan[]
  timer?: DraftTimer
  analysis?: CompositionAnalysis
}
```

```ts
type CompositionAnalysis = {
  winRateRange: [number, number]
  confidence: 'low' | 'medium' | 'high'
  strengths: string[]
  risks: string[]
  suggestions: string[]
  dimensions: {
    engage: number
    frontline: number
    magicDamage: number
    physicalDamage: number
    scaling: number
    crowdControl: number
    waveClear: number
    objectiveSpeed: number
  }
}
```

## 阵容分析第一版

第一版不要做机器学习，先做规则模型。

原因：

- 可解释
- 快速落地
- 容易调参
- 不需要大量真实对局数据

每个英雄维护一组标签：

```text
frontline
engage
peel
burst
poke
wave_clear
scaling
magic_damage
physical_damage
crowd_control
side_lane
objective_speed
execution_difficulty
```

分析输出必须解释原因，而不是只给数字。

## 开发里程碑

### M0：Rust LCU Probe

- 创建 Rust workspace
- 实现 lockfile 发现
- 实现 LCU REST 请求
- 实现 WebSocket 事件监听
- 控制台输出选人阶段 JSON

### M1：Tauri 壳

- 创建 Tauri 应用
- 前端显示连接状态
- 前端显示当前召唤师
- 前端显示 gameflow

### M2：选人实时 UI

- 显示我方英雄
- 显示敌方已公开英雄
- 显示 ban 位
- 显示倒计时
- 处理未知位置和未选择状态

### M3：阵容分析

- 建立英雄标签库
- 输出胜率区间
- 输出可信度
- 输出优势、风险和建议

### M4：符文确认

- 展示推荐符文
- 明确列出将修改的内容
- 用户点击确认后才写入 LCU

### M5：国服兼容和打包

- 增加国服路径探测
- 增加 WeGame 场景测试
- 增加错误提示
- 生成 Windows 安装包

## 风险和应对

### LCU 字段变化

应对：

- JSON 解析字段使用 `Option`
- 保留原始 payload 日志
- 对未知字段保持兼容

### 客户端未启动或 lockfile 丢失

应对：

- UI 显示“等待客户端启动”
- 定时重新扫描
- 提供手动选择客户端目录

### 敌方位置不确定

应对：

- 显示可信度
- 允许手动修正
- 不把推断结果伪装成确定事实

### 自签证书

应对：

- 仅对 `127.0.0.1` LCU client 放宽证书校验
- 外部 API 不共享这个 client

## 推荐下一步

下一步直接开始 M0：

```text
创建 Rust workspace
实现 leagueakari-probe
跑通当前召唤师和选人 session 读取
```

M0 成功后，再搭 Tauri UI。这样能最快验证项目最核心的技术风险。

## 立即开工顺序

1. 检查开发环境：Rust、Node.js、pnpm 或 npm、WebView2、Tauri CLI。
2. 创建 Rust workspace，先不要直接生成完整 Tauri 项目。
3. 实现 `leagueakari-probe`，只做 LCU 探测和日志输出。
4. 用真实客户端测试三种状态：客户端未启动、客户端大厅、选人阶段。
5. 把稳定的数据结构沉淀成 `DraftState` 和 `CompositionAnalysis`。
6. 再创建 Tauri + React + TypeScript 前端，把 Probe 能力接到 UI。
7. 最后接阵容分析规则和符文确认流程。
