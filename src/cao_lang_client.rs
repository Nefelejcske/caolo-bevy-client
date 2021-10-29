pub mod cao_lang_model;

use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use cao_lang::compiler::CaoIr;
use futures_lite::future;

use crate::{account::AuthToken, cao_lang_client::cao_lang_model::SchemaNode};

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

// TODO handle errors...
pub async fn fetch_my_programs(token: AuthToken) -> Result<(), ()> {
    let resp = surf::get(format!("{}/scripting/my-programs", crate::API_BASE_URL))
        .header("Authorization", token)
        .await;
    todo!()
}

pub type CreateNewProgramResult = Result<(), cao_lang_model::CreateProgramError>;
pub async fn create_new_program(name: String, token: AuthToken) -> CreateNewProgramResult {
    #[derive(serde::Serialize)]
    struct Payload {
        name: String,
    }
    let resp = surf::post(format!("{}/scripting/create-program", crate::API_BASE_URL))
        .header("Authorization", token)
        .body_json(&Payload { name })
        .unwrap()
        .await;

    dbg!(resp);
    todo!()
}

// TODO handle errors...
pub type CompileProgramResult = Result<(), cao_lang_model::RemoteCompileError>;
pub async fn compile_program(program: CaoIr) -> CompileProgramResult {
    let mut resp = loop {
        let resp = surf::post(format!("{}/scripting/compile", crate::API_BASE_URL))
            .body_json(&program)
            .expect("failed to serialize program");
        match resp.await {
            Ok(resp) => break resp,
            Err(err) => {
                error!("Request send failed, retrying, err: {:?}", err);
            }
        }
    };
    match resp.status() {
        surf::StatusCode::Ok => Ok(()),
        surf::StatusCode::BadRequest => {
            let body = resp
                .body_json()
                .await
                .expect("Failed to get compilation error");

            Err(body)
        }
        surf::StatusCode::UnprocessableEntity => todo!(),
        _ => {
            error!("Unexpected response {:?}", resp);
            todo!()
        }
    }
}

async fn get_schema() -> CaoLangSchema {
    let payload = surf::get(format!("{}/scripting/schema", crate::API_BASE_URL))
        .recv_json()
        .await
        .expect("Failed to get schema");
    trace!("Got schema payload {:#?}", payload);
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
