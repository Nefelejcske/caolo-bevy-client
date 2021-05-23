use bevy::prelude::*;

use crate::caosim::NewEntities;

pub struct Bot;
pub struct LastPos(pub Vec2);
pub struct NextPos(pub Vec2);
pub struct CurrentPos(pub Vec2);

#[derive(Debug, Clone, Default)]
struct WalkTimer(Timer);

pub struct BotsPlugin;

pub const STEP_TIME: f32 = 0.8;

pub fn spawn_bot(
    cmd: &mut Commands,
    pos: Vec2,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    cmd.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            radius: 1.0,
            subdivisions: 2,
        })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        ..Default::default()
    })
    .insert_bundle((Bot, LastPos(pos), NextPos(pos), CurrentPos(pos)))
    .id()
}

fn update_transform(mut query: Query<(&CurrentPos, &mut Transform)>) {
    for (CurrentPos(p), mut tr) in query.iter_mut() {
        tr.translation = p.extend(0.0);
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn update_pos(
    mut t: ResMut<WalkTimer>,
    time: Res<Time>,
    mut query: Query<(&LastPos, &NextPos, &mut CurrentPos), With<Bot>>,
) {
    t.0.tick(time.delta());
    let WalkTimer(ref mut t) = &mut *t;
    let t = t.elapsed_secs() / STEP_TIME;
    let t = smoothstep(t);
    for (last, next, mut curr) in query.iter_mut() {
        curr.0 = last.0.lerp(next.0, t);
    }
}

fn on_new_entities(mut t: ResMut<WalkTimer>, mut new_entities: EventReader<NewEntities>) {
    for _ in new_entities.iter() {
        t.0.reset();
    }
}

fn setup(mut t: ResMut<WalkTimer>) {
    t.0 = Timer::from_seconds(STEP_TIME, false);
}

impl Plugin for BotsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(update_pos.system())
            .add_startup_system(setup.system())
            .add_system(on_new_entities.system())
            .add_system(update_transform.system())
            .init_resource::<WalkTimer>();
    }
}
