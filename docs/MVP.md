# MVP Scope

## Version Goal

Build the first usable version of LeagueAkari Plus around champion select decision support.

The MVP should feel useful before the game starts. It should help a player understand their account, read the current draft, and make safer champion/rune choices.

## Included Features

### 1. Client Connection

- Detect League client process
- Detect LCU port and auth token
- Show connection status
- Support CN and global client paths where possible

### 2. Current Account

- Show summoner name
- Show summoner icon
- Show level
- Show ranked queue summary when available

### 3. Recent Matches

- Show recent 20 matches
- Basic stats: win/loss, champion, role, KDA, damage, CS, vision score
- Highlight recent form

### 4. Champion Select

- Detect champion select session
- Read allied picks
- Read visible enemy picks
- Read bans when available
- Allow manual role correction when role detection is uncertain

### 5. Rune Recommendation

- Recommend rune page by champion, role, and patch data
- Require explicit user confirmation before applying
- Show what will be changed before writing to client

### 6. Composition Analysis

- Score both teams across draft dimensions
- Output win-rate range instead of exact fake precision
- Explain key reasons in readable Chinese
- Suggest missing draft needs while picks are still open

## Not Included In MVP

- Machine learning prediction model
- Full plugin marketplace
- Cloud account sync
- In-game overlay
- Any gameplay automation

## MVP Completion Criteria

- User can launch the app and see client connection status
- User can see their current summoner profile
- User can enter champion select and see draft analysis
- User can review recommended runes and apply them only after confirmation
- Project has clear safety boundaries and does not include balance-affecting behavior
