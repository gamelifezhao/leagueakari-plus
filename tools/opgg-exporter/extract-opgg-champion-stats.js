(() => {
  const parseRate = (text) => {
    const match = String(text || "").match(/([0-9]+(?:\.[0-9]+)?)%/);
    return match ? Number(match[1]) : null;
  };

  const normalizeQueue = (value) =>
    String(value || "")
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "");

  const bodyText = document.body?.innerText ?? "";
  const heading = document.querySelector("h1")?.textContent?.trim() ?? "";
  const sampleMatch = bodyText.match(/Total analyzed samples\s*:\s*([0-9,]+)/i);
  const lastUpdatedMatch = bodyText.match(/Last updated:\s*([^\n]+)/i);
  const patchMatch = heading.match(/Patch\s+([0-9.]+)/i) || bodyText.match(/Version:\s*([0-9.]+)/i);
  const region = new URL(location.href).searchParams.get("region") || "global";
  const tier = (heading.match(/,\s*([^,]+),\s*Patch/i)?.[1] ?? "Emerald +")
    .toLowerCase()
    .replace(/\s*\+\s*/g, "_plus")
    .replace(/\s+/g, "_");

  const entries = Array.from(document.querySelectorAll("table tbody tr"))
    .map((row) => {
      const cells = Array.from(row.querySelectorAll("td"));
      const link = row.querySelector('a[href*="/lol/champions/"]');
      const href = link?.getAttribute("href") || "";
      const match = href.match(/\/lol\/champions\/([^/]+)\/build\/?([^/?#]+)?/);
      const texts = cells.map((cell) => cell.textContent.trim().replace(/\s+/g, " "));

      return {
        rank: Number(texts[0]),
        champion_key: match?.[1] || null,
        champion_name: texts[1] || link?.textContent?.trim() || null,
        role: match?.[2] || null,
        win_rate: parseRate(texts[4]),
        pick_rate: parseRate(texts[5]),
        ban_rate: parseRate(texts[6])
      };
    })
    .filter((entry) =>
      Number.isFinite(entry.rank) &&
      entry.champion_key &&
      entry.champion_name &&
      entry.role &&
      entry.win_rate !== null &&
      entry.pick_rate !== null &&
      entry.ban_rate !== null
    );

  const snapshot = {
    source: "OP.GG public champion tier list",
    source_url: location.href,
    captured_at: new Date().toISOString(),
    patch: patchMatch?.[1] ?? "unknown",
    region,
    tier,
    queue: normalizeQueue("ranked_solo_duo"),
    last_updated_label: lastUpdatedMatch?.[1]?.trim() ?? null,
    sample_count: sampleMatch ? Number(sampleMatch[1].replace(/,/g, "")) : null,
    entries
  };

  copy(JSON.stringify(snapshot, null, 2));
  console.log(`LeagueAkari OP.GG snapshot copied: ${entries.length} entries`, snapshot);
})();
