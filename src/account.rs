mod account_model;

use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use futures_lite::future;

pub type AuthToken = String;
pub type LoginError = String;
pub type LoginResult<T> = Result<T, LoginError>;
pub type LoginRequestTask = Task<LoginResult<AuthToken>>;

pub struct CurrentAuthToken(pub Option<AuthToken>);
pub struct LastLoginError(pub Option<LoginError>);

#[derive(Default, Clone)]
pub struct StartLoginEvent {
    pub username: String,
    pub password: String,
}

async fn login(username: String, password: String) -> LoginResult<AuthToken> {
    let mut res = surf::post(format!("{}/token", crate::API_BASE_URL))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=&username={}&password={}&scope=&client_id=&client_secret=",
            username, password,
        ))
        .await
        .expect("Login request failed");

    match res.status() {
        surf::StatusCode::Ok => {
            let body: account_model::LoginSuccess =
                res.body_json().await.expect("Failed to read response");
            debug!("Successful login");
            Ok(body.access_token)
        }
        surf::StatusCode::UnprocessableEntity => {
            let body: account_model::LoginUnprocEntity = res
                .body_json()
                .await
                .expect("Failed to read unproc entity response");

            debug!("{} {:?}", res.status(), body);

            Err(format!(
                "{}: {}",
                body.detail[0].loc.last().map(|x| x.as_str()).unwrap_or(""),
                body.detail[0].msg
            ))
        }
        _ => {
            let body: account_model::LoginError = res
                .body_json()
                .await
                .expect("Failed to read error response");

            debug!("{} {:?}", res.status(), body);

            Err(body.detail)
        }
    }
}

fn handle_tasks_system(
    mut cmd: Commands,
    mut token: ResMut<CurrentAuthToken>,
    mut error: ResMut<LastLoginError>,
    tasks: Query<(Entity, &mut LoginRequestTask)>,
) {
    tasks.for_each_mut(|(e, mut t)| {
        if let Some(res) = future::block_on(future::poll_once(&mut *t)) {
            match res {
                Ok(t) => token.0 = Some(t),
                Err(e) => error.0 = Some(e),
            }
            cmd.entity(e).despawn_recursive();
        }
    });
}

fn setup_login_task_system(
    mut cmd: Commands,
    task_pool: Res<IoTaskPool>,
    mut events: EventReader<StartLoginEvent>,
    mut error: ResMut<LastLoginError>,
) {
    for StartLoginEvent { username, password } in events.iter() {
        let handle = task_pool.spawn(login(username.clone(), password.clone()));
        cmd.spawn().insert(handle);

        error.0 = None;
    }
}

pub struct AccountPlugin;

impl Plugin for AccountPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(CurrentAuthToken(None))
            .insert_resource(LastLoginError(None))
            .add_event::<StartLoginEvent>()
            .add_system(setup_login_task_system.system())
            .add_system(handle_tasks_system.system());
    }
}
