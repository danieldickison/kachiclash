#!/usr/bin/env node

/**
 * Example GraphQL client for Kachiclash API
 *
 * This script demonstrates how to interact with the Kachiclash GraphQL API
 * to fetch game data including players, bashos, and scores.
 *
 * Usage:
 *   node graphql_client.js
 *
 * Requirements:
 *   npm install node-fetch
 */

const fetch = require('node-fetch');

// Configuration
const GRAPHQL_ENDPOINT = 'http://localhost:8080/api/graphql';

/**
 * Execute a GraphQL query
 */
async function executeQuery(query, variables = {}) {
  try {
    const response = await fetch(GRAPHQL_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        query,
        variables,
      }),
    });

    const result = await response.json();

    if (result.errors) {
      console.error('GraphQL errors:', result.errors);
      return null;
    }

    return result.data;
  } catch (error) {
    console.error('Network error:', error);
    return null;
  }
}

/**
 * Get all bashos
 */
async function getBashos() {
  const query = `
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
  `;

  console.log('🏟️  Fetching bashos...');
  const data = await executeQuery(query);

  if (data && data.bashos) {
    console.log(`Found ${data.bashos.length} bashos:`);
    data.bashos.forEach(basho => {
      console.log(`  📅 ${basho.id}: ${basho.venue} (${basho.playerCount} players)`);
    });
    return data.bashos;
  }

  return [];
}

/**
 * Get players with Emperor's Cups
 */
async function getChampionPlayers() {
  const query = `
    query GetChampions {
      players(filter: { hasEmperorsCup: true }) {
        id
        name
        emperorsCups
        joinDate
        rank
      }
    }
  `;

  console.log('\n🏆 Fetching champion players...');
  const data = await executeQuery(query);

  if (data && data.players) {
    console.log(`Found ${data.players.length} champion players:`);
    data.players.forEach(player => {
      console.log(`  👑 ${player.name}: ${player.emperorsCups} cups (joined ${new Date(player.joinDate).getFullYear()})`);
    });
    return data.players;
  }

  return [];
}

/**
 * Get a specific basho by ID
 */
async function getBashoById(bashoId) {
  const query = `
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
  `;

  console.log(`\n🎯 Fetching basho ${bashoId}...`);
  const data = await executeQuery(query, { id: bashoId });

  if (data && data.basho) {
    const basho = data.basho;
    console.log(`Basho ${basho.id}:`);
    console.log(`  📍 Venue: ${basho.venue}`);
    console.log(`  📅 Start: ${new Date(basho.startDate).toLocaleDateString()}`);
    console.log(`  👥 Players: ${basho.playerCount}`);
    console.log(`  🏁 Started: ${basho.hasStarted ? 'Yes' : 'No'}`);
    if (basho.winningScore) {
      console.log(`  🥇 Winning Score: ${basho.winningScore}`);
    }
    return basho;
  }

  return null;
}

/**
 * Get player scores for a basho
 */
async function getPlayerScores(bashoId, limit = 5) {
  const query = `
    query GetPlayerScores($bashoId: String!) {
      playerScores(bashoId: $bashoId) {
        player {
          id
          name
          emperorsCups
        }
        wins
        place
        awards {
          name
        }
        rikishi {
          name
          wins
          losses
        }
      }
    }
  `;

  console.log(`\n📊 Fetching player scores for basho ${bashoId}...`);
  const data = await executeQuery(query, { bashoId });

  if (data && data.playerScores) {
    const scores = data.playerScores
      .filter(score => score.wins !== null)
      .sort((a, b) => (b.wins || 0) - (a.wins || 0))
      .slice(0, limit);

    console.log(`Top ${Math.min(limit, scores.length)} performers:`);
    scores.forEach((score, index) => {
      const place = score.place ? `#${score.place}` : `~${index + 1}`;
      const awards = score.awards.length > 0 ? ` ${score.awards.map(a => '🏆').join('')}` : '';
      console.log(`  ${place} ${score.player.name}: ${score.wins || 0} wins${awards}`);

      if (score.rikishi && score.rikishi.length > 0) {
        const picks = score.rikishi.filter(r => r !== null);
        if (picks.length > 0) {
          console.log(`      Picks: ${picks.map(r => `${r.name} (${r.wins}W-${r.losses}L)`).join(', ')}`);
        }
      }
    });

    return scores;
  }

  return [];
}

/**
 * Get current leaderboard
 */
async function getCurrentLeaderboard() {
  const query = `
    query GetLeaderboard {
      leaderboard {
        rank
        score
        player {
          name
          emperorsCups
          rank
        }
      }
    }
  `;

  console.log('\n🏅 Fetching current leaderboard...');
  const data = await executeQuery(query);

  if (data && data.leaderboard) {
    const top10 = data.leaderboard.slice(0, 10);
    console.log('Current top 10:');
    top10.forEach(entry => {
      const cups = entry.player.emperorsCups > 0 ? ` (${entry.player.emperorsCups}🏆)` : '';
      const rank = entry.player.rank ? ` [${entry.player.rank}]` : '';
      console.log(`  ${entry.rank}. ${entry.player.name}: ${entry.score} points${cups}${rank}`);
    });
    return top10;
  }

  return [];
}

/**
 * Get rikishi for a basho
 */
async function getBashoRikishi(bashoId, limit = 10) {
  const query = `
    query GetBashoRikishi($bashoId: String!) {
      bashoRikishi(bashoId: $bashoId) {
        id
        name
        rank
        wins
        losses
        picks
        isKyujyo
      }
    }
  `;

  console.log(`\n🤼 Fetching rikishi for basho ${bashoId}...`);
  const data = await executeQuery(query, { bashoId });

  if (data && data.bashoRikishi) {
    const topRikishi = data.bashoRikishi
      .sort((a, b) => b.picks - a.picks)
      .slice(0, limit);

    console.log(`Top ${Math.min(limit, topRikishi.length)} most picked rikishi:`);
    topRikishi.forEach(rikishi => {
      const status = rikishi.isKyujyo ? ' (Kyujo)' : '';
      const record = rikishi.wins || rikishi.losses ? ` ${rikishi.wins}W-${rikishi.losses}L` : '';
      console.log(`  🥋 ${rikishi.name} [${rikishi.rank}]: ${rikishi.picks} picks${record}${status}`);
    });

    return topRikishi;
  }

  return [];
}

/**
 * Main function to demonstrate API usage
 */
async function main() {
  console.log('🎌 Kachiclash GraphQL API Demo');
  console.log('===============================');

  try {
    // Get all bashos
    const bashos = await getBashos();

    // Get champion players
    await getChampionPlayers();

    // Get current leaderboard
    await getCurrentLeaderboard();

    // If we have bashos, demonstrate with the most recent one
    if (bashos.length > 0) {
      const recentBasho = bashos[0];

      // Get detailed basho information
      await getBashoById(recentBasho.id);

      // Get player scores for this basho
      await getPlayerScores(recentBasho.id);

      // Get rikishi for this basho
      await getBashoRikishi(recentBasho.id);
    }

    console.log('\n✅ Demo completed successfully!');
    console.log('\nTo explore more:');
    console.log('1. Open http://localhost:8080/api/graphql in your browser for the GraphQL Playground');
    console.log('2. Try the example queries in GRAPHQL_API.md');
    console.log('3. Modify this script to test different queries');

  } catch (error) {
    console.error('❌ Demo failed:', error);
  }
}

// Run the demo if this script is executed directly
if (require.main === module) {
  main().catch(console.error);
}

module.exports = {
  executeQuery,
  getBashos,
  getChampionPlayers,
  getBashoById,
  getPlayerScores,
  getCurrentLeaderboard,
  getBashoRikishi,
};
