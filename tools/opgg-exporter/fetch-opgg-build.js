#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..");
const defaultStatsPath = path.join(
  repoRoot,
  "crates",
  "leagueakari-probe",
  "data",
  "opgg-champion-stats.sample.json"
);

function usage() {
  console.log(`Usage:
  node tools/opgg-exporter/fetch-opgg-build.js <url...> [options]

Options:
  --output <path>       Write JSON to a file instead of stdout.
  --html <path>         Parse a saved OP.GG HTML file instead of requesting the URL.
  --stats <path>        Read patch/region/tier/queue defaults from a stat snapshot.
  --patch <value>       Override patch, e.g. 16.12.
  --region <value>      Override region. Defaults to the stat snapshot value.
  --tier <value>        Override tier. Defaults to the stat snapshot value.
  --queue <value>       Override queue. Defaults to the stat snapshot value.
  -h, --help            Show this help.

This tool only requests public OP.GG pages. It does not log in, solve challenges, or bypass WAF.`);
}

function parseArgs(argv) {
  const options = {
    urls: [],
    outputPath: null,
    htmlPath: null,
    statsPath: defaultStatsPath,
    patch: null,
    region: null,
    tier: null,
    queue: null,
    help: false
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "-h" || arg === "--help") {
      options.help = true;
    } else if (arg === "--output") {
      options.outputPath = requireValue(argv, (index += 1), "--output");
    } else if (arg === "--html") {
      options.htmlPath = requireValue(argv, (index += 1), "--html");
    } else if (arg === "--stats") {
      options.statsPath = requireValue(argv, (index += 1), "--stats");
    } else if (arg === "--patch") {
      options.patch = requireValue(argv, (index += 1), "--patch");
    } else if (arg === "--region") {
      options.region = normalizeToken(requireValue(argv, (index += 1), "--region"));
    } else if (arg === "--tier") {
      options.tier = normalizeToken(requireValue(argv, (index += 1), "--tier"));
    } else if (arg === "--queue") {
      options.queue = normalizeToken(requireValue(argv, (index += 1), "--queue"));
    } else if (arg.startsWith("-")) {
      throw new Error(`Unknown option: ${arg}`);
    } else {
      options.urls.push(arg);
    }
  }

  return options;
}

function requireValue(argv, index, name) {
  const value = argv[index];
  if (!value || value.startsWith("-")) {
    throw new Error(`${name} requires a value.`);
  }
  return value;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function text(value) {
  return decodeHtml(String(value || "").replace(/<!--\s*-->/g, ""))
    .replace(/<[^>]+>/g, " ")
    .trim()
    .replace(/\s+/g, " ");
}

function stripComments(value) {
  return String(value || "").replace(/<!--\s*-->/g, "");
}

function decodeHtml(value) {
  return String(value || "")
    .replace(/&#x([0-9a-f]+);/gi, (_, hex) => String.fromCodePoint(parseInt(hex, 16)))
    .replace(/&#([0-9]+);/g, (_, code) => String.fromCodePoint(Number(code)))
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">");
}

function attr(tag, name) {
  const match = String(tag || "").match(new RegExp(`${name}="([^"]*)"`, "i"));
  return match ? decodeHtml(match[1]) : "";
}

function parseRate(value) {
  const match = text(value).match(/([0-9]+(?:\.[0-9]+)?)\s*%/);
  return match ? Number(match[1]) : null;
}

function parseGames(value) {
  const match = text(value).match(/([0-9,]+)\s*Games?/i);
  return match ? Number(match[1].replace(/,/g, "")) : null;
}

function parseMetricCell(value) {
  const content = text(value);
  const explicitRate = content.match(/([0-9]+(?:\.[0-9]+)?)\s*%/);

  if (explicitRate) {
    const games = content
      .slice(explicitRate.index + explicitRate[0].length)
      .match(/([0-9,]+)\s*Games?/i);
    return {
      rate: Number(explicitRate[1]),
      games: games ? Number(games[1].replace(/,/g, "")) : parseGames(content)
    };
  }

  const compact = content.match(/^([0-9]+)\.([0-9]{1,2})([0-9,]+)\s*Games?/i);
  if (compact) {
    return {
      rate: Number(`${compact[1]}.${compact[2]}`),
      games: Number(compact[3].replace(/,/g, ""))
    };
  }

  const spaced = content.match(/^([0-9]+(?:\.[0-9]+)?)\s+([0-9,]+)\s*Games?/i);
  if (spaced) {
    return {
      rate: Number(spaced[1]),
      games: Number(spaced[2].replace(/,/g, ""))
    };
  }

  return {
    rate: parseRate(content),
    games: parseGames(content)
  };
}

function normalizeToken(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/\s*\+\s*/g, "_plus")
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function imageAsset(imgTag) {
  const src = attr(imgTag, "src");
  const match = src.match(/\/(spell|summonerSpell|perkStyle|perk|item)\/([^/?]+)\./i);
  return {
    id: match?.[2] || null,
    kind: match?.[1] || null,
    name: text(attr(imgTag, "alt")) || null,
    src
  };
}

function imgTags(html) {
  return Array.from(String(html || "").matchAll(/<img\b[^>]*>/gi)).map((match) => match[0]);
}

function tablesByCaption(html, captionPattern) {
  return Array.from(String(html || "").matchAll(/<table\b[\s\S]*?<\/table>/gi))
    .map((match) => match[0])
    .filter((table) => captionPattern.test(text(table.match(/<caption[^>]*>([\s\S]*?)<\/caption>/i)?.[1])));
}

function cells(rowHtml) {
  return Array.from(String(rowHtml || "").matchAll(/<(td|th)\b[^>]*>([\s\S]*?)<\/\1>/gi)).map(
    (match) => match[2]
  );
}

function tableRows(html, captionPattern) {
  return tablesByCaption(html, captionPattern).flatMap((table) =>
    Array.from(table.matchAll(/<tbody\b[^>]*>([\s\S]*?)<\/tbody>/gi)).flatMap((tbody) =>
      Array.from(tbody[1].matchAll(/<tr\b[^>]*>([\s\S]*?)<\/tr>/gi)).map((row) => {
        const rowCells = cells(row[1]);
        const assetText = text(rowCells[0]);
        const assets = imgTags(rowCells[0])
          .map(imageAsset)
          .filter((asset) => asset.id);
        const pickText = rowCells[1] || "";
        const winText = rowCells[2] || rowCells[1] || "";
        const pickMetric = parseMetricCell(pickText);

        return {
          assets,
          asset_text: assetText,
          pick_rate: pickMetric.rate,
          games: pickMetric.games,
          win_rate: parseRate(winText),
          raw_text: text(row[1])
        };
      })
    )
  );
}

function simpleBuildRows(html, captionPattern) {
  return tableRows(html, captionPattern).map((row) => ({
    items: row.assets.map((asset) => ({
      item_id: Number(asset.id),
      name:
        asset.name ||
        (row.assets.length === 1 && !/^[0-9]+$/.test(row.asset_text) ? row.asset_text : null),
      icon_url: asset.src
    })),
    pick_rate: row.pick_rate,
    games: row.games,
    win_rate: row.win_rate,
    raw_text: row.raw_text
  }));
}

function spellRows(html) {
  return tableRows(html, /summoner/i).map((row) => ({
    spells: row.assets.map((asset) => ({
      spell_key: asset.id,
      name: asset.name,
      icon_url: asset.src
    })),
    pick_rate: row.pick_rate,
    games: row.games,
    win_rate: row.win_rate,
    raw_text: row.raw_text
  }));
}

function skillRows(html) {
  return tableRows(html, /skillorder/i).map((row) => {
    const skillLetters = Array.from(
      row.raw_text.matchAll(/\b([QWER])\b/g),
      (match) => match[1]
    ).join("");
    const order = skillLetters.length > 18 ? skillLetters.slice(-18) : skillLetters;

    return {
      order,
      skills: uniqueById(
        row.assets
          .filter((asset) => asset.kind && /spell|summonerSpell/i.test(asset.kind))
          .map((asset) => ({
            spell_key: asset.id,
            name: asset.name,
            icon_url: asset.src
          }))
      ).slice(0, 4),
      pick_rate: row.pick_rate,
      games: row.games,
      win_rate: row.win_rate,
      raw_text: row.raw_text
    };
  });
}

function selectedRuneAssets(html) {
  const selectedIndex = html.indexOf("border-main-200");
  const exportIndex = html.indexOf("Export rune build", selectedIndex);
  if (selectedIndex < 0 || exportIndex < 0) {
    return [];
  }

  const segment = html.slice(selectedIndex, exportIndex);
  return imgTags(segment)
    .map((img) => {
      const asset = imageAsset(img);
      const className = attr(img, "class");
      const selected =
        asset.kind === "perkStyle" ||
        (!className.includes("grayscale") && !className.includes("opacity-50"));
      return selected && asset.id ? asset : null;
    })
    .filter(Boolean);
}

function uniqueById(values) {
  const seen = new Set();
  return values.filter((value) => {
    const id = value.id || value.perk_id || value.spell_key || value.item_id;
    if (seen.has(id)) {
      return false;
    }
    seen.add(id);
    return true;
  });
}

function runeRows(html) {
  const runeAssets = selectedRuneAssets(html);
  const runeStyleAssets = uniqueById(runeAssets.filter((asset) => asset.kind === "perkStyle"));
  const runePerkAssets = uniqueById(runeAssets.filter((asset) => asset.kind === "perk"));

  if (runeAssets.length === 0) {
    return [];
  }

  return [
    {
      primary_style: runeStyleAssets[0]
        ? {
            style_id: Number(runeStyleAssets[0].id),
            name: runeStyleAssets[0].name,
            icon_url: runeStyleAssets[0].src
          }
        : null,
      secondary_style: runeStyleAssets[1]
        ? {
            style_id: Number(runeStyleAssets[1].id),
            name: runeStyleAssets[1].name,
            icon_url: runeStyleAssets[1].src
          }
        : null,
      perks: runePerkAssets.map((asset) => ({
        perk_id: Number(asset.id),
        name: asset.name,
        icon_url: asset.src
      }))
    }
  ];
}

function defaultsFromStats(statsPath) {
  const stats = readJson(statsPath);
  return {
    patch: stats.patch || "unknown",
    region: normalizeToken(stats.region || "global"),
    tier: normalizeToken(stats.tier || "emerald_plus"),
    queue: normalizeToken(stats.queue || "ranked_solo_duo")
  };
}

async function fetchHtml(url) {
  const response = await fetch(url, {
    headers: {
      "user-agent":
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125 Safari/537.36",
      accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
    }
  });
  const html = await response.text();
  const wafAction = response.headers.get("x-amzn-waf-action");

  if (!response.ok || wafAction) {
    throw new Error(`OP.GG request blocked or failed (${response.status}, waf=${wafAction || "none"}): ${url}`);
  }
  if (!html.includes("SummonerSpells Table") || !html.includes("SkillOrder Table")) {
    throw new Error(`OP.GG page did not include expected build tables: ${url}`);
  }

  return html;
}

function parseSnapshot(html, url, defaults) {
  const pageUrl = new URL(url);
  const pathMatch = pageUrl.pathname.match(/\/lol\/champions\/([^/]+)\/build\/?([^/?#]+)?/);
  const title = text(html.match(/<title[^>]*>([\s\S]*?)<\/title>/i)?.[1]);
  const championName = title.match(/^(.+?)\s+Build/i)?.[1] || pathMatch?.[1] || null;
  const cleanHtml = stripComments(html);

  return {
    source: "OP.GG public champion build page",
    source_url: url,
    captured_at: new Date().toISOString(),
    patch: defaults.patch,
    region: defaults.region,
    tier: defaults.tier,
    queue: defaults.queue,
    champion_key: pathMatch?.[1] || null,
    champion_name: championName,
    role: pathMatch?.[2] || null,
    runes: runeRows(cleanHtml),
    summoner_spells: spellRows(cleanHtml),
    skill_orders: skillRows(cleanHtml),
    item_builds: {
      starter: simpleBuildRows(cleanHtml, /^Items Table$/i),
      boots: simpleBuildRows(cleanHtml, /^Boots Table$/i),
      support: simpleBuildRows(cleanHtml, /^Support Table$/i),
      core: simpleBuildRows(cleanHtml, /^Builds Table$/i),
      fourth: simpleBuildRows(cleanHtml, /^Depth 4 Items Table$/i),
      fifth: simpleBuildRows(cleanHtml, /^Depth 5 Items Table$/i)
    },
    notes: [
      "Fetched from the public OP.GG page without login or challenge bypass.",
      "Validate before importing or applying any recommendation."
    ]
  };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    usage();
    return;
  }
  if (options.urls.length === 0) {
    usage();
    throw new Error("At least one OP.GG build URL is required.");
  }

  if (options.htmlPath && options.urls.length !== 1) {
    throw new Error("--html requires exactly one URL so champion and role can be inferred.");
  }

  const statsDefaults = defaultsFromStats(options.statsPath);
  const defaults = {
    ...statsDefaults,
    patch: options.patch || statsDefaults.patch,
    region: options.region || statsDefaults.region,
    tier: options.tier || statsDefaults.tier,
    queue: options.queue || statsDefaults.queue
  };
  const snapshots = [];

  for (const url of options.urls) {
    const html = options.htmlPath ? fs.readFileSync(options.htmlPath, "utf8") : await fetchHtml(url);
    snapshots.push(parseSnapshot(html, url, defaults));
  }

  const output = snapshots.length === 1 ? snapshots[0] : snapshots;
  const json = `${JSON.stringify(output, null, 2)}\n`;
  if (options.outputPath) {
    fs.mkdirSync(path.dirname(options.outputPath), { recursive: true });
    fs.writeFileSync(options.outputPath, json, "utf8");
  } else {
    process.stdout.write(json);
  }
}

main().catch((error) => {
  console.error(`error: ${error.message}`);
  process.exitCode = 1;
});
