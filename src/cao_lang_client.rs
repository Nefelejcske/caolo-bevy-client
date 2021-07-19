mod caolang_model;

use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use futures_lite::future;

pub struct CaoLangSchema(Vec<caolang_model::SchemaNode>);

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
    CaoLangSchema(payload)
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
