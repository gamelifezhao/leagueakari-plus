window.LEAGUEAKARI_SAMPLE_EVENTS = [
  {
    event: "lcu_connection",
    payload: {
      source: "LeagueClientUx log arguments",
      path: "F:\\WeGameApps\\英雄联盟\\LeagueClient\\LeagueClientUx.log",
      pid: null,
      port: 54159,
      protocol: "https",
      token_hidden: true
    }
  },
  {
    event: "gameflow_phase",
    payload: {
      phase: "ChampSelect"
    }
  },
  {
    event: "draft_snapshot",
    payload: {
      source: "watch",
      lcu_event_type: "Update",
      draft_state: {
        connected: true,
        gameflow: "ChampSelect",
        local_player_cell_id: 2,
        my_team: [
          { cell_id: 0, champion_id: 22, assigned_position: "bottom", summoner_id: null },
          { cell_id: 1, champion_id: 104, assigned_position: "jungle", summoner_id: null },
          { cell_id: 2, champion_id: 89, assigned_position: "utility", summoner_id: null },
          { cell_id: 3, champion_id: 86, assigned_position: "top", summoner_id: null },
          { cell_id: 4, champion_id: 117, assigned_position: "middle", summoner_id: null }
        ],
        their_team: [
          { cell_id: 5, champion_id: 111, assigned_position: "utility", summoner_id: null },
          { cell_id: 6, champion_id: 54, assigned_position: "top", summoner_id: null },
          { cell_id: 7, champion_id: 901, assigned_position: "bottom", summoner_id: null },
          { cell_id: 8, champion_id: 90, assigned_position: "middle", summoner_id: null },
          { cell_id: 9, champion_id: 56, assigned_position: "jungle", summoner_id: null }
        ],
        bans: [
          { champion_id: 11, team_id: 100 },
          { champion_id: 517, team_id: 100 },
          { champion_id: 63, team_id: 100 },
          { champion_id: 83, team_id: 100 },
          { champion_id: 950, team_id: 100 },
          { champion_id: 25, team_id: 200 },
          { champion_id: 17, team_id: 200 },
          { champion_id: 266, team_id: 200 },
          { champion_id: 105, team_id: 200 },
          { champion_id: 11, team_id: 200 }
        ]
      },
      analysis: {
        confidence: "high",
        dimensions: {
          engage: 100,
          frontline: 100,
          magic_damage: 65,
          physical_damage: 100,
          crowd_control: 100,
          scaling: 100
        },
        enemy_dimensions: {
          engage: 100,
          frontline: 100,
          magic_damage: 100,
          physical_damage: 100,
          crowd_control: 100,
          scaling: 100
        },
        strengths: [
          "我方具备较好的主动开团能力。",
          "我方前排厚度较好，团战容错更高。",
          "我方控制链较充足，容易配合抓机会。"
        ],
        risks: [
          "敌方强开和控制链很强，站位过密时容易被连续进场。",
          "敌方伤害类型比较混合，单一抗性装备收益会下降。"
        ],
        enemy_threats: [
          "敌方具备强先手，第一波开团会决定很多团战结果。",
          "敌方前排较硬，正面阵地战不适合无脑硬灌坦克。",
          "敌方控制链很足，被先手命中后容易连续吃技能。"
        ],
        win_conditions: [
          "利用我方控制链先手，优先逼出敌方关键进场或保命技能。",
          "团战站位要分散，留关键控制给敌方第一波进场。",
          "稳住核心装备前少接无视野窄口团，成型后用反开和持续输出赢团。"
        ],
        suggestions: [
          "面对敌方强开，避免五人挤在狭窄入口，先用视野逼他们交开团。",
          "双方都有后期能力时，关键是小龙、先锋和大龙前的提前站位。"
        ]
      }
    }
  }
];
