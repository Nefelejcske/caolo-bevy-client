mod caosim;

use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(caosim::CaoSimPlugin)
        .run();
}
