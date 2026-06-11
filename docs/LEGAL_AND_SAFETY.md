# Legal And Safety Notes

## Upstream License

Before copying or modifying original LeagueAkari source code, confirm the upstream repository license.

If the upstream project has no license, do not redistribute modified source until permission is clear.

## Project Safety Boundary

LeagueAkari Plus should stay in the category of client companion and analysis tool.

Allowed areas:

- Reading public or client-exposed data through documented/local client interfaces
- Showing account, match, champion select, and draft information
- Recommending runes, builds, bans, or picks
- Applying client configuration only after explicit user confirmation

Disallowed areas:

- Gameplay automation
- Memory reading or patching
- Anti-cheat bypass
- Hidden information extraction
- Actions that manipulate live gameplay

## User Trust

The app should clearly explain what data it reads and what actions it performs.

Potentially changing actions, such as applying rune pages, should be explicit and reversible.
