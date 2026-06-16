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
        data_notes: [
          "OP.GG 快照：16.12 / global / emerald_plus / ranked_solo_duo，样本量 3256825。",
          "本局匹配到 6 个英雄的 OP.GG 角色统计；统计只辅助解释，不直接承诺胜负。"
        ],
        champion_stats: [
          { champion_id: 22, champion_key: "ashe", role: "adc", win_rate: 50.39, pick_rate: 11.71, ban_rate: 9.82, rank: 20 },
          { champion_id: 104, champion_key: "graves", role: "jungle", win_rate: 49.51, pick_rate: 12.14, ban_rate: 15.35, rank: 26 },
          { champion_id: 89, champion_key: "leona", role: "support", win_rate: 52.42, pick_rate: 7.92, ban_rate: 7.3, rank: 4 },
          { champion_id: 111, champion_key: "nautilus", role: "support", win_rate: 50.04, pick_rate: 10.98, ban_rate: 14.94, rank: 23 },
          { champion_id: 54, champion_key: "malphite", role: "top", win_rate: 51.3, pick_rate: 7.02, ban_rate: 18.79, rank: 14 },
          { champion_id: 56, champion_key: "nocturne", role: "jungle", win_rate: 50.3, pick_rate: 6.87, ban_rate: 15.88, rank: 21 }
        ],
        build_recommendations: [
          {
            champion_id: 111,
            champion_key: "nautilus",
            champion_name: "Nautilus",
            side: "enemy",
            role: "support",
            patch: "16.12",
            region: "global",
            tier: "emerald_plus",
            queue: "ranked_solo_duo",
            source_url: "https://op.gg/lol/champions/nautilus/build/support",
            rune: {
              primary_style: { style_id: 8400, name: "Resolve", icon_url: null },
              secondary_style: { style_id: 8300, name: "Inspiration", icon_url: null },
              perks: [
                { perk_id: 8439, name: "Aftershock", icon_url: null },
                { perk_id: 8401, name: "Shield Bash", icon_url: null },
                { perk_id: 8473, name: "Bone Plating", icon_url: null },
                { perk_id: 8242, name: "Unflinching", icon_url: null },
                { perk_id: 8345, name: "Biscuit Delivery", icon_url: null },
                { perk_id: 8347, name: "Cosmic Insight", icon_url: null }
              ]
            },
            summoner_spells: [
              {
                spells: [
                  { spell_key: "SummonerFlash", name: "Flash", icon_url: null },
                  { spell_key: "SummonerDot", name: "Ignite", icon_url: null }
                ],
                pick_rate: 71.83,
                games: 19879,
                win_rate: 49.82
              }
            ],
            skill_order: {
              order: "QWEQWEQQRQWQWRWWEE",
              skills: [
                { spell_key: "NautilusAnchorDrag", name: "Dredge Line", icon_url: null },
                { spell_key: "NautilusPiercingGaze", name: "Titan's Wrath", icon_url: null },
                { spell_key: "NautilusSplashZone", name: "Riptide", icon_url: null }
              ],
              pick_rate: 72.68,
              games: 3076,
              win_rate: 65.99
            },
            starter_items: [
              {
                items: [{ item_id: 2003, name: "Health Potion", icon_url: null }],
                pick_rate: 96.39,
                games: 29149,
                win_rate: 49.93
              }
            ],
            boots: [
              {
                items: [{ item_id: 3047, name: "Plated Steelcaps", icon_url: null }],
                pick_rate: 57.29,
                games: 16613,
                win_rate: 50.24
              }
            ],
            support_items: [
              {
                items: [{ item_id: 3869, name: "Celestial Opposition", icon_url: null }],
                pick_rate: 59.84,
                games: 16374,
                win_rate: 48.91
              }
            ],
            core_items: [
              {
                items: [
                  { item_id: 3190, name: "Locket of the Iron Solari", icon_url: null },
                  { item_id: 2524, name: "Bandlepipes", icon_url: null },
                  { item_id: 3109, name: "Knight's Vow", icon_url: null }
                ],
                pick_rate: 9.23,
                games: 812,
                win_rate: 57.27
              }
            ]
          }
        ],
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
        enemy_focus_targets: [
          {
            champion_id: 111,
            champion_key: "nautilus",
            focus_type: "crowd_control",
            priority: 95,
            reason: "关键控制点，被第一段控制命中后容易被接后续技能。"
          },
          {
            champion_id: 901,
            champion_key: "smolder",
            focus_type: "scaling",
            priority: 90,
            reason: "后期成长点，中期资源节奏不能完全放给他发育。"
          },
          {
            champion_id: 54,
            champion_key: "malphite",
            focus_type: "frontline",
            priority: 90,
            reason: "主要前排承伤点，能绕开就别把第一波技能全交给他。"
          }
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
