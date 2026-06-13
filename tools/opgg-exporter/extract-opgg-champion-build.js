(() => {
  const text = (value) =>
    String(value || "")
      .trim()
      .replace(/\s+/g, " ");

  const parseRate = (value) => {
    const match = text(value).match(/([0-9]+(?:\.[0-9]+)?)%/);
    return match ? Number(match[1]) : null;
  };

  const parseGames = (value) => {
    const match = text(value).match(/([0-9,]+)\s*Games?/i);
    return match ? Number(match[1].replace(/,/g, "")) : null;
  };

  const parseMetricCell = (value) => {
    const content = text(value);
    const explicitRate = content.match(/([0-9]+(?:\.[0-9]+)?)%/);

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

    return {
      rate: parseRate(content),
      games: parseGames(content)
    };
  };

  const normalizeToken = (value) =>
    text(value)
      .toLowerCase()
      .replace(/\s*\+\s*/g, "_plus")
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "");

  const imageAsset = (img) => {
    const src = img?.src || "";
    const match = src.match(/\/(spell|summonerSpell|perkStyle|perk|item)\/([^/?]+)\./i);
    return {
      id: match?.[2] || null,
      kind: match?.[1] || null,
      name: text(img?.alt) || null,
      src
    };
  };

  const tableRows = (captionPattern) =>
    Array.from(document.querySelectorAll("table"))
      .filter((table) => captionPattern.test(text(table.querySelector("caption")?.textContent)))
      .flatMap((table) =>
        Array.from(table.querySelectorAll("tbody tr")).map((row) => {
          const cells = Array.from(row.querySelectorAll("td,th"));
          const assetText = text(cells[0]?.textContent);
          const assets = Array.from(cells[0]?.querySelectorAll("img") || [])
            .map(imageAsset)
            .filter((asset) => asset.id);
          const pickText = text(cells[1]?.textContent);
          const winText = text(cells[2]?.textContent || cells[1]?.textContent);
          const pickMetric = parseMetricCell(pickText);

          return {
            assets,
            asset_text: assetText,
            pick_rate: pickMetric.rate,
            games: pickMetric.games,
            win_rate: parseRate(winText),
            raw_text: text(row.textContent)
          };
        })
      );

  const simpleBuildRows = (captionPattern) =>
    tableRows(captionPattern).map((row) => ({
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

  const spellRows = tableRows(/summoner/i).map((row) => ({
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

  const skillRows = tableRows(/skillorder/i).map((row) => ({
    order: text(row.raw_text.match(/[QWER]{6,}/)?.[0] || ""),
    skills: row.assets.map((asset) => ({
      spell_key: asset.id,
      name: asset.name,
      icon_url: asset.src
    })),
    pick_rate: row.pick_rate,
    games: row.games,
    win_rate: row.win_rate,
    raw_text: row.raw_text
  }));

  const selectedRuneAssets = () => {
    const exportButton = Array.from(document.querySelectorAll("button,a")).find((element) =>
      /Export rune build/i.test(text(element.textContent))
    );
    const container = exportButton?.parentElement;
    if (!container) {
      return [];
    }

    return Array.from(container.querySelectorAll("img"))
      .map((img) => {
        const asset = imageAsset(img);
        const style = getComputedStyle(img);
        const selected =
          asset.kind === "perkStyle" ||
          (!String(img.className || "").includes("grayscale") &&
            style.opacity !== "0.5" &&
            style.filter !== "grayscale(1)");
        return selected && asset.id ? asset : null;
      })
      .filter(Boolean);
  };

  const uniqueById = (assets) => {
    const seen = new Set();
    return assets.filter((asset) => {
      if (seen.has(asset.id)) {
        return false;
      }
      seen.add(asset.id);
      return true;
    });
  };

  const runeAssets = selectedRuneAssets();
  const runeStyleAssets = uniqueById(runeAssets.filter((asset) => asset.kind === "perkStyle"));
  const runePerkAssets = uniqueById(runeAssets.filter((asset) => asset.kind === "perk"));
  const url = new URL(location.href);
  const pathMatch = url.pathname.match(/\/lol\/champions\/([^/]+)\/build\/?([^/?#]+)?/);
  const heading = text(document.querySelector("h1")?.textContent);
  const patchMatch =
    heading.match(/Patch\s+([0-9.]+)/i) ||
    text(document.body?.innerText).match(/Version:\s*([0-9.]+)/i);
  const championName =
    heading.match(/^(.+?)\s+Build/i)?.[1] ||
    text(document.querySelector('img[alt]:not([alt=""])')?.alt);
  const tierLabel = Array.from(document.querySelectorAll("img[alt]"))
    .map((img) => text(img.alt))
    .find((alt) => /iron|bronze|silver|gold|platinum|emerald|diamond|master|grandmaster|challenger/i.test(alt));

  const snapshot = {
    source: "OP.GG public champion build page",
    source_url: location.href,
    captured_at: new Date().toISOString(),
    patch: patchMatch?.[1] || "unknown",
    region: url.searchParams.get("region") || "global",
    tier: tierLabel ? normalizeToken(tierLabel) : "unknown",
    queue: "ranked_solo_duo",
    champion_key: pathMatch?.[1] || null,
    champion_name: championName || null,
    role: pathMatch?.[2] || null,
    runes: runeAssets.length
      ? [
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
        ]
      : [],
    summoner_spells: spellRows,
    skill_orders: skillRows,
    item_builds: {
      starter: simpleBuildRows(/^Items Table$/i),
      boots: simpleBuildRows(/^Boots Table$/i),
      support: simpleBuildRows(/^Support Table$/i),
      core: simpleBuildRows(/^Builds Table$/i),
      fourth: simpleBuildRows(/^Depth 4 Items Table$/i),
      fifth: simpleBuildRows(/^Depth 5 Items Table$/i)
    },
    notes: [
      "Extracted from the already-open public OP.GG page.",
      "Rune export is based on visible selected icons and should be validated before applying."
    ]
  };

  copy(JSON.stringify(snapshot, null, 2));
  console.log("LeagueAkari OP.GG champion build snapshot copied", snapshot);
})();
