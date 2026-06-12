# OP.GG 快照工具

这个目录用于把浏览器里已经打开的 OP.GG 公开英雄榜单导出、检查并导入成本地 JSON 快照。

它不会登录 OP.GG，不会绕过 WAF、验证码或访问控制，也不会在 LeagueAkari Plus 运行时自动请求 OP.GG。

## 1. 从浏览器导出

1. 用浏览器打开 OP.GG 英雄榜单：

   ```text
   https://op.gg/lol/champions
   ```

2. 确认页面已经正常显示 Champion Tier List 表格。

3. 打开浏览器开发者工具 Console。

4. 粘贴并运行：

   ```js
   // tools/opgg-exporter/extract-opgg-champion-stats.js
   ```

   实际使用时请复制 `extract-opgg-champion-stats.js` 的完整内容。

5. 脚本会把 JSON 快照复制到剪贴板。

6. 将结果临时保存到本地文件，例如：

   ```text
   work/opgg-snapshot.json
   ```

## 2. 检查并导入

先 dry-run 看报告，不写入缓存：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-snapshot.js work/opgg-snapshot.json --dry-run
```

确认无错误后写入 probe 使用的缓存：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-snapshot.js work/opgg-snapshot.json
```

导入工具会检查：

- 快照条目数是否过少
- `champion_key + role` 是否重复
- 胜率、选率、禁率是否在 0 到 100 之间
- OP.GG 英雄 key 是否能匹配本地 `champion-tags.v1.json`

写入后再运行 Rust 侧自检：

```powershell
C:\Users\admin\.cargo\bin\cargo.exe run -p leagueakari-probe -- --validate-data
```

当前脚本默认导出 `ranked_solo_duo`。如果以后要支持 Ranked Flex 或其他筛选项，应先在 OP.GG 页面上手动切换，再在脚本里补充对应的 queue 识别逻辑。

## 快照字段

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
