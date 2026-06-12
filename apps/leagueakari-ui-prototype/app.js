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
  engage: "开团",
  frontline: "前排",
  magic_damage: "AP",
  physical_damage: "AD",
  crowd_control: "控制",
  scaling: "后期"
};

const state = {
  connection: null,
  phase: "Unknown",
  snapshot: null
};

const elements = {
  connectionStatus: document.querySelector("#connectionStatus"),
  gameflowPhase: document.querySelector("#gameflowPhase"),
  confidence: document.querySelector("#confidence"),
  myMetrics: document.querySelector("#myMetrics"),
  enemyMetrics: document.querySelector("#enemyMetrics"),
  snapshotSource: document.querySelector("#snapshotSource"),
  myPicks: document.querySelector("#myPicks"),
  enemyPicks: document.querySelector("#enemyPicks"),
  myBans: document.querySelector("#myBans"),
  enemyBans: document.querySelector("#enemyBans"),
  myPickCount: document.querySelector("#myPickCount"),
  enemyPickCount: document.querySelector("#enemyPickCount"),
  enemyThreats: document.querySelector("#enemyThreats"),
  winConditions: document.querySelector("#winConditions"),
  risksAndSuggestions: document.querySelector("#risksAndSuggestions"),
  loadSampleButton: document.querySelector("#loadSampleButton")
};

function applyEvent(message) {
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

  elements.connectionStatus.textContent = state.connection ? "已连接" : "等待数据";
  elements.gameflowPhase.textContent = state.phase;
  elements.confidence.textContent = analysis?.confidence ?? "low";
  elements.snapshotSource.textContent = snapshot
    ? `${snapshot.source} ${snapshot.lcu_event_type ?? ""}`.trim()
    : "等待 LCU 实时事件";

  renderMetrics(elements.myMetrics, analysis?.dimensions);
  renderMetrics(elements.enemyMetrics, analysis?.enemy_dimensions);
  renderPicks(elements.myPicks, draft?.my_team ?? []);
  renderPicks(elements.enemyPicks, draft?.their_team ?? []);
  renderBans(elements.myBans, draft?.bans ?? [], 100);
  renderBans(elements.enemyBans, draft?.bans ?? [], 200);

  elements.myPickCount.textContent = `${countPicked(draft?.my_team)} / 5`;
  elements.enemyPickCount.textContent = `${countPicked(draft?.their_team)} / 5`;

  renderList(elements.enemyThreats, analysis?.enemy_threats ?? []);
  renderList(elements.winConditions, analysis?.win_conditions ?? []);
  renderList(elements.risksAndSuggestions, [
    ...(analysis?.risks ?? []),
    ...(analysis?.suggestions ?? [])
  ]);
}

function renderMetrics(container, dimensions = {}) {
  container.replaceChildren(
    ...Object.entries(metricLabels).map(([key, label]) => {
      const value = dimensions[key] ?? 0;
      const item = document.createElement("div");
      item.className = "metric";
      item.innerHTML = `
        <span>${label}</span>
        <div class="bar"><div class="fill" style="width: ${value}%"></div></div>
        <strong>${value}</strong>
      `;
      return item;
    })
  );
}

function renderPicks(container, players) {
  const slots = Array.from({ length: 5 }, (_, index) => players[index] ?? null);
  container.replaceChildren(
    ...slots.map((player, index) => {
      const championId = player?.champion_id;
      const item = document.createElement("article");
      item.className = "pick-slot";
      item.innerHTML = `
        <div class="pick-icon">${index + 1}</div>
        <div>
          <div class="pick-name">${championName(championId)}</div>
          <div class="pick-meta">${positionLabel(player?.assigned_position)}</div>
        </div>
      `;
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
      chip.textContent = championName(ban.champion_id);
      return chip;
    })
  );
}

function renderList(container, items) {
  if (items.length === 0) {
    container.innerHTML = '<li class="empty">等待更多选人信息</li>';
    return;
  }

  container.replaceChildren(
    ...items.map((text) => {
      const item = document.createElement("li");
      item.textContent = text;
      return item;
    })
  );
}

function championName(championId) {
  if (!championId) {
    return "未选择";
  }
  return `${championNames[championId] ?? "未知英雄"} (${championId})`;
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

function countPicked(players = []) {
  return players.filter((player) => player?.champion_id).length;
}

elements.loadSampleButton.addEventListener("click", () => {
  window.LEAGUEAKARI_SAMPLE_EVENTS.forEach(applyEvent);
});

window.LEAGUEAKARI_SAMPLE_EVENTS.forEach(applyEvent);
