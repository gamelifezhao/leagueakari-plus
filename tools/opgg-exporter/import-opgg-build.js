#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..");
const defaultOutputPath = path.join(
  repoRoot,
  "crates",
  "leagueakari-probe",
  "data",
  "opgg-champion-builds.sample.json"
);
const defaultTagsPath = path.join(
  repoRoot,
  "crates",
  "leagueakari-probe",
  "data",
  "champion-tags.v1.json"
);

const knownBuildCollections = ["starter", "boots", "support", "core", "fourth", "fifth"];

function usage() {
  console.log(`Usage:
  node tools/opgg-exporter/import-opgg-build.js <build-snapshot.json> [options]

Options:
  --output <path>       Output build cache path. Defaults to probe build cache.
  --tags <path>         Champion tag file used for coverage checks.
  --append              Append/update this build inside an existing cache array.
  --allow-unmatched     Warn instead of failing when champion_key misses local tags.
  --dry-run             Validate and print a summary without writing.
  -h, --help            Show this help.

Input may be "-" to read JSON from stdin.`);
}

function parseArgs(argv) {
  const options = {
    inputPath: null,
    outputPath: defaultOutputPath,
    tagsPath: defaultTagsPath,
    append: false,
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
    } else if (arg === "--append") {
      options.append = true;
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
  const row =
    rawSnapshot && typeof rawSnapshot === "object" && !Array.isArray(rawSnapshot) ? rawSnapshot : {};

  if (row !== rawSnapshot) {
    errors.push("build snapshot must be a JSON object.");
  }

  const snapshot = {
    source: optionalString(row.source) || "OP.GG public champion build page",
    source_url: optionalString(row.source_url) || null,
    captured_at: optionalString(row.captured_at) || new Date().toISOString(),
    patch: requiredString(row.patch, "patch", errors),
    region: normalizeToken(requiredString(row.region, "region", errors)),
    tier: normalizeToken(requiredString(row.tier, "tier", errors)),
    queue: normalizeToken(requiredString(row.queue, "queue", errors)),
    champion_key: normalizeChampionKey(requiredString(row.champion_key, "champion_key", errors)),
    champion_name: requiredString(row.champion_name, "champion_name", errors),
    role: normalizeRole(requiredString(row.role, "role", errors), "role", errors),
    runes: normalizeRunes(row.runes, errors),
    summoner_spells: normalizeSpellBuilds(row.summoner_spells, errors),
    skill_orders: normalizeSkillOrders(row.skill_orders, errors),
    item_builds: normalizeItemBuilds(row.item_builds, errors),
    notes: Array.isArray(row.notes) ? row.notes.map((note) => optionalString(note)).filter(Boolean) : []
  };

  return { snapshot, errors, warnings };
}

function normalizeInput(rawInput) {
  if (Array.isArray(rawInput)) {
    const snapshots = rawInput.map((entry) => normalizeSnapshot(entry));
    return {
      snapshots: snapshots.map((entry) => entry.snapshot),
      errors: snapshots.flatMap((entry) => entry.errors),
      warnings: snapshots.flatMap((entry) => entry.warnings)
    };
  }

  const normalized = normalizeSnapshot(rawInput);
  return {
    snapshots: [normalized.snapshot],
    errors: normalized.errors,
    warnings: normalized.warnings
  };
}

function normalizeRunes(value, errors) {
  if (value === undefined || value === null) {
    return [];
  }
  if (!Array.isArray(value)) {
    errors.push("runes must be an array.");
    return [];
  }

  return value.map((rune, index) => {
    const row = rune && typeof rune === "object" ? rune : {};
    if (row !== rune) {
      errors.push(`runes[${index}] must be an object.`);
    }

    return {
      primary_style: normalizeStyle(row.primary_style, `runes[${index}].primary_style`, errors),
      secondary_style: normalizeStyle(row.secondary_style, `runes[${index}].secondary_style`, errors),
      perks: Array.isArray(row.perks)
        ? row.perks.map((perk, perkIndex) =>
            normalizePerk(perk, `runes[${index}].perks[${perkIndex}]`, errors)
          )
        : []
    };
  });
}

function normalizeStyle(value, field, errors) {
  if (value === undefined || value === null) {
    errors.push(`${field} is required.`);
    return null;
  }
  const row = value && typeof value === "object" ? value : {};
  if (row !== value) {
    errors.push(`${field} must be an object.`);
  }
  return {
    style_id: positiveInteger(row.style_id, `${field}.style_id`, errors),
    name: requiredString(row.name, `${field}.name`, errors),
    icon_url: optionalString(row.icon_url) || null
  };
}

function normalizePerk(value, field, errors) {
  const row = value && typeof value === "object" ? value : {};
  if (row !== value) {
    errors.push(`${field} must be an object.`);
  }
  return {
    perk_id: positiveInteger(row.perk_id, `${field}.perk_id`, errors),
    name: requiredString(row.name, `${field}.name`, errors),
    icon_url: optionalString(row.icon_url) || null
  };
}

function normalizeSpellBuilds(value, errors) {
  if (value === undefined || value === null) {
    return [];
  }
  if (!Array.isArray(value)) {
    errors.push("summoner_spells must be an array.");
    return [];
  }

  return value.map((build, index) => {
    const row = build && typeof build === "object" ? build : {};
    if (row !== build) {
      errors.push(`summoner_spells[${index}] must be an object.`);
    }
    return {
      spells: Array.isArray(row.spells)
        ? row.spells.map((spell, spellIndex) =>
            normalizeSpell(spell, `summoner_spells[${index}].spells[${spellIndex}]`, errors)
          )
        : [],
      pick_rate: rate(row.pick_rate, `summoner_spells[${index}].pick_rate`, errors),
      games: optionalPositiveInteger(row.games, `summoner_spells[${index}].games`, errors),
      win_rate: rate(row.win_rate, `summoner_spells[${index}].win_rate`, errors),
      raw_text: optionalString(row.raw_text)
    };
  });
}

function normalizeSpell(value, field, errors) {
  const row = value && typeof value === "object" ? value : {};
  if (row !== value) {
    errors.push(`${field} must be an object.`);
  }
  return {
    spell_key: requiredString(row.spell_key, `${field}.spell_key`, errors),
    name: requiredString(row.name, `${field}.name`, errors),
    icon_url: optionalString(row.icon_url) || null
  };
}

function normalizeSkillOrders(value, errors) {
  if (value === undefined || value === null) {
    return [];
  }
  if (!Array.isArray(value)) {
    errors.push("skill_orders must be an array.");
    return [];
  }

  return value.map((build, index) => {
    const row = build && typeof build === "object" ? build : {};
    if (row !== build) {
      errors.push(`skill_orders[${index}] must be an object.`);
    }
    const order = requiredString(row.order, `skill_orders[${index}].order`, errors).toUpperCase();
    if (order && !/^[QWER]+$/.test(order)) {
      errors.push(`skill_orders[${index}].order must only contain Q/W/E/R.`);
    }

    return {
      order,
      skills: Array.isArray(row.skills)
        ? row.skills.map((skill, skillIndex) =>
            normalizeSpell(skill, `skill_orders[${index}].skills[${skillIndex}]`, errors)
          )
        : [],
      pick_rate: rate(row.pick_rate, `skill_orders[${index}].pick_rate`, errors),
      games: optionalPositiveInteger(row.games, `skill_orders[${index}].games`, errors),
      win_rate: rate(row.win_rate, `skill_orders[${index}].win_rate`, errors),
      raw_text: optionalString(row.raw_text)
    };
  });
}

function normalizeItemBuilds(value, errors) {
  const row = value && typeof value === "object" && !Array.isArray(value) ? value : {};
  if (value === undefined || value === null) {
    return Object.fromEntries(knownBuildCollections.map((key) => [key, []]));
  }
  if (row !== value) {
    errors.push("item_builds must be an object.");
  }

  return Object.fromEntries(
    knownBuildCollections.map((collection) => [
      collection,
      normalizeItemRows(row[collection], `item_builds.${collection}`, errors)
    ])
  );
}

function normalizeItemRows(value, field, errors) {
  if (value === undefined || value === null) {
    return [];
  }
  if (!Array.isArray(value)) {
    errors.push(`${field} must be an array.`);
    return [];
  }

  return value.map((build, index) => {
    const row = build && typeof build === "object" ? build : {};
    if (row !== build) {
      errors.push(`${field}[${index}] must be an object.`);
    }

    return {
      items: Array.isArray(row.items)
        ? row.items.map((item, itemIndex) =>
            normalizeItem(item, `${field}[${index}].items[${itemIndex}]`, errors)
          )
        : [],
      pick_rate: rate(row.pick_rate, `${field}[${index}].pick_rate`, errors),
      games: optionalPositiveInteger(row.games, `${field}[${index}].games`, errors),
      win_rate: rate(row.win_rate, `${field}[${index}].win_rate`, errors),
      raw_text: optionalString(row.raw_text)
    };
  });
}

function normalizeItem(value, field, errors) {
  const row = value && typeof value === "object" ? value : {};
  if (row !== value) {
    errors.push(`${field} must be an object.`);
  }
  return {
    item_id: positiveInteger(row.item_id, `${field}.item_id`, errors),
    name: optionalString(row.name) || null,
    icon_url: optionalString(row.icon_url) || null
  };
}

function validateSnapshot(snapshot, tagKeys, options) {
  const errors = [];
  const warnings = [];

  if (!tagKeys.has(snapshot.champion_key)) {
    const message = `${snapshot.champion_key} is missing from local champion tags.`;
    if (options.allowUnmatched) {
      warnings.push(message);
    } else {
      errors.push(message);
    }
  }
  if (snapshot.runes.length === 0) {
    warnings.push("build snapshot has no rune recommendations.");
  }
  if (snapshot.summoner_spells.length === 0) {
    warnings.push("build snapshot has no summoner spell recommendations.");
  }
  if (snapshot.skill_orders.length === 0) {
    warnings.push("build snapshot has no skill order recommendations.");
  }
  if (snapshot.item_builds.core.length === 0) {
    warnings.push("build snapshot has no core item recommendations.");
  }

  return { errors, warnings };
}

function validateSnapshots(snapshots, tagKeys, options) {
  const reports = snapshots.map((snapshot) => validateSnapshot(snapshot, tagKeys, options));
  const duplicateKeys = duplicates(snapshots.map(buildKey));
  const errors = reports.flatMap((report) => report.errors);
  const warnings = reports.flatMap((report) => report.warnings);

  if (duplicateKeys.length > 0) {
    errors.push(`duplicate build keys: ${duplicateKeys.join(", ")}`);
  }

  return { errors, warnings, duplicateKeys };
}

function buildKey(snapshot) {
  return `${snapshot.champion_key}:${snapshot.role}:${snapshot.patch}:${snapshot.region}:${snapshot.tier}:${snapshot.queue}`;
}

function mergeSnapshots(existing, snapshots) {
  const byKey = new Map();

  for (const snapshot of existing) {
    byKey.set(buildKey(snapshot), snapshot);
  }
  for (const snapshot of snapshots) {
    byKey.set(buildKey(snapshot), snapshot);
  }

  return Array.from(byKey.values()).sort((left, right) =>
    buildKey(left).localeCompare(buildKey(right))
  );
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
  const normalized = normalizeToken(value);
  const aliases = {
    adc: "adc",
    bottom: "adc",
    bot: "adc",
    support: "support",
    utility: "support",
    mid: "mid",
    middle: "mid",
    top: "top",
    jungle: "jungle",
    jg: "jungle"
  };
  const role = aliases[normalized];
  if (!role) {
    errors.push(`${field} has unsupported role: ${value}`);
    return normalized;
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

function optionalPositiveInteger(value, field, errors) {
  if (value === undefined || value === null || value === "") {
    return null;
  }
  return positiveInteger(value, field, errors);
}

function rate(value, field, errors) {
  if (value === undefined || value === null || value === "") {
    return null;
  }

  const parsed = Number(String(value).replace("%", "").trim());
  if (!Number.isFinite(parsed) || parsed < 0 || parsed > 100) {
    errors.push(`${field} must be between 0 and 100.`);
  }
  return Math.round(parsed * 100) / 100;
}

function printReport(report) {
  console.log("OP.GG build import report:");
  console.log(`  builds: ${report.snapshots.length}`);
  console.log(`  keys: ${report.snapshots.map(buildKey).join(", ") || "none"}`);
  console.log(
    `  runes: ${report.snapshots.reduce((sum, snapshot) => sum + snapshot.runes.length, 0)}`
  );
  console.log(
    `  summoner spell builds: ${report.snapshots.reduce((sum, snapshot) => sum + snapshot.summoner_spells.length, 0)}`
  );
  console.log(
    `  skill orders: ${report.snapshots.reduce((sum, snapshot) => sum + snapshot.skill_orders.length, 0)}`
  );
  console.log(
    `  core item builds: ${report.snapshots.reduce((sum, snapshot) => sum + snapshot.item_builds.core.length, 0)}`
  );
  console.log(`  output: ${report.outputPath}`);
  console.log(`  append: ${report.append}`);
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
    throw new Error("Missing build snapshot input path.");
  }

  const tagKeys = loadTagKeys(options.tagsPath);
  const { snapshots, errors: normalizeErrors, warnings: normalizeWarnings } = normalizeInput(
    readJson(options.inputPath)
  );
  const validation = validateSnapshots(snapshots, tagKeys, options);
  const errors = [...normalizeErrors, ...validation.errors];
  const warnings = [...normalizeWarnings, ...validation.warnings];

  printReport({
    snapshots,
    outputPath: options.outputPath,
    append: options.append,
    dryRun: options.dryRun,
    warnings
  });

  if (errors.length > 0) {
    fail(errors);
    return;
  }

  if (!options.dryRun) {
    const output = options.append
      ? mergeSnapshots(
          fs.existsSync(options.outputPath) ? [].concat(readJson(options.outputPath)) : [],
          snapshots
        )
      : snapshots;
    fs.mkdirSync(path.dirname(options.outputPath), { recursive: true });
    fs.writeFileSync(options.outputPath, `${JSON.stringify(output, null, 2)}\n`, "utf8");
  }
}

try {
  main();
} catch (error) {
  console.error(`error: ${error.message}`);
  process.exitCode = 1;
}
