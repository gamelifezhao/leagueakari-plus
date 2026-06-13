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
const defaultBuildsPath = path.join(
  repoRoot,
  "crates",
  "leagueakari-probe",
  "data",
  "opgg-champion-builds.sample.json"
);
const defaultTop = 15;

function usage() {
  console.log(`Usage:
  node tools/opgg-exporter/build-coverage.js [options]

Options:
  --stats <path>         OP.GG champion stat snapshot path.
  --builds <path>        OP.GG champion build snapshot path.
  --top <number>         Number of missing build URLs to print. Defaults to ${defaultTop}.
  --role <role>          Filter missing entries to one role: top, jungle, mid, adc, support.
  --min-pick-rate <n>    Filter missing entries by minimum pick rate.
  --json                 Print machine-readable JSON.
  -h, --help             Show this help.

The tool is offline-only. It never requests OP.GG; it only reports which public pages to open manually.`);
}

function parseArgs(argv) {
  const options = {
    statsPath: defaultStatsPath,
    buildsPath: defaultBuildsPath,
    top: defaultTop,
    role: null,
    minPickRate: null,
    json: false,
    help: false
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "-h" || arg === "--help") {
      options.help = true;
    } else if (arg === "--stats") {
      options.statsPath = requireValue(argv, (index += 1), "--stats");
    } else if (arg === "--builds") {
      options.buildsPath = requireValue(argv, (index += 1), "--builds");
    } else if (arg === "--top") {
      options.top = positiveInteger(requireValue(argv, (index += 1), "--top"), "--top");
    } else if (arg === "--role") {
      options.role = normalizeRole(requireValue(argv, (index += 1), "--role"));
    } else if (arg === "--min-pick-rate") {
      options.minPickRate = nonNegativeNumber(
        requireValue(argv, (index += 1), "--min-pick-rate"),
        "--min-pick-rate"
      );
    } else if (arg === "--json") {
      options.json = true;
    } else {
      throw new Error(`Unknown option: ${arg}`);
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

function positiveInteger(value, field) {
  const number = Number(value);
  if (!Number.isInteger(number) || number < 1) {
    throw new Error(`${field} must be a positive integer.`);
  }
  return number;
}

function nonNegativeNumber(value, field) {
  const number = Number(value);
  if (!Number.isFinite(number) || number < 0) {
    throw new Error(`${field} must be a non-negative number.`);
  }
  return number;
}

function normalizeRole(value) {
  const normalized = String(value || "").trim().toLowerCase();
  const aliases = {
    bottom: "adc",
    bot: "adc",
    adc: "adc",
    utility: "support",
    support: "support",
    mid: "mid",
    middle: "mid",
    top: "top",
    jungle: "jungle",
    jg: "jungle"
  };
  const role = aliases[normalized];
  if (!role) {
    throw new Error(`Unsupported role: ${value}`);
  }
  return role;
}

function contextKey(row) {
  return [row.patch, row.region, row.tier, row.queue].map((value) => value || "unknown").join(":");
}

function buildKey(row) {
  return `${normalizeChampionKey(row.champion_key)}:${normalizeRole(row.role)}`;
}

function normalizeChampionKey(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

function buildUrl(row) {
  return `https://op.gg/lol/champions/${normalizeChampionKey(row.champion_key)}/build/${normalizeRole(row.role)}`;
}

function priorityScore(row) {
  const rankWeight = Math.max(0, 60 - Number(row.rank || 999)) * 1.2;
  return round2(rankWeight + Number(row.pick_rate || 0) * 2 + Number(row.ban_rate || 0));
}

function round2(value) {
  return Math.round(value * 100) / 100;
}

function coverageReport(options) {
  const stats = readJson(options.statsPath);
  const builds = readJson(options.buildsPath);
  const entries = Array.isArray(stats.entries) ? stats.entries : [];
  const buildRows = Array.isArray(builds) ? builds : [];
  const buildByKey = new Map(buildRows.map((build) => [buildKey(build), build]));
  const statKeys = new Set(entries.map(buildKey));
  const statsContext = contextKey(stats);

  const coveredEntries = entries.filter((entry) => buildByKey.has(buildKey(entry)));
  const exactContextCoveredEntries = coveredEntries.filter(
    (entry) => contextKey(buildByKey.get(buildKey(entry))) === statsContext
  );
  const missingEntries = entries
    .filter((entry) => !buildByKey.has(buildKey(entry)))
    .filter((entry) => (options.role ? normalizeRole(entry.role) === options.role : true))
    .filter((entry) =>
      options.minPickRate === null ? true : Number(entry.pick_rate || 0) >= options.minPickRate
    )
    .map((entry) => ({
      rank: Number(entry.rank),
      champion_key: normalizeChampionKey(entry.champion_key),
      champion_name: String(entry.champion_name || entry.champion_key || ""),
      role: normalizeRole(entry.role),
      win_rate: Number(entry.win_rate),
      pick_rate: Number(entry.pick_rate),
      ban_rate: Number(entry.ban_rate),
      priority_score: priorityScore(entry),
      url: buildUrl(entry)
    }))
    .sort((left, right) => {
      if (left.rank !== right.rank) {
        return left.rank - right.rank;
      }
      return right.priority_score - left.priority_score;
    });

  const staleBuilds = buildRows
    .filter((build) => statKeys.has(buildKey(build)) && contextKey(build) !== statsContext)
    .map((build) => ({
      champion_key: normalizeChampionKey(build.champion_key),
      role: normalizeRole(build.role),
      build_context: contextKey(build),
      stats_context: statsContext,
      url: buildUrl(build)
    }));
  const orphanBuilds = buildRows
    .filter((build) => !statKeys.has(buildKey(build)))
    .map((build) => ({
      champion_key: normalizeChampionKey(build.champion_key),
      role: normalizeRole(build.role),
      url: buildUrl(build)
    }));

  return {
    stats: {
      path: options.statsPath,
      patch: stats.patch || "unknown",
      region: stats.region || "unknown",
      tier: stats.tier || "unknown",
      queue: stats.queue || "unknown",
      context: statsContext,
      entries: entries.length
    },
    builds: {
      path: options.buildsPath,
      entries: buildRows.length
    },
    coverage: {
      covered_entries: coveredEntries.length,
      covered_percent: entries.length ? round2((coveredEntries.length / entries.length) * 100) : 0,
      exact_context_entries: exactContextCoveredEntries.length,
      exact_context_percent: entries.length
        ? round2((exactContextCoveredEntries.length / entries.length) * 100)
        : 0,
      missing_entries: entries.length - coveredEntries.length,
      filtered_missing_entries: missingEntries.length,
      stale_builds: staleBuilds.length,
      orphan_builds: orphanBuilds.length
    },
    top_missing: missingEntries.slice(0, options.top),
    stale_builds: staleBuilds,
    orphan_builds: orphanBuilds
  };
}

function printText(report) {
  console.log("OP.GG build coverage:");
  console.log(
    `  stats context: ${report.stats.patch} / ${report.stats.region} / ${report.stats.tier} / ${report.stats.queue}`
  );
  console.log(`  stat entries: ${report.stats.entries}`);
  console.log(`  build snapshots: ${report.builds.entries}`);
  console.log(
    `  covered stat entries: ${report.coverage.covered_entries} (${report.coverage.covered_percent}%)`
  );
  console.log(
    `  exact-context covered: ${report.coverage.exact_context_entries} (${report.coverage.exact_context_percent}%)`
  );
  console.log(`  missing stat entries: ${report.coverage.missing_entries}`);
  console.log(`  filtered missing entries: ${report.coverage.filtered_missing_entries}`);
  console.log(`  stale build snapshots: ${report.coverage.stale_builds}`);
  console.log(`  orphan build snapshots: ${report.coverage.orphan_builds}`);
  console.log("");

  if (report.top_missing.length === 0) {
    console.log("Top missing build pages: none");
    return;
  }

  console.log("Top missing build pages:");
  for (const entry of report.top_missing) {
    console.log(
      `  #${entry.rank} ${entry.champion_name} ${entry.role} | win ${entry.win_rate.toFixed(2)}% | pick ${entry.pick_rate.toFixed(2)}% | ban ${entry.ban_rate.toFixed(2)}%`
    );
    console.log(`     ${entry.url}`);
  }
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    usage();
    return;
  }

  const report = coverageReport(options);
  if (options.json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    printText(report);
  }
}

try {
  main();
} catch (error) {
  console.error(`error: ${error.message}`);
  process.exitCode = 1;
}
