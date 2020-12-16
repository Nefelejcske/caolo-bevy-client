mod caosim;

use bevy::prelude::*;

pub struct RoomCameraTag;

fn setup_tracing() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,caolo=debug".to_string());
    let sub = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(sub).unwrap();
}

fn setup(mut cmd: Commands) {
    // spawn the camera looking at rooms
    cmd.spawn(Camera2dComponents {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 20.0))
            .looking_at(Vec3::default(), Vec3::unit_y()),
        ..Default::default()
    })
    .with(RoomCameraTag);
}

fn main() {
    setup_tracing();

    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(TransformPlugin)
        .add_plugin(caosim::CaoSimPlugin)
        .add_startup_system(setup.system())
        .add_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .run();
}
