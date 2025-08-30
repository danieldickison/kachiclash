# Kachi Clash

A Grand Sumo prediction game.

https://kachiclash.com

## GraphQL API

Kachiclash now includes a read-only GraphQL API for accessing game data including player picks, scores, and basho information.

### Quick Start

1. Start the server: `cargo run`
2. Open GraphQL Playground: `http://localhost:8080/api/graphql`
3. Try example queries from `GRAPHQL_API.md`

### API Features

- **Players**: Get player information, rankings, and Emperor's Cup winners
- **Bashos**: Access tournament data, venues, dates, and participation stats
- **Scores**: Retrieve player performance and picks for specific bashos
- **Rikishi**: View wrestler data including records and pick popularity
- **Leaderboards**: Get current standings and historical rankings

### Documentation

- **Full API Documentation**: See `GRAPHQL_API.md` for detailed schema and examples
- **Interactive Demo**: Open `examples/graphql_demo.html` in your browser
- **Client Examples**: Check `examples/graphql_client.js` for programmatic usage

### Example Query

```graphql
query GetCurrentLeaderboard {
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

The GraphQL API is read-only and provides comprehensive access to all game data for building dashboards, analytics tools, and mobile applications.
