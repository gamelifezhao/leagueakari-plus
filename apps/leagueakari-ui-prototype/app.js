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
  connection: null,
  summoner: null,
  phase: "Unknown",
  snapshot: null,
  bridgeStatus: null,
  watchStatus: null,
  champSelectStatus: null,
  usingLiveData: false
};

const elements = {
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
  adviceTags: document.querySelector("#adviceTags"),
  myPicks: document.querySelector("#myPicks"),
  enemyPicks: document.querySelector("#enemyPicks"),
  myBans: document.querySelector("#myBans"),
  enemyBans: document.querySelector("#enemyBans"),
  myPickCount: document.querySelector("#myPickCount"),
  enemyPickCount: document.querySelector("#enemyPickCount"),
  dimensionCompare: document.querySelector("#dimensionCompare"),
  keyReasons: document.querySelector("#keyReasons"),
  heroRecommendations: document.querySelector("#heroRecommendations"),
  buildSourceNote: document.querySelector("#buildSourceNote"),
  buildRecommendation: document.querySelector("#buildRecommendation"),
  reconnectButton: document.querySelector("#reconnectButton"),
  loadSampleButton: document.querySelector("#loadSampleButton")
};

function applyEvent(message) {
  if (!message?.event) {
    return;
  }

  if (message.event === "probe_bridge_status") {
    state.bridgeStatus = message.payload;
  }

  if (message.event === "lcu_connection") {
    state.connection = message.payload;
  }

  if (message.event === "summoner_summary") {
    state.summoner = message.payload;
  }

  if (message.event === "gameflow_phase") {
    state.phase = String(message.payload.phase ?? "Unknown");
  }

  if (message.event === "watch_gameflow") {
    state.phase = String(message.payload.phase ?? "Unknown");
  }

  if (message.event === "watch_status") {
    state.watchStatus = message.payload;
  }

  if (message.event === "champ_select_status") {
    state.champSelectStatus = message.payload;
  }

  if (message.event === "probe_status") {
    state.watchStatus = message.payload;
  }

  if (message.event === "draft_snapshot") {
    state.snapshot = message.payload;
    state.phase = message.payload.draft_state?.gameflow ?? state.phase;
    state.champSelectStatus = null;
  }

  render();
}

function render() {
  const snapshot = state.snapshot;
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
  renderBans(elements.myBans, draft?.bans ?? [], 100);
  renderBans(elements.enemyBans, draft?.bans ?? [], 200);
  renderDimensions(myDimensions, enemyDimensions);
  renderAdvice(analysis);
  renderReasons(analysis);
  renderRecommendations(analysis);
  renderBuildRecommendation(analysis);

  elements.myPickCount.textContent = `${countPicked(draft?.my_team)} / 5`;
  elements.enemyPickCount.textContent = `${countPicked(draft?.their_team)} / 5`;
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

function renderBans(container, bans, teamId) {
  const teamBans = bans.filter((ban) => ban.team_id === teamId);
  if (teamBans.length === 0) {
    container.innerHTML = '<span class="empty">暂无</span>';
    return;
  }

  container.replaceChildren(
    ...teamBans.map((ban) => {
      const chip = document.createElement("span");
      chip.className = "ban-chip";
      chip.textContent = championName(ban.champion_id, false);
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

function renderAdvice(analysis) {
  const suggestions = analysis?.suggestions ?? [];
  const winConditions = analysis?.win_conditions ?? [];
  elements.currentAdvice.textContent =
    suggestions[0] ?? winConditions[0] ?? "等待更多选人信息。";

  const tags = adviceTagsFor(analysis);
  elements.adviceTags.replaceChildren(
    ...tags.map((tag) => {
      const item = document.createElement("span");
      item.className = tag.type;
      item.textContent = tag.text;
      return item;
    })
  );
}

function renderReasons(analysis) {
  const dataNotes = (analysis?.data_notes ?? []).slice(0, 1);
  const strengths = (analysis?.strengths ?? []).slice(0, 2);
  const risks = (analysis?.risks ?? []).slice(0, 2);
  const suggestions = (analysis?.suggestions ?? []).slice(0, 1);
  const reasons = [
    ...dataNotes.map((text) => ({ text: `数据：${text}`, type: "suggestion" })),
    ...strengths.map((text) => ({ text: `+ ${text}`, type: "positive" })),
    ...risks.map((text) => ({ text: `- ${text}`, type: "negative" })),
    ...suggestions.map((text) => ({ text: `建议：${text}`, type: "suggestion" }))
  ];

  if (reasons.length === 0) {
    reasons.push({ text: "等待双方阵容成型后生成关键原因。", type: "suggestion" });
  }

  elements.keyReasons.replaceChildren(
    ...reasons.map((reason) => {
      const item = document.createElement("li");
      item.className = reason.type;
      item.textContent = reason.text;
      return item;
    })
  );
}

function renderRecommendations(analysis) {
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
  const name = championNames[championId] ?? "未知英雄";
  return includeId ? `${name} (${championId})` : name;
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
    low: "低",
    medium: "中",
    high: "高"
  };
  return labels[confidence] ?? "低";
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
    note: `可信度：${confidenceLabel(confidence)} · 只用于阵容解释`
  };
}

function adviceTagsFor(analysis) {
  if (!analysis) {
    return [{ text: "等待阵容", type: "gold-tag" }];
  }

  const tags = [];
  const magic = analysis.dimensions?.magic_damage ?? 0;
  const engage = analysis.dimensions?.engage ?? 0;
  const enemyEngage = analysis.enemy_dimensions?.engage ?? 0;

  if (magic <= 40) {
    tags.push({ text: "优先补 AP", type: "gold-tag" });
  }
  if (engage >= 65) {
    tags.push({ text: "可以主动开团", type: "teal-tag" });
  }
  if (enemyEngage >= 75) {
    tags.push({ text: "注意反开站位", type: "gold-tag" });
  }

  return tags.length > 0 ? tags : [{ text: "稳住资源节奏", type: "teal-tag" }];
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

function loadSampleEvents() {
  state.connection = null;
  state.summoner = null;
  state.phase = "Unknown";
  state.snapshot = null;
  state.bridgeStatus = null;
  state.watchStatus = null;
  state.champSelectStatus = null;
  state.usingLiveData = false;
  window.LEAGUEAKARI_SAMPLE_EVENTS.forEach(applyEvent);
}

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
