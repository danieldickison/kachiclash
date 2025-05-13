use juniper::{graphql_object, EmptyMutation, EmptySubscription};

use crate::data::{BashoId, BashoInfo, DbConn};

struct Context {
    db: DbConn,
}

impl juniper::Context for Context {}

struct Query;

#[graphql_object]
#[graphql(context = Context)]
impl Query {
    fn api_version() -> &'static str {
        "1.0"
    }

    fn basho(
        basho_id: BashoId,
        context: &Context,
    ) -> Result<Option<BashoInfo>, juniper::FieldError> {
        todo!("Implement basho query")
    }
}

pub type Schema =
    juniper::RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;
