# OP.GG build 覆盖率

单英雄 build 快照用于驱动 UI 里的符文、召唤师技能、技能加点和装备路径推荐。
当前客户端运行时不会请求 OP.GG；扩库流程仍然是手动打开公开页面，再导出本地 JSON。

## 查看覆盖率

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/build-coverage.js --top 10
```

只看某个位置：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/build-coverage.js --role support --top 5
```

输出 JSON：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/build-coverage.js --top 10 --json
```

## 当前含义

- `covered stat entries`：当前 OP.GG 榜单里已有 build 快照的英雄/位置数量。
- `exact-context covered`：build 快照的 patch、region、tier、queue 与当前榜单快照完全一致的数量。
- `missing stat entries`：榜单里还没有 build 快照的英雄/位置数量。
- `stale build snapshots`：英雄/位置仍在榜单中，但 build 快照上下文已经过期。
- `orphan build snapshots`：build 快照存在，但当前榜单里没有对应英雄/位置。

## 当前优先级示例

在 16.12 / global / emerald_plus / ranked_solo_duo 快照下，当前只有 Nautilus support 有 build 快照。
下一批优先补：

1. Senna adc
2. Rek'Sai jungle
3. Rammus jungle
4. Leona support
5. Syndra mid

这些 URL 可以直接由 `build-coverage.js` 输出，然后逐个打开 OP.GG 页面运行 `extract-opgg-champion-build.js`。
