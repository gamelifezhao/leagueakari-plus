# OP.GG 快照导出工具

这个工具用于把浏览器里已经打开的 OP.GG 公开英雄榜单导出成本地 JSON 快照。

它不会登录 OP.GG，不会绕过 WAF、验证码或访问控制，也不会在 LeagueAkari Plus 运行时自动请求 OP.GG。

## 使用方法

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

6. 将结果保存到：

   ```text
   crates/leagueakari-probe/data/opgg-champion-stats.sample.json
   ```

7. 运行验证：

   ```powershell
C:\Users\admin\.cargo\bin\cargo.exe test -p leagueakari-probe
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
