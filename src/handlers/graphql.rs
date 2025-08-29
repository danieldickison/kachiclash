use crate::graphql::{create_schema, GraphQLSchema};
use crate::handlers::HandlerError;
use crate::AppState;
use actix_web::{web, HttpResponse, Result};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};

/// GraphQL endpoint handler
pub async fn graphql_handler(
    schema: web::Data<GraphQLSchema>,
    req: GraphQLRequest,
) -> Result<GraphQLResponse, HandlerError> {
    let response = schema.execute(req.into_inner()).await;
    Ok(GraphQLResponse::from(response))
}

/// GraphQL Playground handler for development/testing
pub async fn graphql_playground() -> Result<HttpResponse, HandlerError> {
    let source = playground_source(
        GraphQLPlaygroundConfig::new("/api/graphql").subscription_endpoint("/api/graphql"),
    );
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(source))
}

/// Initialize GraphQL schema with database connection
pub fn init_schema(app_state: &AppState) -> GraphQLSchema {
    create_schema(app_state.db.clone())
}
