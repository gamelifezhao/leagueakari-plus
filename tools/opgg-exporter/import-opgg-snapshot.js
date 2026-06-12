#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..");
const defaultOutputPath = path.join(
  repoRoot,
  "crates",
  "leagueakari-probe",
  "data",
  "opgg-champion-stats.sample.json"
);
const defaultTagsPath = path.join(
  repoRoot,
  "crates",
  "leagueakari-probe",
  "data",
  "champion-tags.v1.json"
);

const roleAliases = new Map([
  ["adc", "adc"],
  ["bottom", "adc"],
  ["bot", "adc"],
  ["support", "support"],
  ["utility", "support"],
  ["mid", "mid"],
  ["middle", "mid"],
  ["top", "top"],
  ["jungle", "jungle"],
  ["jg", "jungle"]
]);

function usage() {
  console.log(`Usage:
  node tools/opgg-exporter/import-opgg-snapshot.js <snapshot.json> [options]

Options:
  --output <path>       Output cache path. Defaults to the probe OP.GG cache.
  --tags <path>         Champion tag file used for coverage checks.
  --min-entries <n>     Minimum accepted entries. Default: 40.
  --allow-unmatched     Warn instead of failing when entries miss local tags.
  --dry-run             Validate and print a summary without writing.
  -h, --help            Show this help.

Input may be "-" to read JSON from stdin.`);
}

function parseArgs(argv) {
  const options = {
    inputPath: null,
    outputPath: defaultOutputPath,
    tagsPath: defaultTagsPath,
    minEntries: 40,
    allowUnmatched: false,
    dryRun: false
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "-h" || arg === "--help") {
      options.help = true;
    } else if (arg === "--output") {
      options.outputPath = requireValue(argv, (index += 1), "--output");
    } else if (arg === "--tags") {
      options.tagsPath = requireValue(argv, (index += 1), "--tags");
    } else if (arg === "--min-entries") {
      options.minEntries = Number(requireValue(argv, (index += 1), "--min-entries"));
    } else if (arg === "--allow-unmatched") {
      options.allowUnmatched = true;
    } else if (arg === "--dry-run") {
      options.dryRun = true;
    } else if (arg === "-" && !options.inputPath) {
      options.inputPath = arg;
    } else if (arg.startsWith("-")) {
      throw new Error(`Unknown option: ${arg}`);
    } else if (!options.inputPath) {
      options.inputPath = arg;
    } else {
      throw new Error(`Unexpected argument: ${arg}`);
    }
  }

  if (!Number.isInteger(options.minEntries) || options.minEntries < 1) {
    throw new Error("--min-entries must be a positive integer.");
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
  const text = filePath === "-" ? fs.readFileSync(0, "utf8") : fs.readFileSync(filePath, "utf8");
  return JSON.parse(text);
}

function normalizeSnapshot(rawSnapshot) {
  const errors = [];
  const warnings = [];
  const snapshotObject =
    rawSnapshot && typeof rawSnapshot === "object" && !Array.isArray(rawSnapshot) ? rawSnapshot : {};
  const rawEntries = Array.isArray(snapshotObject.entries) ? snapshotObject.entries : [];

  if (snapshotObject !== rawSnapshot) {
    errors.push("snapshot must be a JSON object.");
  }
  if (!Array.isArray(snapshotObject.entries)) {
    errors.push("snapshot.entries must be an array.");
  }

  const entries = rawEntries
    .map((entry, index) => normalizeEntry(entry, index, errors, warnings))
    .sort((left, right) => left.rank - right.rank);

  const snapshot = {
    source: optionalString(snapshotObject.source) || "OP.GG public champion tier list",
    source_url: optionalString(snapshotObject.source_url) || null,
    captured_at: optionalString(snapshotObject.captured_at) || new Date().toISOString(),
    patch: requiredString(snapshotObject.patch, "patch", errors),
    region: normalizeToken(requiredString(snapshotObject.region, "region", errors)),
    tier: normalizeToken(requiredString(snapshotObject.tier, "tier", errors)),
    queue: normalizeToken(requiredString(snapshotObject.queue, "queue", errors)),
    last_updated_label: optionalString(snapshotObject.last_updated_label) || null,
    sample_count: optionalInteger(snapshotObject.sample_count, "sample_count", errors),
    entries
  };

  return { snapshot, errors, warnings };
}

function normalizeEntry(entry, index, errors, warnings) {
  const row = entry && typeof entry === "object" ? entry : {};
  const field = (name) => `entries[${index}].${name}`;
  const championKey = normalizeChampionKey(requiredString(row.champion_key, field("champion_key"), errors));
  const role = normalizeRole(requiredString(row.role, field("role"), errors), field("role"), errors);

  if (row !== entry) {
    errors.push(`entries[${index}] must be an object.`);
  }
  if (row.champion_key && championKey !== String(row.champion_key).trim().toLowerCase()) {
    warnings.push(`${field("champion_key")} normalized to ${championKey}.`);
  }

  return {
    rank: positiveInteger(row.rank, field("rank"), errors),
    champion_key: championKey,
    champion_name: requiredString(row.champion_name, field("champion_name"), errors),
    role,
    win_rate: rate(row.win_rate, field("win_rate"), errors),
    pick_rate: rate(row.pick_rate, field("pick_rate"), errors),
    ban_rate: rate(row.ban_rate, field("ban_rate"), errors)
  };
}

function requiredString(value, field, errors) {
  const normalized = optionalString(value);
  if (!normalized) {
    errors.push(`${field} is required.`);
  }
  return normalized;
}

function optionalString(value) {
  if (value === undefined || value === null) {
    return "";
  }
  return String(value).trim();
}

function normalizeToken(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/\s*\+\s*/g, "_plus")
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function normalizeChampionKey(value) {
  return String(value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

function normalizeRole(value, field, errors) {
  const key = normalizeToken(value);
  const role = roleAliases.get(key);
  if (!role) {
    errors.push(`${field} has unsupported role: ${value}`);
    return key;
  }
  return role;
}

function positiveInteger(value, field, errors) {
  const number = Number(value);
  if (!Number.isInteger(number) || number < 1) {
    errors.push(`${field} must be a positive integer.`);
  }
  return number;
}

function optionalInteger(value, field, errors) {
  if (value === undefined || value === null || value === "") {
    return null;
  }

  const number = Number(String(value).replace(/,/g, ""));
  if (!Number.isInteger(number) || number < 0) {
    errors.push(`${field} must be a non-negative integer or null.`);
  }
  return number;
}

function rate(value, field, errors) {
  const parsed = Number(String(value).replace("%", "").trim());
  if (!Number.isFinite(parsed) || parsed < 0 || parsed > 100) {
    errors.push(`${field} must be between 0 and 100.`);
  }
  return Math.round(parsed * 100) / 100;
}

function validateSnapshot(snapshot, tagKeys, options) {
  const errors = [];
  const warnings = [];

  if (snapshot.entries.length < options.minEntries) {
    errors.push(
      `snapshot has ${snapshot.entries.length} entries, below --min-entries ${options.minEntries}.`
    );
  }

  const duplicateStatKeys = duplicates(
    snapshot.entries.map((entry) => `${entry.champion_key}:${entry.role}`)
  );
  if (duplicateStatKeys.length > 0) {
    errors.push(`duplicate champion-role stat keys: ${duplicateStatKeys.join(", ")}`);
  }

  const unmatched = snapshot.entries
    .filter((entry) => !tagKeys.has(entry.champion_key))
    .map((entry) => `${entry.champion_key}:${entry.role}`);

  if (unmatched.length > 0) {
    const message = `OP.GG entries missing local champion tags: ${unmatched.join(", ")}`;
    if (options.allowUnmatched) {
      warnings.push(message);
    } else {
      errors.push(message);
    }
  }

  return { errors, warnings, unmatched, duplicateStatKeys };
}

function duplicates(values) {
  const seen = new Set();
  const duplicated = new Set();

  for (const value of values) {
    if (seen.has(value)) {
      duplicated.add(value);
    }
    seen.add(value);
  }

  return Array.from(duplicated).sort();
}

function loadTagKeys(tagsPath) {
  const tags = readJson(tagsPath);
  if (!Array.isArray(tags)) {
    throw new Error(`Champion tag file must be an array: ${tagsPath}`);
  }

  return new Set(
    tags
      .map((tag) => normalizeChampionKey(tag.champion_key))
      .filter((championKey) => championKey.length > 0)
  );
}

function printReport(report) {
  console.log("OP.GG snapshot import report:");
  console.log(`  patch: ${report.snapshot.patch}`);
  console.log(`  region: ${report.snapshot.region}`);
  console.log(`  tier: ${report.snapshot.tier}`);
  console.log(`  queue: ${report.snapshot.queue}`);
  console.log(`  entries: ${report.snapshot.entries.length}`);
  console.log(`  unmatched local tags: ${report.unmatched.length}`);
  console.log(`  duplicate stat keys: ${report.duplicateStatKeys.length}`);
  console.log(`  output: ${report.outputPath}`);
  console.log(`  dry_run: ${report.dryRun}`);

  for (const warning of report.warnings) {
    console.warn(`warning: ${warning}`);
  }
}

function fail(errors) {
  for (const error of errors) {
    console.error(`error: ${error}`);
  }
  process.exitCode = 1;
}

function main() {
  const options = parseArgs(process.argv.slice(2));

  if (options.help) {
    usage();
    return;
  }

  if (!options.inputPath) {
    usage();
    throw new Error("Missing snapshot input path.");
  }

  const tagKeys = loadTagKeys(options.tagsPath);
  const { snapshot, errors: normalizeErrors, warnings: normalizeWarnings } = normalizeSnapshot(
    readJson(options.inputPath)
  );
  const validation = validateSnapshot(snapshot, tagKeys, options);
  const errors = [...normalizeErrors, ...validation.errors];
  const warnings = [...normalizeWarnings, ...validation.warnings];

  printReport({
    snapshot,
    unmatched: validation.unmatched,
    duplicateStatKeys: validation.duplicateStatKeys,
    outputPath: options.outputPath,
    dryRun: options.dryRun,
    warnings
  });

  if (errors.length > 0) {
    fail(errors);
    return;
  }

  if (!options.dryRun) {
    fs.mkdirSync(path.dirname(options.outputPath), { recursive: true });
    fs.writeFileSync(options.outputPath, `${JSON.stringify(snapshot, null, 2)}\n`, "utf8");
  }
}

try {
  main();
} catch (error) {
  console.error(`error: ${error.message}`);
  process.exitCode = 1;
}
