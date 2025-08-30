# Kachiclash GraphQL API Documentation

This document describes the read-only GraphQL API for accessing Kachiclash game data, including player picks, scores, and basho information.

## Endpoint

The GraphQL API is available at:

- **Endpoint**: `/api/graphql`
- **Playground**: `/api/graphql` (GET request for development/testing)

## Schema Overview

The API provides access to the following main types:

### Core Types

#### Player

Represents a player in the game.

```graphql
type Player {
  id: Int! # Unique player ID
  name: String! # Player's display name
  joinDate: DateTime! # When the player joined
  emperorsCups: Int! # Number of Emperor's Cups won
  rank: String # Current rank (if any)
  hasEmperorsCup: Boolean! # Whether player has won an Emperor's Cup
  urlPath: String! # URL path for player profile
}
```

#### Basho

Represents a tournament/basho.

```graphql
type Basho {
  id: String! # Basho ID (e.g., "202401" for January 2024)
  startDate: DateTime! # When the basho starts
  venue: String! # Venue name
  externalLink: String # External link (if available)
  playerCount: Int! # Number of participating players
  winningScore: Int # Winning score
  hasStarted: Boolean! # Whether the basho has started
}
```

#### Rikishi

Represents a sumo wrestler.

```graphql
type Rikishi {
  id: Int! # Unique rikishi ID
  name: String! # Wrestling name
  rank: String! # Rank in the tournament
  results: [String]! # Daily results (W/L strings)
  wins: Int! # Total wins
  losses: Int! # Total losses
  picks: Int! # Number of picks by players
  isKyujyo: Boolean! # Whether this rikishi is absent
}
```

#### PlayerScore

Represents a player's performance in a specific basho.

```graphql
type PlayerScore {
  player: Player! # The player
  basho: Basho! # The basho
  rank: String # Player's rank before this basho
  rikishi: [PlayerBashoRikishi] # Picked rikishi with their performance
  wins: Int # Total wins achieved
  place: Int # Final ranking/place
  awards: [Award]! # Awards earned
}
```

## Query Examples

### Get All Bashos

```graphql
query GetBashos {
  bashos {
    id
    startDate
    venue
    playerCount
    winningScore
    hasStarted
  }
}
```

### Get Specific Basho by ID

```graphql
query GetBasho($id: String!) {
  basho(id: $id) {
    id
    startDate
    venue
    externalLink
    playerCount
    winningScore
    hasStarted
  }
}
```

**Variables:**

```json
{
  "id": "202401"
}
```

### Get All Players

```graphql
query GetPlayers {
  players {
    id
    name
    joinDate
    emperorsCups
    rank
    hasEmperorsCup
    urlPath
  }
}
```

### Get Players with Filters

```graphql
query GetPlayersWithFilters($filter: PlayerFilter) {
  players(filter: $filter) {
    id
    name
    emperorsCups
    hasEmperorsCup
  }
}
```

**Variables (get only players with Emperor's Cups):**

```json
{
  "filter": {
    "hasEmperorsCup": true,
    "limit": 10
  }
}
```

### Get Player by Name

```graphql
query GetPlayerByName($name: String!) {
  playerByName(name: $name) {
    id
    name
    joinDate
    emperorsCups
    rank
    hasEmperorsCup
  }
}
```

**Variables:**

```json
{
  "name": "YourPlayerName"
}
```

### Get Player Scores for a Basho

```graphql
query GetPlayerScores($bashoId: String!, $playerId: Int) {
  playerScores(bashoId: $bashoId, playerId: $playerId) {
    player {
      id
      name
      emperorsCups
    }
    basho {
      id
      venue
      startDate
    }
    rank
    wins
    place
    rikishi {
      name
      wins
      losses
    }
    awards {
      awardType
      name
    }
  }
}
```

**Variables (get all player scores for a basho):**

```json
{
  "bashoId": "202401"
}
```

**Variables (get specific player's score):**

```json
{
  "bashoId": "202401",
  "playerId": 123
}
```

### Get Rikishi for a Basho

```graphql
query GetBashoRikishi($bashoId: String!) {
  bashoRikishi(bashoId: $bashoId) {
    id
    name
    rank
    wins
    losses
    picks
    isKyujyo
    results
  }
}
```

**Variables:**

```json
{
  "bashoId": "202401"
}
```

### Get Current Leaderboard

```graphql
query GetLeaderboard($bashoId: String) {
  leaderboard(bashoId: $bashoId) {
    rank
    score
    player {
      id
      name
      emperorsCups
      rank
    }
  }
}
```

**Variables (current basho leaderboard):**

```json
{}
```

**Variables (specific basho leaderboard):**

```json
{
  "bashoId": "202401"
}
```

## Input Types

### BashoFilter

```graphql
input BashoFilter {
  id: String # Filter by specific basho ID
  completedOnly: Boolean # Only include completed bashos
  limit: Int # Limit number of results
}
```

### PlayerFilter

```graphql
input PlayerFilter {
  name: String # Filter by player name
  id: Int # Filter by player ID
  hasEmperorsCup: Boolean # Only include players with Emperor's Cups
  limit: Int # Limit number of results
}
```

## Common Use Cases

### 1. Game Dashboard

Get current basho information and leaderboard:

```graphql
query GameDashboard {
  bashos(filter: { limit: 1 }) {
    id
    startDate
    venue
    playerCount
    hasStarted
  }
  leaderboard {
    rank
    score
    player {
      name
      emperorsCups
    }
  }
}
```

### 2. Player Profile

Get comprehensive player information:

```graphql
query PlayerProfile($playerName: String!) {
  player: playerByName(name: $playerName) {
    id
    name
    joinDate
    emperorsCups
    rank
    hasEmperorsCup
    urlPath
  }
}
```

### 3. Basho Results

Get detailed results for a completed basho:

```graphql
query BashoResults($bashoId: String!) {
  basho(id: $bashoId) {
    id
    venue
    startDate
    winningScore
  }
  playerScores(bashoId: $bashoId) {
    player {
      name
    }
    wins
    place
    awards {
      name
    }
  }
}
```

## Error Handling

The API returns standard GraphQL errors for:

- Invalid basho ID format
- Player not found
- Basho not found
- Database connection issues

Example error response:

```json
{
  "errors": [
    {
      "message": "Invalid basho ID format",
      "locations": [{ "line": 2, "column": 3 }],
      "path": ["basho"]
    }
  ],
  "data": null
}
```

## Development

### Running the Server

```bash
cargo run --bin kachiclash
```

The GraphQL playground will be available at `http://localhost:8080/api/graphql` (adjust port as needed).

### GraphQL Playground

The GraphQL playground provides:

- Interactive query editor
- Schema documentation
- Query validation
- Result visualization

Access it by navigating to the GraphQL endpoint in your browser.
