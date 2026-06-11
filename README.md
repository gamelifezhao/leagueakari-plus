# LeagueAkari Plus

LeagueAkari Plus is a second-development project based on the LeagueAkari idea: a League of Legends companion that helps players understand how to win a match, without building features that affect game balance.

> Not to help you cheat, but to help you understand how this game can be won.

## Product Direction

The first version focuses on a clean and safe MVP:

- League client connection status
- Current summoner profile
- Recent match history
- Champion select detection
- Rune recommendation with explicit confirmation before applying
- Team composition analysis with win-rate range
- Composition weakness hints, such as missing AP damage, frontline, engage, control, or scaling
- CN/Global client compatibility improvements

## Core Feature: Composition Analysis

LeagueAkari Plus should explain a draft in practical language:

- Estimated win-rate range, not fake precision
- Confidence level
- Strengths and weaknesses for both teams
- Damage mix, engage, peel, frontline, crowd control, scaling, and tempo
- Suggestions during champion select, based on the player's own champion pool

Example output:

```text
Estimated win rate: 54% - 58%
Confidence: Medium

Key reasons:
- Our team has stronger engage and better dragon-fight setup.
- Enemy team scales better after three items.
- Our AP damage is slightly low.
- Bot lane may face early pressure.
```

## Safety Boundary

This project should not include:

- In-game automation
- Memory reading or memory patching
- Anti-cheat bypass
- Auto dodge/auto play scripts
- Real-time gameplay manipulation

The goal is client-side analysis, organization, and decision support.

## Status

This repository is currently in the product planning and project bootstrap stage.

See:

- [MVP scope](docs/MVP.md)
- [Composition analysis design](docs/COMPOSITION_ANALYSIS.md)
- [Roadmap](docs/ROADMAP.md)
- [Legal and safety notes](docs/LEGAL_AND_SAFETY.md)
