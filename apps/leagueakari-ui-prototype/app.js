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
  phase: "Unknown",
  snapshot: null,
  bridgeStatus: null,
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
  loadSampleButton: document.querySelector("#loadSampleButton")
};

function applyEvent(message) {
  if (message.event === "probe_bridge_status") {
    state.bridgeStatus = message.payload;
  }

  if (message.event === "lcu_connection") {
    state.connection = message.payload;
  }

  if (message.event === "gameflow_phase") {
    state.phase = String(message.payload.phase ?? "Unknown");
  }

  if (message.event === "draft_snapshot") {
    state.snapshot = message.payload;
    state.phase = message.payload.draft_state?.gameflow ?? state.phase;
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
  const hasLiveConnection = state.usingLiveData && state.connection;
  const hasSampleConnection = !state.usingLiveData && state.connection;

  elements.connectionStatus.textContent = hasLiveConnection
    ? "LCU 已连接"
    : hasSampleConnection
      ? "样例数据"
      : "等待 LCU";
  elements.serverStatus.textContent = state.connection
    ? hasLiveConnection
      ? "国服 · 召唤师峡谷"
      : bridgeLabel(state.bridgeStatus)
    : bridgeLabel(state.bridgeStatus);
  elements.connectionDetail.textContent = state.connection
    ? hasLiveConnection
      ? `端口 ${state.connection.port} · token 已隐藏`
      : "样例事件已载入"
    : bridgeMessage(state.bridgeStatus);
  elements.gameflowPhase.textContent = phaseLabel(state.phase);
  elements.confidence.textContent = confidenceLabel(analysis?.confidence);
  elements.snapshotSource.textContent = snapshot
    ? `${snapshot.source} ${snapshot.lcu_event_type ?? ""}`.trim()
    : "等待 LCU 实时事件";

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
  const strengths = (analysis?.strengths ?? []).slice(0, 2);
  const risks = (analysis?.risks ?? []).slice(0, 2);
  const suggestions = (analysis?.suggestions ?? []).slice(0, 1);
  const reasons = [
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
    already_running: "probe 已运行"
  };
  return labels[status.status] ?? "等待客户端启动";
}

function bridgeMessage(status) {
  if (!status) {
    return state.usingLiveData ? "等待实时 LCU 事件" : "使用样例 JSON 事件";
  }
  return status.message || "不会写入客户端配置";
}

async function startTauriEventBridge() {
  const tauriEvent = window.__TAURI__?.event;
  if (!tauriEvent?.listen || !tauriEvent?.emit) {
    return false;
  }

  state.usingLiveData = true;
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

elements.loadSampleButton.addEventListener("click", () => {
  window.LEAGUEAKARI_SAMPLE_EVENTS.forEach(applyEvent);
});

startTauriEventBridge()
  .then((connected) => {
    if (!connected) {
      window.LEAGUEAKARI_SAMPLE_EVENTS.forEach(applyEvent);
    }
  })
  .catch((error) => {
    state.bridgeStatus = {
      status: "error",
      message: `Tauri event bridge failed: ${error}`
    };
    render();
  });
