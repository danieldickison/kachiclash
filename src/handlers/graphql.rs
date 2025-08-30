use crate::graphql::{create_schema, GraphQLSchema};
use crate::handlers::HandlerError;
use crate::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_actix_web::GraphQLRequest;

/// GraphQL endpoint handler
pub async fn graphql_handler(
    schema: web::Data<GraphQLSchema>,
    req: GraphQLRequest,
) -> Result<HttpResponse, HandlerError> {
    let response = schema.execute(req.into_inner()).await;

    // Create HTTP response with CORS headers
    let mut http_response = HttpResponse::Ok();
    add_cors_headers(&mut http_response);

    // Set content type and body with the GraphQL response
    Ok(http_response
        .content_type("application/json")
        .json(response))
}

/// GraphQL Playground handler for development/testing
pub async fn graphql_playground() -> Result<HttpResponse, HandlerError> {
    let source = playground_source(
        GraphQLPlaygroundConfig::new("/api/graphql").subscription_endpoint("/api/graphql"),
    );
    let mut response = HttpResponse::Ok();
    add_cors_headers(&mut response);

    Ok(response
        .content_type("text/html; charset=utf-8")
        .body(source))
}

/// Handle preflight CORS requests
pub async fn graphql_preflight(_req: HttpRequest) -> Result<HttpResponse, HandlerError> {
    let mut response = HttpResponse::Ok();
    add_cors_headers(&mut response);
    Ok(response.finish())
}

/// Add CORS headers to HTTP response
fn add_cors_headers(response: &mut actix_web::HttpResponseBuilder) {
    response
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
        .insert_header((
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, X-Requested-With",
        ))
        .insert_header(("Access-Control-Max-Age", "86400"));
}

/// Initialize GraphQL schema with database connection
pub fn init_schema(app_state: &AppState) -> GraphQLSchema {
    create_schema(app_state.db.clone())
}
