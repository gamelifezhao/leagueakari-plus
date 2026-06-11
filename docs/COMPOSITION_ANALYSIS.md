# Composition Analysis Design

## Purpose

The composition analysis module should help players understand a draft.

It should not promise exact predictions. It should provide a conservative win-rate range, confidence level, and practical explanations.

## First Model: Rule-Based Scoring

The first version should use a transparent rule model instead of machine learning.

Each champion receives tags and scores, for example:

```text
Ornn:
frontline 5
engage 4
peel 3
scaling 4
magic_damage 2
physical_damage 1
crowd_control 5

Zed:
burst 5
side_lane 4
engage 1
scaling 2
physical_damage 5
reliability 2
```

The team score is calculated from champion tags, role assumptions, and draft context.

## Draft Dimensions

- Lane strength
- Early tempo
- Scaling
- Engage
- Counter-engage
- Peel
- Frontline
- Crowd control
- Damage mix
- Poke range
- Wave clear
- Objective speed
- Team-fight reliability
- Split-push pressure
- Execution difficulty

## Output Shape

The user-facing result should be concise:

```text
Estimated win rate: 51% - 55%
Confidence: Medium

Our strengths:
- Better engage
- Stronger 5v5 front-to-back fights
- More reliable crowd control

Our risks:
- Low AP damage
- Weak side-lane pressure
- Enemy bot lane has early priority

Draft suggestion:
- If mid pick is still open, prefer AP wave-clear or control mage.
```

## Confidence Rules

Confidence should be lower when:

- Enemy picks are incomplete
- Roles are uncertain
- Patch data is missing
- Player champion proficiency is unknown
- The analysis depends on high-execution champions

## Later Improvements

- Add patch-based champion win rates
- Add lane matchup data
- Add champion synergy data
- Add player champion-pool preference
- Add optional prediction model after enough reliable data exists
