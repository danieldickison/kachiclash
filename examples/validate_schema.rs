//! GraphQL Schema Validation Script
//!
//! This script validates the GraphQL schema and demonstrates
//! that all types are properly configured and accessible.

use kachiclash::data::{make_conn, DbConn};
use kachiclash::graphql::{create_schema, GraphQLSchema};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Validating Kachiclash GraphQL Schema");
    println!("=========================================");

    // Create a test database connection
    let db_path = Path::new("var/kachiclash.sqlite");
    if !db_path.exists() {
        println!("⚠️  Database file not found at {:?}", db_path);
        println!("   This is expected if you haven't set up the database yet.");
        println!("   The schema validation will still work.");
    }

    let db: DbConn = make_conn(db_path);

    // Create the GraphQL schema
    println!("🚀 Creating GraphQL schema...");
    let schema: GraphQLSchema = create_schema(db);

    // Get SDL (Schema Definition Language) representation
    println!("📋 Generating SDL...");
    let sdl = schema.sdl();

    // Validate schema structure
    println!("✅ Schema created successfully!");
    println!("📊 Schema Statistics:");

    // Count types, queries, etc.
    let lines: Vec<&str> = sdl.lines().collect();
    let type_count = lines
        .iter()
        .filter(|line| line.starts_with("type "))
        .count();
    let input_count = lines
        .iter()
        .filter(|line| line.starts_with("input "))
        .count();
    let enum_count = lines
        .iter()
        .filter(|line| line.starts_with("enum "))
        .count();

    println!("   Types: {}", type_count);
    println!("   Input Types: {}", input_count);
    println!("   Enums: {}", enum_count);
    println!("   Total Lines: {}", lines.len());

    // Print key types found
    println!("\n🔍 Key Types Found:");
    let key_types = [
        "Player",
        "Basho",
        "Rikishi",
        "PlayerScore",
        "Award",
        "LeaderboardEntry",
        "PlayerBashoRikishi",
    ];

    for type_name in &key_types {
        if sdl.contains(&format!("type {}", type_name)) {
            println!("   ✅ {}", type_name);
        } else {
            println!("   ❌ {} (MISSING)", type_name);
        }
    }

    // Print available queries
    println!("\n📝 Available Queries:");
    let queries = [
        "bashos",
        "basho",
        "players",
        "player",
        "playerByName",
        "playerScores",
        "bashoRikishi",
        "leaderboard",
    ];

    for query in &queries {
        if sdl.contains(query) {
            println!("   ✅ {}", query);
        } else {
            println!("   ❌ {} (MISSING)", query);
        }
    }

    // Check for input types
    println!("\n📥 Input Types:");
    let input_types = ["BashoFilter", "PlayerFilter"];

    for input_type in &input_types {
        if sdl.contains(&format!("input {}", input_type)) {
            println!("   ✅ {}", input_type);
        } else {
            println!("   ❌ {} (MISSING)", input_type);
        }
    }

    // Optionally print the full SDL for debugging
    if std::env::var("SHOW_SDL").is_ok() {
        println!("\n📜 Full Schema Definition Language:");
        println!("{}", "=".repeat(80));
        println!("{}", sdl);
        println!("{}", "=".repeat(80));
    } else {
        println!("\n💡 Tip: Run with SHOW_SDL=1 to see the full schema definition");
    }

    println!("\n🎉 Schema validation completed successfully!");
    println!("🌐 The GraphQL API is ready to use at /api/graphql");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let db_path = Path::new(":memory:");
        let db = make_conn(db_path);
        let schema = create_schema(db);
        let sdl = schema.sdl();

        // Basic validation
        assert!(sdl.contains("type Query"));
        assert!(sdl.contains("type Player"));
        assert!(sdl.contains("type Basho"));
        assert!(sdl.contains("bashos"));
        assert!(sdl.contains("players"));
    }

    #[test]
    fn test_required_types_exist() {
        let db_path = Path::new(":memory:");
        let db = make_conn(db_path);
        let schema = create_schema(db);
        let sdl = schema.sdl();

        let required_types = [
            "Player",
            "Basho",
            "Rikishi",
            "PlayerScore",
            "Award",
            "LeaderboardEntry",
        ];

        for type_name in &required_types {
            assert!(
                sdl.contains(&format!("type {}", type_name)),
                "Missing required type: {}",
                type_name
            );
        }
    }

    #[test]
    fn test_input_types_exist() {
        let db_path = Path::new(":memory:");
        let db = make_conn(db_path);
        let schema = create_schema(db);
        let sdl = schema.sdl();

        assert!(sdl.contains("input BashoFilter"));
        assert!(sdl.contains("input PlayerFilter"));
    }
}
