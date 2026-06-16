const championNames = {
  11: "无极剑圣",
  17: "迅捷斥候",
  22: "寒冰射手",
  25: "堕落天使",
  54: "熔岩巨兽",
  56: "永恒梦魇",
  63: "复仇焰魂",
  83: "牧魂人",
  86: "德玛西亚之力",
  89: "曙光女神",
  90: "虚空先知",
  104: "法外狂徒",
  105: "潮汐海灵",
  111: "深海泰坦",
  117: "仙灵女巫",
  266: "暗裔剑魔",
  517: "解脱者",
  901: "炽炎雏龙",
  950: "百裂冥犬"
};

const metricLabels = {
  engage: "开团能力",
  magic_damage: "AP 伤害",
  scaling: "后期成长",
  crowd_control: "控制链",
  frontline: "容错率",
  physical_damage: "AD 输出"
};

const state = {
  currentView: "draft",
  connection: null,
  summoner: null,
  phase: "Unknown",
  snapshot: null,
  lastDraftSnapshot: null,
  draftFingerprint: null,
  bridgeStatus: null,
  watchStatus: null,
  champSelectStatus: null,
  usingLiveData: false,
  detailItems: {
    ally: [],
    enemy: [],
    teammates: []
  },
  matchHistory: null,
  matchHistoryStatus: "idle",
  matchHistoryError: null
};

const elements = {
  navItems: document.querySelectorAll("[data-view]"),
  draftView: document.querySelector("#draftView"),
  historyView: document.querySelector("#historyView"),
  placeholderView: document.querySelector("#placeholderView"),
  placeholderTitle: document.querySelector("#placeholderTitle"),
  placeholderText: document.querySelector("#placeholderText"),
  connectionStatus: document.querySelector("#connectionStatus"),
  serverStatus: document.querySelector("#serverStatus"),
  connectionDetail: document.querySelector("#connectionDetail"),
  gameflowPhase: document.querySelector("#gameflowPhase"),
  confidence: document.querySelector("#confidence"),
  snapshotSource: document.querySelector("#snapshotSource"),
  winRange: document.querySelector("#winRange"),
  winRangeNote: document.querySelector("#winRangeNote"),
  engageDelta: document.querySelector("#engageDelta"),
  damageDelta: document.querySelector("#damageDelta"),
  currentAdvice: document.querySelector("#currentAdvice"),
  teammateSummary: document.querySelector("#teammateSummary"),
  teammateOverview: document.querySelector("#teammateOverview"),
  teammatePerformance: document.querySelector("#teammatePerformance"),
  myPicks: document.querySelector("#myPicks"),
  enemyPicks: document.querySelector("#enemyPicks"),
  myBans: document.querySelector("#myBans"),
  enemyBans: document.querySelector("#enemyBans"),
  myPickCount: document.querySelector("#myPickCount"),
  enemyPickCount: document.querySelector("#enemyPickCount"),
  dimensionCompare: document.querySelector("#dimensionCompare"),
  keyReasons: document.querySelector("#keyReasons"),
  enemyAnalysis: document.querySelector("#enemyAnalysis"),
  heroRecommendations: document.querySelector("#heroRecommendations"),
  buildSourceNote: document.querySelector("#buildSourceNote"),
  buildRecommendation: document.querySelector("#buildRecommendation"),
  reconnectButton: document.querySelector("#reconnectButton"),
  loadSampleButton: document.querySelector("#loadSampleButton"),
  refreshMatchesButton: document.querySelector("#refreshMatchesButton"),
  historyStatus: document.querySelector("#historyStatus"),
  historySummary: document.querySelector("#historySummary"),
  favoriteChampions: document.querySelector("#favoriteChampions"),
  matchHistoryList: document.querySelector("#matchHistoryList"),
  detailModal: document.querySelector("#detailModal"),
  detailModalTitle: document.querySelector("#detailModalTitle"),
  detailModalSubtitle: document.querySelector("#detailModalSubtitle"),
  detailModalBody: document.querySelector("#detailModalBody"),
  detailModalClose: document.querySelector("#detailModalClose")
};

function applyEvent(message) {
  if (!message?.event) {
    return;
  }

  if (message.event === "probe_bridge_status") {
    state.bridgeStatus = message.payload;
    if (isDisconnectedBridgeStatus(message.payload)) {
      clearLiveConnection();
    }
  }

  if (message.event === "lcu_connection") {
    state.connection = message.payload;
  }

  if (message.event === "summoner_summary") {
    state.summoner = message.payload;
  }

  if (message.event === "gameflow_phase") {
    applyPhase(message.payload.phase);
  }

  if (message.event === "watch_gameflow") {
    applyPhase(message.payload.phase);
  }

  if (message.event === "watch_status") {
    state.watchStatus = message.payload;
    if (message.payload?.status === "closed" || message.payload?.status === "stopped") {
      clearLiveConnection();
    }
  }

  if (message.event === "champ_select_status") {
    state.champSelectStatus = message.payload;
  }

  if (message.event === "probe_status") {
    state.watchStatus = message.payload;
    if (message.payload?.status === "finished") {
      clearLiveConnection();
    }
  }

  if (message.event === "draft_snapshot") {
    cacheChampionNames(message.payload?.champion_names);
    if (isEmptyDeletedDraftSnapshot(message.payload) && state.lastDraftSnapshot) {
      state.snapshot = state.lastDraftSnapshot;
      state.champSelectStatus = null;
      render();
      return;
    }
    const draftPhase = message.payload?.draft_state?.gameflow ?? state.phase;
    resetSnapshotsForNewDraft(message.payload);
    applyPhase(draftPhase);
    if (shouldClearDraftForPhase(draftPhase)) {
      clearDraftSnapshots();
    } else if (isUsableDraftSnapshot(message.payload)) {
      state.lastDraftSnapshot = message.payload;
      state.snapshot = message.payload;
    } else if (hasTeammatePerformance(message.payload) && state.lastDraftSnapshot) {
      mergeTeammatePerformance(message.payload);
    } else if (!state.lastDraftSnapshot) {
      state.snapshot = message.payload;
    }
    state.champSelectStatus = null;
  }

  render();
}

function render() {
  const snapshot = displaySnapshot();
  const draft = snapshot?.draft_state;
  const analysis = snapshot?.analysis;
  const myDimensions = analysis?.dimensions ?? {};
  const enemyDimensions = analysis?.enemy_dimensions ?? {};
  const winScore = estimateWinScore(myDimensions, enemyDimensions, analysis?.confidence);
  const hasLiveConnection = state.usingLiveData && state.connection && !isBridgeError(state.bridgeStatus);
  const hasSampleConnection = !state.usingLiveData && state.connection;
  const connectionCopy = connectionDisplay();

  elements.connectionStatus.textContent = connectionCopy.title;
  elements.serverStatus.textContent = state.connection
    ? hasLiveConnection
      ? liveConnectionSubtitle()
      : bridgeLabel(state.bridgeStatus)
    : bridgeLabel(state.bridgeStatus);
  elements.connectionDetail.textContent = connectionCopy.detail;
  elements.gameflowPhase.textContent = phaseLabel(state.phase);
  elements.confidence.textContent = confidenceLabel(analysis?.confidence);
  elements.snapshotSource.textContent = snapshot
    ? `${snapshot.source} ${snapshot.lcu_event_type ?? ""}`.trim()
    : waitingSnapshotText();

  elements.winRange.textContent = winScore.range;
  elements.winRangeNote.textContent = winScore.note;
  elements.engageDelta.textContent = formatDelta("开团", myDimensions.engage, enemyDimensions.engage);
  elements.damageDelta.textContent = formatDelta(
    "伤害",
    Math.max(myDimensions.magic_damage ?? 0, myDimensions.physical_damage ?? 0),
    Math.max(enemyDimensions.magic_damage ?? 0, enemyDimensions.physical_damage ?? 0)
  );

  renderPicks(elements.myPicks, draft?.my_team ?? [], "ally");
  renderPicks(elements.enemyPicks, draft?.their_team ?? [], "enemy");
  renderBans(elements.myBans, draft?.bans ?? [], 100, snapshot);
  renderBans(elements.enemyBans, draft?.bans ?? [], 200, snapshot);
  renderDimensions(myDimensions, enemyDimensions);
  renderCurrentAdvice(analysis, draft);
  renderTeammatePerformance(snapshot?.teammate_performance ?? [], draft);
  renderReasons(analysis);
  renderEnemyAnalysis(analysis, draft);
  renderRecommendations(analysis);
  renderBuildRecommendation(analysis);
  renderCurrentView();
  renderMatchHistory();

  elements.myPickCount.textContent = `${countPicked(draft?.my_team)} / 5`;
  elements.enemyPickCount.textContent = `${countPicked(draft?.their_team)} / 5`;
}

function renderCurrentView() {
  const placeholderViews = new Set(["account", "champions", "settings"]);
  elements.draftView?.classList.toggle("hidden", state.currentView !== "draft");
  elements.historyView?.classList.toggle("hidden", state.currentView !== "history");
  elements.placeholderView?.classList.toggle("hidden", !placeholderViews.has(state.currentView));

  elements.navItems.forEach((item) => {
    item.classList.toggle("active", item.dataset.view === state.currentView);
  });

  if (placeholderViews.has(state.currentView)) {
    const copy = {
      account: ["账号概览", "这块之后再做，当前先完成近期战绩。"],
      champions: ["英雄池", "英雄池会继续沿用本地标签库和 OP.GG 快照。"],
      settings: ["设置", "设置页后续用于管理数据源、模型和显示偏好。"]
    }[state.currentView];
    elements.placeholderTitle.textContent = copy[0];
    elements.placeholderText.textContent = copy[1];
  }
}

function renderMatchHistory() {
  if (!elements.matchHistoryList || !elements.historySummary || !elements.historyStatus) {
    return;
  }

  elements.historyStatus.textContent = matchHistoryStatusText();

  if (state.matchHistoryStatus === "loading") {
    renderHistoryEmpty("正在读取最近 20 场对局。");
    return;
  }

  if (state.matchHistoryError) {
    renderHistoryEmpty(state.matchHistoryError);
    return;
  }

  const history = state.matchHistory;
  if (!history?.matches?.length) {
    renderHistoryEmpty("暂无最近战绩数据。进入客户端后可重新刷新。");
    return;
  }

  renderHistorySummary(history.summary);
  renderFavoriteChampions(history.summary?.favorite_champions ?? []);
  elements.matchHistoryList.replaceChildren(...history.matches.map((match) => matchHistoryCard(match)));
}

function renderHistoryEmpty(message) {
  elements.historySummary.replaceChildren(historyMetric("状态", message));
  elements.favoriteChampions.replaceChildren(emptyInline("暂无数据"));
  const empty = document.createElement("article");
  empty.className = "history-empty";
  empty.textContent = message;
  elements.matchHistoryList.replaceChildren(empty);
}

function matchHistoryStatusText() {
  if (state.matchHistoryStatus === "loading") {
    return "读取中";
  }
  if (state.matchHistoryError) {
    return "读取失败";
  }
  if (state.matchHistory?.matches?.length) {
    return `${state.matchHistory.matches.length} 场`;
  }
  return "等待读取";
}

function renderHistorySummary(summary = {}) {
  elements.historySummary.replaceChildren(
    historyMetric("Akari Score", akariScore(summary)),
    historyMetric("平均 KDA", Number(summary.avg_kda ?? 0).toFixed(2)),
    historyMetric("参团率", `${Number(summary.avg_kill_participation ?? 0).toFixed(0)}%`),
    historyMetric("伤害占比", `${Number(summary.avg_damage_share ?? 0).toFixed(0)}%`),
    historyMetric("活跃对局", `${summary.wins ?? 0} 胜 ${summary.losses ?? 0} 负 (${Number(summary.win_rate ?? 0).toFixed(0)}%)`)
  );
}

function renderFavoriteChampions(champions = []) {
  if (!champions.length) {
    elements.favoriteChampions.replaceChildren(emptyInline("暂无常用英雄"));
    return;
  }

  elements.favoriteChampions.replaceChildren(
    ...champions.map((champion) => {
      const item = document.createElement("div");
      item.className = "favorite-champion";
      item.append(
        championIcon(champion.champion_alias, champion.champion_name),
        inlineStack(champion.champion_name, `${champion.games} 场 · ${champion.wins} 胜`)
      );
      return item;
    })
  );
}

function historyMetric(label, value) {
  const row = document.createElement("div");
  row.className = "history-metric";
  const name = document.createElement("span");
  name.textContent = label;
  const amount = document.createElement("strong");
  amount.textContent = value;
  row.append(name, amount);
  return row;
}

function matchHistoryCard(match) {
  const card = document.createElement("article");
  card.className = `match-card ${match.result}`;
  card.append(
    matchChampionBlock(match),
    matchStatsBlock(match),
    matchLoadoutBlock(match),
    matchParticipantsBlock(match)
  );
  return card;
}

function matchChampionBlock(match) {
  const block = document.createElement("div");
  block.className = "match-champion";
  block.append(championIcon(match.champion_alias, match.champion_name));
  const text = inlineStack(match.champion_name, `${match.queue_label} · ${match.position} · ${durationLabel(match.duration_seconds)} · ${match.ended_at_label}`);
  block.append(text);
  return block;
}

function matchStatsBlock(match) {
  const block = document.createElement("div");
  block.className = "match-stats";
  const kda = document.createElement("strong");
  kda.textContent = `${match.kills} / ${match.deaths} / ${match.assists}`;
  const details = document.createElement("span");
  details.textContent = `KDA ${Number(match.kda).toFixed(2)} · 参团 ${Number(match.kill_participation).toFixed(0)}%`;
  const damage = document.createElement("span");
  damage.textContent = `${Number(match.total_damage).toLocaleString("zh-CN")} 伤害 · ${match.cs} 补刀`;
  block.append(kda, details, damage, tagRow(match.tags ?? []));
  return block;
}

function matchLoadoutBlock(match) {
  const block = document.createElement("div");
  block.className = "match-loadout";
  const items = document.createElement("div");
  items.className = "item-row";
  const itemIds = (match.items ?? []).slice(0, 7);
  const slots = Array.from({ length: 7 }, (_, index) => itemIds[index] ?? null);
  items.replaceChildren(...slots.map(itemIcon));
  block.append(items);
  return block;
}

function matchParticipantsBlock(match) {
  const block = document.createElement("div");
  block.className = "match-participants";
  (match.teams ?? []).slice(0, 2).forEach((team) => {
    const column = document.createElement("div");
    column.className = "participant-column";
    column.replaceChildren(...team.slice(0, 5).map(participantRow));
    block.append(column);
  });
  return block;
}

function participantRow(participant) {
  const row = document.createElement("span");
  row.className = participant.is_current_player ? "participant current" : "participant";
  row.append(championMiniIcon(participant.champion_alias, participant.champion_name));
  const name = document.createElement("span");
  name.textContent = shortName(participant.display_name);
  row.append(name);
  return row;
}

function tagRow(tags) {
  const row = document.createElement("div");
  row.className = "match-tags";
  row.replaceChildren(...tags.map((tag) => {
    const pill = document.createElement("span");
    pill.textContent = tag;
    return pill;
  }));
  return row;
}

function championIcon(alias, name) {
  const icon = document.createElement("div");
  icon.className = "champion-icon";
  if (alias) {
    const image = document.createElement("img");
    image.src = championIconUrl(alias);
    image.alt = name;
    image.loading = "lazy";
    icon.append(image);
  }
  const fallback = document.createElement("span");
  fallback.textContent = shortIconText(name);
  icon.append(fallback);
  return icon;
}

function championMiniIcon(alias, name) {
  const icon = championIcon(alias, name);
  icon.classList.add("mini");
  return icon;
}

function itemIcon(itemId) {
  const slot = document.createElement("span");
  slot.className = itemId ? "item-icon" : "item-icon empty";
  if (itemId) {
    const image = document.createElement("img");
    image.src = itemIconUrl(itemId);
    image.alt = `装备 ${itemId}`;
    image.loading = "lazy";
    slot.append(image);
  }
  return slot;
}

function inlineStack(title, subtitle) {
  const stack = document.createElement("div");
  stack.className = "inline-stack";
  const strong = document.createElement("strong");
  strong.textContent = title;
  const span = document.createElement("span");
  span.textContent = subtitle;
  stack.append(strong, span);
  return stack;
}

function emptyInline(text) {
  const span = document.createElement("span");
  span.className = "empty-inline";
  span.textContent = text;
  return span;
}

function akariScore(summary = {}) {
  const score = (Number(summary.avg_kda ?? 0) * 4)
    + Number(summary.avg_kill_participation ?? 0) * 0.08
    + Number(summary.avg_damage_share ?? 0) * 0.12
    + Number(summary.win_rate ?? 0) * 0.04;
  return score.toFixed(2);
}

function durationLabel(seconds) {
  const safeSeconds = Number(seconds || 0);
  const minutes = Math.floor(safeSeconds / 60);
  const rest = String(safeSeconds % 60).padStart(2, "0");
  return `${minutes}:${rest}`;
}

function shortIconText(name = "英") {
  return String(name || "英").slice(0, 1);
}

function championIconUrl(alias) {
  return `https://ddragon.leagueoflegends.com/cdn/16.12.1/img/champion/${alias}.png`;
}

function itemIconUrl(itemId) {
  return `https://ddragon.leagueoflegends.com/cdn/16.12.1/img/item/${itemId}.png`;
}

function renderPicks(container, players, side) {
  const slots = Array.from({ length: 5 }, (_, index) => players[index] ?? null);
  container.replaceChildren(
    ...slots.map((player) => {
      const championId = player?.champion_id;
      const item = document.createElement("article");
      item.className = `hero-card ${championId ? "picked" : "empty"}`;
      item.innerHTML = `
        <div class="hero-name">${championName(championId, false)}</div>
        <div class="hero-position">${positionLabel(player?.assigned_position)}</div>
      `;
      item.dataset.side = side;
      return item;
    })
  );
}

function renderBans(container, bans, teamId, snapshot) {
  if (isLiveClientRestoredSnapshot(snapshot) && bans.length === 0) {
    const message = document.createElement("span");
    message.className = "ban-chip unavailable";
    message.textContent = "游戏内恢复，Ban 不可用";
    container.replaceChildren(message);
    return;
  }

  const teamBans = bans.filter((ban) => ban.team_id === teamId).slice(0, 5);
  const slots = Array.from({ length: 5 }, (_, index) => teamBans[index] ?? null);

  container.replaceChildren(
    ...slots.map((ban) => {
      const chip = document.createElement("span");
      chip.className = `ban-chip ${ban ? "" : "empty"}`.trim();
      chip.textContent = ban ? championName(ban.champion_id, false) : "-";
      return chip;
    })
  );
}

function renderDimensions(myDimensions, enemyDimensions) {
  elements.dimensionCompare.replaceChildren(
    ...Object.entries(metricLabels).map(([key, label]) => {
      const myValue = myDimensions[key] ?? 0;
      const enemyValue = enemyDimensions[key] ?? 0;
      const verdict = dimensionVerdict(myValue, enemyValue);
      const row = document.createElement("div");
      row.className = "dimension-row";
      row.innerHTML = `
        <span>${label}</span>
        <div class="bar" aria-label="${label}">
          <div class="bar-fill" style="width: ${myValue}%"></div>
          <div class="bar-fill enemy" style="width: ${enemyValue}%"></div>
        </div>
        <strong class="${verdict.className}">${verdict.text}</strong>
      `;
      return row;
    })
  );
}

function renderTeammatePerformance(teammates = [], draft) {
  if (teammates.length === 0) {
    const hasDraftPicks = countDraftPicks(draft) > 0;
    elements.teammateSummary.textContent = hasDraftPicks
      ? "正在读取队友质量数据。"
      : "进入 BP 后开始分析队友质量。";
    elements.teammateOverview.replaceChildren(qualityBadge("等待", hasDraftPicks ? "读取中" : "BP 后"));
    elements.teammatePerformance.replaceChildren(
      emptyTeammateState(hasDraftPicks ? "正在读取队友最近战绩" : "等待 BP 队友名单")
    );
    state.detailItems.teammates = [];
    return;
  }

  const sortedTeammates = [...teammates].sort(teammateQualitySort);
  elements.teammateSummary.textContent = teammateQualitySummary(sortedTeammates);
  elements.teammateOverview.replaceChildren(...teammateQualityBadges(sortedTeammates));
  elements.teammatePerformance.replaceChildren(
    ...sortedTeammates.slice(0, 2).map((teammate) => teammateQualityCard(teammate))
  );
  state.detailItems.teammates = sortedTeammates.map((teammate) => ({
    title: `${shortName(teammate.display_name)} · ${teammate.tier_label ?? "数据少"}`,
    text: `${championName(teammate.champion_id, false)} · ${positionLabel(teammate.assigned_position)} · ${formatAverageKda(teammate)}`,
    type: teammate.tier === "workhorse" || teammate.tier === "low_horse" ? "negative" : "positive"
  }));
}

function renderCurrentAdvice(analysis, draft) {
  if (!elements.currentAdvice) {
    return;
  }
  if (!hasEnoughPicksForAdvice(draft)) {
    const picked = countDraftPicks(draft);
    renderAdviceLines([
      { text: `等待阵容继续成型，当前已识别 ${picked}/10 个英雄。` },
      { text: "先观察分路和关键控制位，暂不下判断。" }
    ]);
    return;
  }
  const suggestions = analysis?.suggestions ?? [];
  const winConditions = analysis?.win_conditions ?? [];
  const headline = winConditions[0] ?? suggestions[0] ?? "围绕阵容强势期打资源节奏。";
  renderAdviceLines([{ text: headline }, ...currentAdviceLines(analysis)]);
}

function currentAdviceLines(analysis) {
  const myDimensions = analysis?.dimensions ?? {};
  const enemyDimensions = analysis?.enemy_dimensions ?? {};
  const biggestGap = strongestDimensionGap(myDimensions, enemyDimensions);
  const lines = [];

  if (biggestGap && Math.abs(biggestGap.delta) > 8) {
    lines.push({
      text: `${biggestGap.delta < 0 ? "风险" : "优势"}：${metricLabels[biggestGap.key]}，${dimensionHint(
        biggestGap.key,
        biggestGap.mine,
        biggestGap.enemy
      )}`,
      tone: biggestGap.delta < 0 ? "danger" : "normal"
    });
  }

  const enemyDamage = enemyDamageProfile(enemyDimensions).replace(/^伤害结构：/, "");
  lines.push({ text: `伤害结构：${enemyDamage}`, tone: enemyDamage.includes("压力") || enemyDamage.includes("不能只堆") ? "danger" : "normal" });

  return lines.slice(0, 2);
}

function renderAdviceLines(lines) {
  elements.currentAdvice.replaceChildren(
    ...lines.map((line) => {
      const item = document.createElement("p");
      item.className = line.tone === "danger" ? "advice-line danger" : "advice-line";
      item.textContent = line.text;
      return item;
    })
  );
}

function teammateQualityCard(teammate) {
  const item = document.createElement("div");
  item.className = `teammate-card ${teammate.tier ?? "data_light"}`;

  const identity = document.createElement("div");
  identity.className = "teammate-card-identity";
  const name = document.createElement("strong");
  name.textContent = shortName(teammate.display_name);
  const role = document.createElement("span");
  role.textContent = `${championName(teammate.champion_id, false)} · ${positionLabel(teammate.assigned_position)}`;
  identity.append(name, role);

  const metrics = document.createElement("small");
  metrics.className = "teammate-card-metrics";
  metrics.textContent = formatAverageKda(teammate);

  const tier = document.createElement("b");
  tier.textContent = teammate.tier_label ?? "数据少";

  item.append(identity, metrics, tier);
  return item;
}

function emptyTeammateState(text = "暂无队友质量数据") {
  const item = document.createElement("div");
  item.className = "teammate-empty";
  item.textContent = text;
  return item;
}

function teammateQualityBadges(teammates) {
  const counts = teammateQualityCounts(teammates);
  return [
    qualityBadge("上等马", counts.top_horse),
    qualityBadge("普通", counts.stable),
    qualityBadge("下等马", counts.low_horse),
    qualityBadge("牛马", counts.workhorse)
  ];
}

function qualityBadge(label, value) {
  const badge = document.createElement("span");
  badge.className = "quality-badge";
  badge.textContent = `${label} ${value}`;
  return badge;
}

function renderReasons(analysis) {
  const draft = displaySnapshot()?.draft_state;
  if (!hasEnoughPicksForAdvice(draft)) {
    const waiting = [{ title: "等待阵容成型", text: "我方至少选出 3 个英雄后开始分析。", type: "suggestion" }];
    state.detailItems.ally = waiting;
    elements.keyReasons.replaceChildren(...waiting.map((reason) => reasonItem(reason.title, reason.text, reason.type)));
    return;
  }

  const strengths = (analysis?.strengths ?? []).slice(0, 2);
  const risks = (analysis?.risks ?? []).slice(0, 2);
  const suggestions = (analysis?.suggestions ?? []).slice(0, 1);
  const reasons = [
    ...strengths.map((text) => ({ title: "优势", text, type: "positive" })),
    ...risks.map((text) => ({ title: "风险", text, type: "negative" })),
    ...suggestions.map((text) => ({ title: "打法", text, type: "suggestion" }))
  ];

  if (reasons.length === 0) {
    reasons.push({ title: "等待阵容成型", text: "继续观察双方选人。", type: "suggestion" });
  }

  state.detailItems.ally = reasons;
  elements.keyReasons.replaceChildren(
    ...reasons.slice(0, 2).map((reason) => reasonItem(reason.title, reason.text, reason.type))
  );
}

function renderEnemyAnalysis(analysis, draft) {
  if (!hasEnoughEnemyPicksForAnalysis(draft)) {
    const waiting = [{ title: "等待敌方阵容", text: "敌方至少选出 2 个英雄后开始分析。", type: "suggestion" }];
    state.detailItems.enemy = waiting;
    elements.enemyAnalysis.replaceChildren(...waiting.map((item) => reasonItem(item.title, item.text, item.type)));
    return;
  }

  const enemyPlayers = draft?.their_team ?? [];
  const enemyPicks = enemyPlayers
    .filter((player) => player?.champion_id)
    .map((player) => championName(player.champion_id, false));
  const enemyDimensions = analysis?.enemy_dimensions ?? {};
  const enemyThreats = (analysis?.enemy_threats ?? []).slice(0, 2);
  const enemyFocusTargets = enemyFocusItems(analysis, enemyPlayers);
  const items = [];

  if (enemyPicks.length > 0) {
    items.push({
      type: "suggestion",
      title: "已识别",
      text: enemyPicks.join(" / ")
    });
  }

  if (analysis) {
    items.push(...enemyFocusTargets);

    items.push({
      type: enemyDamageWarningType(enemyDimensions),
      title: "伤害结构",
      text: enemyDamageProfile(enemyDimensions)
    });

    if ((enemyDimensions.engage ?? 0) >= 70) {
      items.push({ type: "negative", title: "开团压力", text: "对面第一波先手很关键，河道和野区入口别站太密。" });
    } else if ((enemyDimensions.engage ?? 0) <= 35) {
      items.push({ type: "positive", title: "开团不足", text: "可以主动逼资源，迫使对面先交关键技能。" });
    }

    if ((enemyDimensions.crowd_control ?? 0) >= 70) {
      items.push({ type: "negative", title: "控制链", text: "被第一段控制命中后容易连续吃技能。" });
    }

    if ((enemyDimensions.scaling ?? 0) >= 75) {
      items.push({ type: "negative", title: "后期强", text: "中期资源节奏要更主动，别拖到对面成型。" });
    }
  }

  const hasSpecificFocus = enemyFocusTargets.length > 0;
  enemyThreats
    .filter((text) => !(hasSpecificFocus && text.includes("前排")))
    .forEach((text) => {
    items.push({ type: "negative", title: "威胁点", text });
  });

  if (items.length === 0) {
    items.push({ type: "suggestion", title: "等待阵容成型", text: "继续观察敌方后续选人。" });
  }

  state.detailItems.enemy = items;
  elements.enemyAnalysis.replaceChildren(
    ...items.slice(0, 2).map((item) => {
      return reasonItem(item.title, item.text, item.type);
    })
  );
}

function enemyFocusItems(analysis, enemyPlayers = []) {
  const targets = analysis?.enemy_focus_targets ?? [];
  if (targets.length > 0) {
    return targets.slice(0, 4).map((target) => ({
      type: enemyFocusTypeTone(target.focus_type),
      title: enemyFocusTypeLabel(target.focus_type),
      text: `${championName(target.champion_id, false)}：${target.reason}`
    }));
  }

  const picked = enemyPlayers.filter((player) => player?.champion_id);
  const carries = picked.filter((player) => ["bottom", "middle"].includes(player.assigned_position));
  const frontline = picked.find((player) => ["top", "utility", "jungle"].includes(player.assigned_position));
  const items = [];

  if (carries.length > 0) {
    items.push({
      type: "negative",
      title: "优先限制",
      text: `${carries.map((player) => championName(player.champion_id, false)).join(" / ")}：优先压低输出环境，团前别让他们免费站位。`
    });
  }
  if (frontline) {
    items.push({
      type: "suggestion",
      title: "前排处理",
      text: `${championName(frontline.champion_id, false)}：如果很难秒掉，就绕开他找后排或等他关键技能交完。`
    });
  }

  return items;
}

function enemyFocusTypeLabel(type) {
  const labels = {
    backline_carry: "优先限制",
    frontline: "前排处理",
    engage: "开团点",
    crowd_control: "控制点",
    scaling: "后期点"
  };
  return labels[type] ?? "重点英雄";
}

function enemyFocusTypeTone(type) {
  return type === "frontline" ? "suggestion" : "negative";
}

function reasonItem(title, text, type) {
  const item = document.createElement("li");
  item.className = type;
  const titleNode = document.createElement("strong");
  titleNode.textContent = title;
  const textNode = document.createElement("span");
  textNode.textContent = text;
  item.append(titleNode, textNode);
  return item;
}

function renderRecommendations(analysis) {
  if (!elements.heroRecommendations) {
    return;
  }
  const recommendations = recommendHeroes(analysis);
  elements.heroRecommendations.replaceChildren(
    ...recommendations.map((recommendation) => {
      const item = document.createElement("div");
      item.className = "recommend-card";
      item.innerHTML = `
        <strong>${recommendation.name}</strong>
        <span>${recommendation.score}</span>
      `;
      return item;
    })
  );
}

function renderBuildRecommendation(analysis) {
  if (!elements.buildRecommendation || !elements.buildSourceNote) {
    return;
  }
  const build = chooseBuildRecommendation(analysis);
  if (!build) {
    elements.buildSourceNote.textContent = "暂无匹配到本地 OP.GG 方案快照";
    elements.buildRecommendation.replaceChildren(emptyBuildState());
    return;
  }

  elements.buildSourceNote.textContent = [
    `${build.champion_name || build.champion_key} · ${roleLabel(build.role)}`,
    build.side === "enemy" ? "敌方参考" : "我方推荐",
    `${build.patch} / ${build.tier}`
  ].join(" · ");

  const primaryRune = build.rune?.primary_style?.name ?? "待确认";
  const secondaryRune = build.rune?.secondary_style?.name ?? "待确认";
  const perkNames = (build.rune?.perks ?? []).map((perk) => perk.name).slice(0, 6);
  const spellBuild = build.summoner_spells?.[0];
  const skillOrder = build.skill_order?.order;

  elements.buildRecommendation.replaceChildren(
    buildBlock("符文", [
      `${primaryRune} + ${secondaryRune}`,
      perkNames.join(" / ") || "暂无符文明细"
    ]),
    buildBlock("召唤师技能", [
      spellBuild ? spellBuild.spells.map((spell) => spell.name).join(" + ") : "暂无推荐",
      metricLine(spellBuild)
    ]),
    buildBlock("技能加点", [
      skillOrder ? compactSkillOrder(skillOrder) : "暂无推荐",
      metricLine(build.skill_order)
    ]),
    buildItemBlock("出门", build.starter_items?.[0]),
    buildItemBlock("鞋子", build.boots?.[0]),
    buildItemBlock("辅助装", build.support_items?.[0]),
    buildItemBlock("核心装备", build.core_items?.[0])
  );
}

function chooseBuildRecommendation(analysis) {
  const builds = analysis?.build_recommendations ?? [];
  return builds.find((build) => build.side === "ally") ?? builds[0] ?? null;
}

function emptyBuildState() {
  const item = document.createElement("div");
  item.className = "build-empty";
  item.textContent = "继续扩展英雄 build 快照后，这里会显示符文、技能和装备路径。";
  return item;
}

function buildBlock(title, lines) {
  const cleanLines = lines.filter(Boolean);
  const item = document.createElement("div");
  item.className = "build-block";

  const titleNode = document.createElement("span");
  titleNode.textContent = title;
  const primary = document.createElement("strong");
  primary.textContent = cleanLines[0] ?? "暂无推荐";
  const secondary = document.createElement("small");
  secondary.textContent = cleanLines[1] ?? "";

  item.replaceChildren(titleNode, primary, secondary);
  return item;
}

function buildItemBlock(title, build) {
  const names = build?.items?.map((item) => item.name || item.item_id).join(" > ");
  return buildBlock(title, [names || "暂无推荐", metricLine(build)]);
}

function metricLine(row) {
  if (!row) {
    return "";
  }

  const parts = [];
  if (row.pick_rate !== undefined && row.pick_rate !== null) {
    parts.push(`登场 ${Number(row.pick_rate).toFixed(2)}%`);
  }
  if (row.win_rate !== undefined && row.win_rate !== null) {
    parts.push(`胜率 ${Number(row.win_rate).toFixed(2)}%`);
  }
  if (row.games) {
    parts.push(`${Number(row.games).toLocaleString("zh-CN")} 场`);
  }
  return parts.join(" · ");
}

function compactSkillOrder(order) {
  return String(order || "").match(/.{1,3}/g)?.join(" ") ?? "";
}

function championName(championId, includeId = true) {
  if (!championId) {
    return "待选";
  }
  const name = championNames[championId] ?? `英雄 ${championId}`;
  return includeId ? `${name} (${championId})` : name;
}

function cacheChampionNames(names = {}) {
  Object.entries(names || {}).forEach(([id, info]) => {
    const championId = Number(id);
    const name = info?.name || info?.alias;
    if (Number.isFinite(championId) && name) {
      championNames[championId] = name;
    }
  });
}

function applyPhase(phase) {
  const nextPhase = String(phase ?? "Unknown");
  state.phase = nextPhase;
  if (shouldClearDraftForPhase(nextPhase)) {
    clearDraftSnapshots();
  }
}

function displaySnapshot() {
  return state.snapshot ?? state.lastDraftSnapshot;
}

function isLiveClientRestoredSnapshot(snapshot) {
  return snapshot?.source === "live-client";
}

function isInGameSnapshot(snapshot) {
  return snapshot?.draft_state?.gameflow === "InProgress" || isLiveClientRestoredSnapshot(snapshot);
}

function isEmptyDeletedDraftSnapshot(snapshot) {
  const draft = snapshot?.draft_state;
  if (!draft || snapshot?.lcu_event_type !== "Delete") {
    return false;
  }
  return (
    countDraftPicks(draft) === 0 &&
    (draft.bans?.length ?? 0) === 0 &&
    (draft.local_player_cell_id === -1 || draft.local_player_cell_id === null)
  );
}

function clearDraftSnapshots() {
  state.snapshot = null;
  state.lastDraftSnapshot = null;
  state.draftFingerprint = null;
}

function shouldClearDraftForPhase(phase) {
  return [
    "None",
    "WaitingForStats",
    "PreEndOfGame",
    "EndOfGame"
  ].includes(String(phase ?? "Unknown"));
}

function isUsableDraftSnapshot(snapshot) {
  const draft = snapshot?.draft_state;
  if (!draft) {
    return false;
  }

  return (
    hasPickedChampion(draft.my_team) ||
    hasPickedChampion(draft.their_team) ||
    (draft.bans?.length ?? 0) > 0
  );
}

function hasTeammatePerformance(snapshot) {
  return (snapshot?.teammate_performance?.length ?? 0) > 0;
}

function mergeTeammatePerformance(snapshot) {
  state.lastDraftSnapshot = {
    ...state.lastDraftSnapshot,
    teammate_performance: snapshot.teammate_performance
  };
  if (state.snapshot) {
    state.snapshot = {
      ...state.snapshot,
      teammate_performance: snapshot.teammate_performance
    };
  }
}

function resetSnapshotsForNewDraft(snapshot) {
  const fingerprint = draftFingerprint(snapshot);
  if (!fingerprint) {
    return;
  }
  if (state.draftFingerprint && state.draftFingerprint !== fingerprint) {
    state.snapshot = null;
    state.lastDraftSnapshot = null;
  }
  state.draftFingerprint = fingerprint;
}

function draftFingerprint(snapshot) {
  const draft = snapshot?.draft_state;
  if (!draft) {
    return "";
  }
  const allyIds = (draft.my_team ?? [])
    .map((player) => player?.summoner_id)
    .filter(Boolean)
    .join(",");
  const localCell = draft.local_player_cell_id ?? "";
  const banIds = (draft.bans ?? [])
    .map((ban) => ban?.champion_id)
    .filter(Boolean)
    .join(",");
  if (!allyIds && !banIds) {
    return "";
  }
  return `${localCell}|${allyIds}|${banIds}`;
}

function hasPickedChampion(players = []) {
  return players.some((player) => player?.champion_id);
}

function countDraftPicks(draft) {
  return countPicked(draft?.my_team ?? []) + countPicked(draft?.their_team ?? []);
}

function isDraftComplete(draft) {
  return countPicked(draft?.my_team ?? []) >= 5 && countPicked(draft?.their_team ?? []) >= 5;
}

function hasEnoughPicksForAdvice(draft) {
  return countDraftPicks(draft) >= 3;
}

function hasEnoughEnemyPicksForAnalysis(draft) {
  return countPicked(draft?.their_team ?? []) >= 2;
}

function positionLabel(position) {
  const labels = {
    top: "上路",
    jungle: "打野",
    middle: "中路",
    bottom: "下路",
    utility: "辅助"
  };
  return labels[position] ?? "位置待定";
}

function roleLabel(role) {
  const labels = {
    top: "上路",
    jungle: "打野",
    mid: "中路",
    adc: "下路",
    support: "辅助"
  };
  return labels[role] ?? role ?? "未知位置";
}

function phaseLabel(phase) {
  if (phase === "ChampSelect") {
    return "BP 阶段检测中";
  }
  if (phase === "InProgress") {
    return "游戏进行中";
  }
  if (phase === "None") {
    return "等待对局";
  }
  if (phase === "Lobby") {
    return "房间中";
  }
  if (phase === "Matchmaking") {
    return "匹配中";
  }
  if (phase === "ReadyCheck") {
    return "确认对局";
  }
  if (phase === "Reconnect") {
    return "等待重连";
  }
  if (phase === "WaitingForStats") {
    return "结算中";
  }
  if (phase === "PreEndOfGame" || phase === "EndOfGame") {
    return "对局结束";
  }
  return phase;
}

function confidenceLabel(confidence) {
  const labels = {
    low: "不足",
    medium: "部分",
    high: "完整"
  };
  return labels[confidence] ?? "不足";
}

function countPicked(players = []) {
  return players.filter((player) => player?.champion_id).length;
}

function formatDelta(label, mine = 0, enemy = 0) {
  const delta = mine - enemy;
  if (Math.abs(delta) <= 8) {
    return `${label} 持平`;
  }
  const prefix = delta > 0 ? "+" : "";
  return `${label} ${prefix}${delta}`;
}

function strongestDimensionGap(myDimensions = {}, enemyDimensions = {}) {
  return Object.keys(metricLabels)
    .map((key) => ({
      key,
      mine: myDimensions[key] ?? 0,
      enemy: enemyDimensions[key] ?? 0,
      delta: (myDimensions[key] ?? 0) - (enemyDimensions[key] ?? 0)
    }))
    .sort((a, b) => Math.abs(b.delta) - Math.abs(a.delta))[0] ?? null;
}

function dimensionHint(key, mine = 0, enemy = 0) {
  const delta = mine - enemy;
  if (Math.abs(delta) <= 8) {
    return "双方接近，更多看操作和资源前站位。";
  }
  const allyAhead = delta > 0;
  const hints = {
    engage: allyAhead ? "我方更容易先手开团，资源前可以主动逼位置。" : "敌方更容易先手，河道和野区入口别站太密。",
    magic_damage: allyAhead ? "我方 AP 压力更足，适合逼敌方补魔抗。" : "敌方 AP 压力更高，关键位要注意魔抗和控制。",
    physical_damage: allyAhead ? "我方 AD 输出更稳定，正面持续输出空间重要。" : "敌方 AD 输出更高，护甲和后排站位更关键。",
    scaling: allyAhead ? "我方后期更好，前中期别无谓送节奏。" : "敌方后期更强，中期资源要更主动。",
    crowd_control: allyAhead ? "我方控制链更完整，抓落单和反开价值高。" : "敌方控制更多，吃第一段控制后容易被接死。",
    frontline: allyAhead ? "我方前排更厚，正面团容错更高。" : "我方承伤偏薄，避免无视野硬接正面团。"
  };
  return hints[key] ?? (allyAhead ? "我方这一项更强。" : "敌方这一项更强。");
}

function enemyDamageProfile(enemyDimensions = {}) {
  const magic = enemyDimensions.magic_damage ?? 0;
  const physical = enemyDimensions.physical_damage ?? 0;

  if (magic >= 60 && physical >= 60) {
    return "伤害结构：混合伤害完整，抗性不能只堆一边。";
  }
  if (magic >= 65 && physical < 45) {
    return "伤害结构：AP 压力更高，注意魔抗和关键控制。";
  }
  if (physical >= 65 && magic < 45) {
    return "伤害结构：AD 压力更高，护甲和站位更关键。";
  }
  if (magic < 45 && physical < 45) {
    return "伤害结构：输出暂不完整，可以观察后续补位。";
  }
  return "伤害结构：相对均衡，优先看谁先拿资源节奏。";
}

function enemyDamageWarningType(enemyDimensions = {}) {
  const magic = enemyDimensions.magic_damage ?? 0;
  const physical = enemyDimensions.physical_damage ?? 0;
  if (magic >= 65 || physical >= 65) {
    return "negative";
  }
  return "suggestion";
}

function dimensionVerdict(mine = 0, enemy = 0) {
  if (mine - enemy >= 15) {
    return { text: "强", className: "good" };
  }
  if (enemy - mine >= 20) {
    return { text: "风险", className: "risk" };
  }
  return { text: "均衡", className: "warn" };
}

function estimateWinScore(myDimensions = {}, enemyDimensions = {}, confidence = "low") {
  const myTotal = Object.values(myDimensions).reduce((sum, value) => sum + value, 0);
  const enemyTotal = Object.values(enemyDimensions).reduce((sum, value) => sum + value, 0);
  const delta = Math.max(-8, Math.min(8, Math.round((myTotal - enemyTotal) / 18)));
  const center = 50 + delta;
  const spread = confidence === "high" ? 2 : confidence === "medium" ? 3 : 5;
  return {
    range: `${center - spread}% - ${center + spread}%`,
    note: `数据完整度：${confidenceLabel(confidence)} · 只用于阵容解释`
  };
}

function recommendHeroes(analysis) {
  const statRecommendations = (analysis?.champion_stats ?? [])
    .filter((stat) => stat.win_rate >= 51 || stat.pick_rate >= 10 || stat.ban_rate >= 15)
    .slice(0, 3)
    .map((stat) => ({
      name: championNames[stat.champion_id] ?? stat.champion_key,
      score: `OP.GG ${stat.role} ${stat.win_rate.toFixed(1)}%`
    }));

  if (statRecommendations.length > 0) {
    return statRecommendations;
  }

  const magic = analysis?.dimensions?.magic_damage ?? 0;
  if (magic <= 40) {
    return [
      { name: "发条魔灵", score: "适配 92" },
      { name: "岩雀", score: "适配 86" },
      { name: "辛德拉", score: "熟练度中" }
    ];
  }

  return [
    { name: "加里奥", score: "反开 88" },
    { name: "阿狸", score: "节奏 84" },
    { name: "维克托", score: "后期 82" }
  ];
}

function shortName(name = "队友") {
  const cleanName = String(name || "队友");
  return cleanName.length > 10 ? `${cleanName.slice(0, 10)}…` : cleanName;
}

function formatAverageKda(teammate) {
  if (!teammate?.games) {
    return "暂无战绩";
  }
  return `KD ${Number(teammate.kd_ratio).toFixed(2)} · KDA ${Number(teammate.avg_kills).toFixed(1)}/${Number(teammate.avg_deaths).toFixed(1)}/${Number(teammate.avg_assists).toFixed(1)}`;
}

function teammateQualitySort(a, b) {
  const tierScore = {
    top_horse: 4,
    stable: 3,
    low_horse: 2,
    workhorse: 1,
    data_light: 0
  };
  const scoreDelta = (tierScore[b?.tier] ?? 0) - (tierScore[a?.tier] ?? 0);
  if (scoreDelta !== 0) {
    return scoreDelta;
  }
  return Number(b?.kd_ratio ?? 0) - Number(a?.kd_ratio ?? 0);
}

function teammateQualitySummary(teammates) {
  const counts = teammateQualityCounts(teammates);

  if (counts.top_horse > 0) {
    return `${counts.top_horse} 个强势队友，优先围绕节奏点。`;
  }
  if (counts.workhorse > 0) {
    return `${counts.workhorse} 个高风险位，前期降低单点压力。`;
  }
  if (counts.low_horse > 0) {
    return `${counts.low_horse} 个偏弱风险位，少让他单独扛局面。`;
  }
  if (counts.data_light === teammates.length) {
    return "队友数据不足，先看阵容和对线分配。";
  }
  return "整体质量中等，按阵容强势期推进。";
}

function teammateQualityCounts(teammates) {
  return teammates.reduce(
    (acc, teammate) => {
      const tier = teammate?.tier ?? "data_light";
      acc[tier] = (acc[tier] ?? 0) + 1;
      return acc;
    },
    { top_horse: 0, stable: 0, low_horse: 0, workhorse: 0, data_light: 0 }
  );
}

function bridgeLabel(status) {
  if (!status) {
    return state.usingLiveData ? "等待 probe 桥接" : "浏览器样例模式";
  }
  const labels = {
    starting: "正在启动 probe",
    running: "probe 已运行",
    stderr: "probe 日志",
    error: "probe 启动失败",
    stopped: "probe 已停止",
    parse_error: "事件解析失败",
    already_running: "probe 已运行",
    listening: "监听选人事件",
    closed: "监听已断开",
    skipped: "等待进入 BP",
    finished: "probe 已结束",
    retrying: "等待自动重试"
  };
  return labels[status.status] ?? "等待客户端启动";
}

function bridgeMessage(status) {
  if (!status) {
    return state.usingLiveData ? "等待实时 LCU 事件" : "使用样例 JSON 事件";
  }
  return friendlyBridgeMessage(status.message || "不会写入客户端配置");
}

function isBridgeError(status) {
  return status?.status === "error" || status?.status === "parse_error";
}

function isDisconnectedBridgeStatus(status) {
  return ["error", "parse_error", "retrying", "stopped"].includes(status?.status);
}

function clearLiveConnection() {
  state.connection = null;
  state.summoner = null;
  state.phase = "Unknown";
  state.snapshot = null;
  state.lastDraftSnapshot = null;
  state.watchStatus = null;
  state.champSelectStatus = null;
}

function liveConnectionSubtitle() {
  const level = state.summoner?.summoner_level ? ` · 等级 ${state.summoner.summoner_level}` : "";
  return `国服 · 召唤师峡谷${level}`;
}

function connectionDisplay() {
  if (!state.usingLiveData && state.connection) {
    return {
      title: "样例数据",
      detail: "样例事件已载入"
    };
  }

  if (isBridgeError(state.bridgeStatus)) {
    return {
      title: "连接失败",
      detail: bridgeMessage(state.bridgeStatus)
    };
  }

  if (state.connection) {
    const watchDetail = state.watchStatus ? bridgeMessage(state.watchStatus) : "正在监听 LCU 实时事件";
    return {
      title: "LCU 已连接",
      detail: `端口 ${state.connection.port} · token 已隐藏 · ${watchDetail}`
    };
  }

  return {
    title: "等待 LCU",
    detail: bridgeMessage(state.bridgeStatus)
  };
}

function waitingSnapshotText() {
  if (!state.usingLiveData) {
    return "等待 LCU 实时事件";
  }
  if (state.champSelectStatus?.status === "skipped") {
    return "客户端已连接，进入 BP 后会自动显示双方阵容";
  }
  if (state.phase && state.phase !== "Unknown" && state.phase !== "ChampSelect") {
    return `当前阶段：${phaseLabel(state.phase)}，等待进入 BP`;
  }
  if (state.connection) {
    return "LCU 已连接，等待选人事件";
  }
  return "等待 LCU 实时事件";
}

function friendlyBridgeMessage(message) {
  const text = String(message || "");
  if (text.includes("lockfile was not found") || text.includes("League Client lockfile was not found")) {
    return "没有找到英雄联盟客户端，请先打开并登录客户端。";
  }
  if (text.includes("operation timed out") || text.includes("timed out")) {
    return "LCU 请求超时，可能是客户端刚启动或 WeGame 正在切换进程，LeagueAkari Plus 会自动重试。";
  }
  if (text.includes("error sending request for url")) {
    return "找到了旧的 LCU 端口，但当前不可用。请确认英雄联盟客户端正在运行，LeagueAkari Plus 会自动重试。";
  }
  if (text.includes("all LCU connection candidates failed")) {
    return "找到了客户端线索，但本地 LCU 接口暂时连不上。请确认英雄联盟客户端正在运行，LeagueAkari Plus 会自动重试。";
  }
  if (text.includes("retrying in")) {
    return "暂时没有连上 LCU，正在后台自动重试。";
  }
  if (text.includes("gameflow is not ChampSelect")) {
    return "当前还不在 BP 阶段，进入选人后会自动刷新。";
  }
  if (text.includes("LCU websocket connected")) {
    return "实时监听已连接，等待游戏阶段变化。";
  }
  if (text.includes("waiting for frontend event bridge")) {
    return "正在建立前端和 Rust 后端的事件桥。";
  }
  if (text.includes("starting ")) {
    return "正在启动本地 LCU 探针。";
  }
  if (text.includes("probe process started")) {
    return "本地探针已启动。";
  }
  if (text.includes("Ctrl+C received")) {
    return "监听已停止。";
  }
  if (text.includes("websocket closed")) {
    return "LCU 实时连接已断开，可重新连接。";
  }
  if (text.includes("probe process exited")) {
    return "本地探针已结束，可重新连接。";
  }
  return text;
}

async function startTauriEventBridge() {
  const tauriEvent = window.__TAURI__?.event;
  if (!tauriEvent?.listen || !tauriEvent?.emit) {
    return false;
  }

  resetLiveState();
  state.bridgeStatus = {
    status: "starting",
    message: "waiting for frontend event bridge"
  };
  render();

  await tauriEvent.listen("leagueakari-probe-event", (event) => {
    applyEvent(event.payload);
  });
  await tauriEvent.emit("leagueakari-frontend-ready");
  return true;
}

function resetLiveState() {
  state.connection = null;
  state.summoner = null;
  state.phase = "Unknown";
  state.snapshot = null;
  state.lastDraftSnapshot = null;
  state.bridgeStatus = null;
  state.watchStatus = null;
  state.champSelectStatus = null;
  state.usingLiveData = true;
}

async function reconnectLiveData() {
  const tauriEvent = window.__TAURI__?.event;
  if (!tauriEvent?.emit) {
    loadSampleEvents();
    return;
  }

  resetLiveState();
  state.bridgeStatus = {
    status: "starting",
    message: "waiting for frontend event bridge"
  };
  render();
  await tauriEvent.emit("leagueakari-frontend-ready");
}

async function fetchRecentMatches(force = false) {
  if (state.matchHistoryStatus === "loading") {
    return;
  }
  if (!force && state.matchHistory?.matches?.length) {
    return;
  }

  const tauriCore = window.__TAURI__?.core;
  if (!tauriCore?.invoke) {
    state.matchHistoryStatus = "error";
    state.matchHistoryError = "当前不是桌面客户端，无法读取 LCU 最近战绩。";
    render();
    return;
  }

  state.matchHistoryStatus = "loading";
  state.matchHistoryError = null;
  render();

  try {
    const output = await tauriCore.invoke("fetch_recent_matches");
    const recentEvent = parseProbeEventLines(output).find((event) => event.event === "recent_matches");
    if (!recentEvent?.payload) {
      throw new Error("probe 没有返回 recent_matches 事件");
    }
    state.matchHistory = recentEvent.payload;
    state.matchHistoryStatus = "ready";
    state.matchHistoryError = null;
  } catch (error) {
    state.matchHistoryStatus = "error";
    state.matchHistoryError = `读取最近战绩失败：${humanizeBridgeMessage(String(error?.message ?? error))}`;
  }

  render();
}

function parseProbeEventLines(output) {
  return String(output || "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return null;
      }
    })
    .filter(Boolean);
}

function loadSampleEvents() {
  state.connection = null;
  state.summoner = null;
  state.phase = "Unknown";
  state.snapshot = null;
  state.lastDraftSnapshot = null;
  state.bridgeStatus = null;
  state.watchStatus = null;
  state.champSelectStatus = null;
  state.usingLiveData = false;
  window.LEAGUEAKARI_SAMPLE_EVENTS.forEach(applyEvent);
}

function openDetailModal(panel) {
  const config = {
    ally: {
      title: "我方阵容分析",
      subtitle: "优势、风险和本局打法建议",
      items: state.detailItems.ally
    },
    enemy: {
      title: "对方阵容分析",
      subtitle: "敌方威胁点和处理方式",
      items: state.detailItems.enemy
    },
    teammates: {
      title: "队友质量分析",
      subtitle: "基于队友近期表现的本地判断",
      items: state.detailItems.teammates
    }
  }[panel];

  if (!config || !elements.detailModal) {
    return;
  }

  elements.detailModalTitle.textContent = config.title;
  elements.detailModalSubtitle.textContent = config.subtitle;
  elements.detailModalBody.replaceChildren(...detailRows(config.items));
  elements.detailModal.classList.remove("hidden");
}

function closeDetailModal() {
  elements.detailModal?.classList.add("hidden");
}

function detailRows(items = []) {
  const rows = items.length
    ? items
    : [{ title: "暂无详情", text: "等待更多本局数据。", type: "suggestion" }];

  return rows.map((item) => {
    const row = document.createElement("div");
    row.className = `detail-row ${item.type ?? "suggestion"}`;
    const title = document.createElement("strong");
    title.textContent = item.title;
    const text = document.createElement("span");
    text.textContent = item.text;
    row.append(title, text);
    return row;
  });
}

document.querySelectorAll("[data-detail-panel]").forEach((node) => {
  node.addEventListener("click", (event) => {
    event.stopPropagation();
    openDetailModal(node.dataset.detailPanel);
  });
  node.addEventListener("keydown", (event) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      openDetailModal(node.dataset.detailPanel);
    }
  });
});

elements.detailModalClose?.addEventListener("click", closeDetailModal);
elements.detailModal?.addEventListener("click", (event) => {
  if (event.target === elements.detailModal) {
    closeDetailModal();
  }
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    closeDetailModal();
  }
});

elements.navItems.forEach((item) => {
  item.addEventListener("click", (event) => {
    event.preventDefault();
    state.currentView = item.dataset.view || "draft";
    render();
    if (state.currentView === "history") {
      fetchRecentMatches(false);
    }
  });
});

elements.loadSampleButton.addEventListener("click", () => {
  loadSampleEvents();
});

elements.reconnectButton.addEventListener("click", () => {
  reconnectLiveData().catch((error) => {
    state.bridgeStatus = {
      status: "error",
      message: `重新连接失败：${error}`
    };
    render();
  });
});

elements.refreshMatchesButton?.addEventListener("click", () => {
  fetchRecentMatches(true);
});

startTauriEventBridge()
  .then((connected) => {
    if (!connected) {
      loadSampleEvents();
    }
  })
  .catch((error) => {
    state.bridgeStatus = {
      status: "error",
      message: `Tauri event bridge failed: ${error}`
    };
    render();
  });
