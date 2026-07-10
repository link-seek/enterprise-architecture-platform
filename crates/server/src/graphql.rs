use sea_orm::DatabaseConnection;
use seaography::{Builder, BuilderContext, RelatedEntityFilter, RelationBuilder};

use user_management::infrastructure::persistence::entities::{
    oauth_authorization_code, refresh_token, user,
};
use business_architecture::infrastructure::persistence::entities::{
    business_capability, business_process, capability_process, process_step, stage_capability,
    value_stream, value_stream_stage,
};

pub type GraphqlSchema = async_graphql::dynamic::Schema;

#[derive(Copy, Clone, Debug, sea_orm::EnumIter)]
enum NoRelation {}

impl RelationBuilder for NoRelation {
    fn get_relation_name(&self, _: &'static BuilderContext) -> String {
        unreachable!()
    }
    fn get_relation(
        &self,
        _: &'static BuilderContext,
    ) -> async_graphql::dynamic::Field {
        unreachable!()
    }
    fn get_related_entity_filter(
        &self,
        _: &'static BuilderContext,
    ) -> seaography::RelatedEntityFilterField {
        unreachable!()
    }
}

fn register_entity<T>(builder: &mut Builder)
where
    T: sea_orm::EntityTrait,
    <T as sea_orm::EntityTrait>::Model: Sync,
{
    let context = builder.context;
    let filter = RelatedEntityFilter::<T>::build::<NoRelation>(context);
    builder.register_entity::<T>(vec![], &filter);
}

fn register_entity_with_mutations<T, A>(builder: &mut Builder)
where
    T: sea_orm::EntityTrait,
    <T as sea_orm::EntityTrait>::Model: Sync,
    <T as sea_orm::EntityTrait>::Model: sea_orm::IntoActiveModel<A>,
    A: sea_orm::ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
{
    register_entity::<T>(builder);
    builder.register_entity_mutations::<T, A>();
}

pub async fn build_graphql_schema(db: &DatabaseConnection) -> anyhow::Result<GraphqlSchema> {
    let context: &'static BuilderContext = Box::leak(Box::new(BuilderContext::default()));

    let mut builder = Builder::new(context, db.clone());

    register_entity_with_mutations::<user::Entity, user::ActiveModel>(&mut builder);
    register_entity_with_mutations::<refresh_token::Entity, refresh_token::ActiveModel>(&mut builder);
    register_entity_with_mutations::<oauth_authorization_code::Entity, oauth_authorization_code::ActiveModel>(&mut builder);

    register_entity_with_mutations::<business_capability::Entity, business_capability::ActiveModel>(&mut builder);
    register_entity_with_mutations::<business_process::Entity, business_process::ActiveModel>(&mut builder);
    register_entity_with_mutations::<process_step::Entity, process_step::ActiveModel>(&mut builder);
    register_entity_with_mutations::<value_stream::Entity, value_stream::ActiveModel>(&mut builder);
    register_entity_with_mutations::<value_stream_stage::Entity, value_stream_stage::ActiveModel>(&mut builder);
    register_entity_with_mutations::<capability_process::Entity, capability_process::ActiveModel>(&mut builder);
    register_entity_with_mutations::<stage_capability::Entity, stage_capability::ActiveModel>(&mut builder);

    builder = builder
        .register_entity_dataloader_one_to_one(user::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(user::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(refresh_token::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(refresh_token::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(oauth_authorization_code::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(oauth_authorization_code::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(business_capability::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(business_capability::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(business_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(business_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(process_step::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(process_step::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(value_stream::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(value_stream::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(value_stream_stage::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(value_stream_stage::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(capability_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(capability_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(stage_capability::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(stage_capability::Entity, tokio::spawn);

    let schema = builder.schema_builder().finish()?;

    Ok(schema)
}
