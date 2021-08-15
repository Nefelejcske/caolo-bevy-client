pub mod cao_lang_model;

use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use futures_lite::future;

use crate::cao_lang_client::cao_lang_model::SchemaNode;

pub struct CaoLangSchema(pub Vec<cao_lang_model::SchemaNode>);

pub struct CaoLangPlugin;

fn handle_tasks_system(
    mut commands: Commands,
    mut layout: ResMut<CaoLangSchema>,
    q: Query<(Entity, &mut Task<CaoLangSchema>)>,
) {
    q.for_each_mut(|(e, mut t)| {
        if let Some(stuff) = future::block_on(future::poll_once(&mut *t)) {
            *layout = stuff;
            commands.entity(e).remove::<Task<CaoLangSchema>>();
        }
    });
}

async fn get_schema() -> CaoLangSchema {
    let payload = surf::get(format!("{}/scripting/schema", crate::API_BASE_URL))
        .recv_json()
        .await
        .expect("Failed to get schema");
    debug!("Got schema payload {:#?}", payload);
    let mut result = CaoLangSchema(payload);
    let default_cards = cao_lang::compiler::card_description::get_instruction_descriptions();
    result.0.extend(default_cards.iter().map(
        |cao_lang::SubProgram {
             name,
             description,
             ty,
             output,
             input,
             properties,
         }| {
            SchemaNode {
                name: name.to_string(),
                ty: format!("{}", ty.as_str()),
                description: description.to_string(),
                input: input.iter().map(ToString::to_string).collect(),
                output: output.iter().map(ToString::to_string).collect(),
                properties: properties.iter().map(ToString::to_string).collect(),
            }
        },
    ));

    result
}

fn setup_schema_task_system(mut commands: Commands, task_pool: Res<IoTaskPool>) {
    let handle = task_pool.spawn(get_schema());
    commands.spawn().insert(handle);
}

impl Plugin for CaoLangPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(CaoLangSchema(Vec::new()))
            .add_startup_system(setup_schema_task_system.system())
            .add_system(handle_tasks_system.system());
    }
}
