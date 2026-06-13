# OP.GG 数据管线

## 目标

OP.GG 统计数据用于补充“当前版本环境”的解释，例如：

- 某个英雄当前版本选率很高
- 某个英雄禁用率很高，通常代表处理成本高
- 某个英雄高登场但胜率偏低，需要更关注熟练度和阵容配合
- 某个英雄当前版本常用召唤师技能、符文、技能加点和装备路径

这些数据只辅助解释，不直接承诺胜负，也不替代本地英雄机制标签。符文或方案应用必须保留用户确认步骤。

## 当前结论

2026-06-12 验证时，命令行直接请求 OP.GG 会被 CloudFront WAF challenge 拦截：

```text
HTTP/1.1 202 Accepted
x-amzn-waf-action: challenge
Content-Length: 0
```

浏览器正常访问 `https://op.gg/lol/champions` 可以看到公开页面数据，包括：

- Patch 16.12
- Global
- Emerald+
- Ranked Solo/Duo
- 总样本量
- Champion / Role / Win rate / Pick rate / Ban rate

因此当前不在 app 运行时直接爬 OP.GG，也不绕过 WAF challenge。推荐方式是：

1. 用浏览器或人工流程查看公开页面。
2. 使用 `tools/opgg-exporter/extract-opgg-champion-stats.js` 从已打开页面导出 JSON。
3. 使用 `tools/opgg-exporter/import-opgg-snapshot.js` 检查并导入到 `crates/leagueakari-probe/data/opgg-champion-stats.sample.json`。
4. Rust 分析模块从本地 JSON 缓存读取。
5. 缓存过期时手动或半自动刷新。
6. 更新缓存后运行 `leagueakari-probe --validate-data` 检查字段、重复项和标签匹配。

单英雄 build 页面的流程类似：

1. 用浏览器打开 `https://op.gg/lol/champions/<champion>/build/<role>`。
2. 使用 `tools/opgg-exporter/extract-opgg-champion-build.js` 从已打开页面导出 JSON。
3. 使用 `tools/opgg-exporter/import-opgg-build.js` 检查并导入到 `crates/leagueakari-probe/data/opgg-champion-builds.sample.json`。
4. 后续 UI 只读取本地 build 缓存，并在应用符文前要求用户确认。

## 数据分层

`champion-tags.v1.json`

本地机制标签库，维护英雄的阵容解释属性：

- `engage`
- `frontline`
- `magic_damage`
- `physical_damage`
- `crowd_control`
- `scaling`
- `roles`
- `archetypes`

`opgg-champion-stats.sample.json`

OP.GG 公开页面快照，维护版本统计属性：

- `patch`
- `region`
- `tier`
- `queue`
- `sample_count`
- `champion_key`
- `role`
- `win_rate`
- `pick_rate`
- `ban_rate`
- `rank`

`opgg-champion-builds.sample.json`

OP.GG 单英雄 build 页面快照，维护方案属性：

- `champion_key`
- `role`
- `runes`
- `summoner_spells`
- `skill_orders`
- `item_builds`

## 刷新命令

导入前先 dry-run：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-snapshot.js work/opgg-snapshot.json --dry-run
```

确认报告无错误后写入缓存：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-snapshot.js work/opgg-snapshot.json
```

最后运行 Rust 侧数据自检：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-probe -- --validate-data
```

单英雄 build 导入前先 dry-run：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-build.js work/opgg-build-nautilus-support.json --dry-run
```

确认报告无错误后写入或更新 build 缓存：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-build.js work/opgg-build-nautilus-support.json --append
```

## 使用边界

- 不登录 OP.GG。
- 不绕过 WAF、验证码或访问控制。
- 不把网页结构硬编码进运行时核心流程。
- 不把 OP.GG 胜率当作精确预测，只作为解释上下文。
- 不自动应用符文、装备或任何客户端写入操作；应用前必须由用户确认。
