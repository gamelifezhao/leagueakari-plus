# OP.GG 数据管线

## 目标

OP.GG 统计数据用于补充“当前版本环境”的解释，例如：

- 某个英雄当前版本选率很高
- 某个英雄禁用率很高，通常代表处理成本高
- 某个英雄高登场但胜率偏低，需要更关注熟练度和阵容配合

这些数据只辅助解释，不直接承诺胜负，也不替代本地英雄机制标签。

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
2. 导出/整理成 `crates/leagueakari-probe/data/opgg-champion-stats.sample.json`。
3. Rust 分析模块从本地 JSON 缓存读取。
4. 缓存过期时手动或半自动刷新。

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

## 使用边界

- 不登录 OP.GG。
- 不绕过 WAF、验证码或访问控制。
- 不把网页结构硬编码进运行时核心流程。
- 不把 OP.GG 胜率当作精确预测，只作为解释上下文。
