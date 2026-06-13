# OP.GG 快照工具

这个目录用于把浏览器里已经打开的 OP.GG 公开英雄榜单导出、检查并导入成本地 JSON 快照。

它不会登录 OP.GG，不会绕过 WAF、验证码或访问控制，也不会在 LeagueAkari Plus 运行时自动请求 OP.GG。

## 1. 从浏览器导出英雄榜单

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

## 2. 检查并导入英雄榜单

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

## 3. 从浏览器导出单英雄方案

单英雄方案用于保存类似原版 League Akari 里的召唤师技能、符文、技能加点和装备路径。

1. 用浏览器打开 OP.GG 单英雄 build 页面，例如：

   ```text
   https://op.gg/lol/champions/nautilus/build/support
   ```

2. 确认页面已经显示符文、召唤师技能、技能加点和装备表格。
3. 打开浏览器开发者工具 Console。
4. 复制并运行：

   ```js
   // tools/opgg-exporter/extract-opgg-champion-build.js
   ```

5. 将剪贴板里的 JSON 临时保存到：

   ```text
   work/opgg-build-nautilus-support.json
   ```

## 4. 检查并导入单英雄方案

先 dry-run：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-build.js work/opgg-build-nautilus-support.json --dry-run
```

确认无错误后写入或更新 build 缓存：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-build.js work/opgg-build-nautilus-support.json --append
```

导入工具会检查：

- 英雄 key 是否能匹配本地 `champion-tags.v1.json`
- 符文主系、副系、核心符文是否存在
- 召唤师技能是否为两个技能一组
- 技能加点是否只包含 `Q/W/E/R`
- 装备 ID、胜率、选率、场次是否在合理范围内

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

## 单英雄方案字段

- `champion_key`
- `role`
- `runes`
- `summoner_spells`
- `skill_orders`
- `item_builds`

## Build 覆盖率工具

继续扩展单英雄方案库前，可以先生成缺失清单：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/build-coverage.js --top 10
```

只看某个位置：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/build-coverage.js --role support --top 5
```

输出机器可读 JSON：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/build-coverage.js --top 10 --json
```

这个工具只读取本地 `opgg-champion-stats.sample.json` 和 `opgg-champion-builds.sample.json`，不会请求 OP.GG。
它会输出当前覆盖率、版本上下文是否一致，以及下一批应手动打开的 OP.GG build 页面 URL。

## 从公开 HTML 解析 build

如果公开 OP.GG 页面能正常返回 HTML，可以先保存页面，再离线解析：

```powershell
$url = "https://op.gg/lol/champions/senna/build/adc"
$html = "work/opgg-html/senna-adc.html"
$json = "work/opgg-builds/senna-adc.json"
Invoke-WebRequest -Uri $url -UseBasicParsing -MaximumRedirection 3 | Select-Object -ExpandProperty Content | Set-Content -Path $html -Encoding UTF8
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/fetch-opgg-build.js $url --html $html --output $json
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/import-opgg-build.js $json --dry-run
```

直接请求也支持，但如果 OP.GG 返回 WAF challenge，工具会失败退出，不会尝试绕过：

```powershell
C:\Users\admin\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe tools/opgg-exporter/fetch-opgg-build.js https://op.gg/lol/champions/senna/build/adc --output work/opgg-builds/senna-adc.json
```
