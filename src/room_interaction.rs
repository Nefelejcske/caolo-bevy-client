use bevy::{
    ecs::schedule::ShouldRun,
    prelude::*,
    render::camera::{Camera, CameraProjection, PerspectiveProjection},
};

use crate::{
    camera_control::RoomCameraTag, cao_sim_client::cao_sim_model::AxialPos, terrain::RoomOffsets,
    AppState,
};

#[derive(Debug, Clone, Copy)]
pub struct EguiInteraction(pub bool);

#[derive(Default, Debug, Clone, Copy)]
pub struct SelectedEntity {
    pub entity: Option<Entity>,
}

#[derive(Debug, Clone)]
struct EntitySelection {
    pub click_id: u32,
    /// absolute position
    pub pos: AxialPos,
}

impl Default for EntitySelection {
    fn default() -> Self {
        EntitySelection {
            click_id: 0,
            pos: Default::default(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct HoveredTile {
    /// in world coordinate system
    pub axial: AxialPos,
    pub world_pos: Vec3,
}

#[derive(Default, Debug, Clone)]
pub struct LookAtRoom {
    /// in world coordinate system
    pub id: AxialPos,
    pub world_pos: Vec3,
}

fn select_tile_system(
    tile: Res<HoveredTile>,
    keys: Res<Input<MouseButton>>,
    mut selection: ResMut<EntitySelection>,
    mut selected: ResMut<SelectedEntity>,
    entities: Res<crate::cao_entities::EntityPositionMap>,
) {
    if keys
        .get_just_pressed()
        .any(|k| matches!(k, MouseButton::Left))
    {
        let is_new_tile = tile.axial != selection.pos;
        if is_new_tile {
            selection.click_id = 0;
        } else {
            selection.click_id += 1
        }
        selection.pos = tile.axial;
        selected.entity = None;
        let entity_ids = entities.0.get(&selection.pos);
        selected.entity = entity_ids.and_then(|all| {
            (!all.is_empty()).then(|| {
                let ind = selection.click_id as usize % all.len();
                all[ind]
            })
        });
    }
}

fn window_to_world(
    window_pos: Vec2,
    window: &Window,
    cam_transform: &GlobalTransform,
    projection: &PerspectiveProjection,
) -> Vec3 {
    // normalized device coordinates
    let ndc = Vec3::new(
        (2.0 * window_pos.x) / window.width() - 1.,
        (2.0 * window_pos.y) / window.height() - 1.,
        projection.near,
    );

    let ndc_to_world =
        cam_transform.compute_matrix() * projection.get_projection_matrix().inverse();
    ndc_to_world.project_point3(ndc)
}

/// intersect a given AB line with the plane of the terrain.
/// Assumes that the line always intersects the plane...
fn intersect_line_terrain_plain(a: Vec3, b: Vec3) -> Vec3 {
    intersect_ray_terrain_plain(a, b - a)
}

/// intersect a given ray with the plane of the terrain.
/// Assumes that the ray always intersects the plane...
///
/// - `n=<0, 1, 0>`
/// - `d=-1`
fn intersect_ray_terrain_plain(a: Vec3, dir: Vec3) -> Vec3 {
    let n = Vec3::Y;
    let t = (-1.0 - n.dot(a)) / n.dot(dir);

    a + t * dir
}

fn update_lookat_room_system(
    mut room: ResMut<LookAtRoom>,
    q_cam: Query<&GlobalTransform, With<RoomCameraTag>>,
    current: Res<crate::terrain::CurrentRoom>,
    mut new_current_room: EventWriter<crate::terrain::NewCurrentRoom>,
    mut offsets: ResMut<RoomOffsets>,
) {
    for cam_tr in q_cam.iter() {
        let point_q = intersect_ray_terrain_plain(cam_tr.translation, cam_tr.local_z());

        // hex size = 1
        let q = 3.0f32.sqrt() / 3.0 * point_q.x - point_q.z / 3.;
        let r = 2. * point_q.z / 3.;

        let axial_on_plane = cao_math::hex::round_to_nearest_axial(q, r);

        let axial = AxialPos {
            q: axial_on_plane.x as i32,
            r: axial_on_plane.y as i32,
        };

        const RADIUS: i32 = 30; // TODO query this pls...

        let offset = match offsets.0.get(&current.0) {
            Some(x) => x,
            None => continue,
        };
        let center = AxialPos {
            q: offset.q + RADIUS,
            r: offset.r + RADIUS,
        };
        let delta = AxialPos {
            q: (center.q - axial.q).abs(),
            r: (center.r - axial.r).abs(),
        };
        // add bias to the current room so we don't trigger switch if the player moves the camera
        // back-and-forth
        if delta.q * delta.q + delta.r * delta.r >= (RADIUS * 3 / 2).pow(2) {
            // out of current room
            if let Some((room_id, _, _)) = offsets
                .0
                .iter()
                .map(|(id, offset)| {
                    // distance from center
                    let dq = axial.q - (offset.q + RADIUS);
                    let dr = axial.r - (offset.r + RADIUS);

                    // filter out negative distances, these are out of bounds
                    (id, offset, AxialPos { q: dq, r: dr })
                })
                .min_by_key(|(_id, _offset, delta)| delta.q * delta.q + delta.r * delta.r)
            {
                let room_id = *room_id;
                room.id = room_id;
                room.world_pos = point_q;
                if room_id != current.0 {
                    new_current_room.send(crate::terrain::NewCurrentRoom(room_id));
                }
            }
        }
    }
}

fn update_selected_tile_system(
    mut st: ResMut<HoveredTile>,
    windows: Res<Windows>,
    mut cur_move: EventReader<CursorMoved>,
    q_cam: Query<(&GlobalTransform, &Camera, &PerspectiveProjection), With<RoomCameraTag>>,
) {
    for m in cur_move.iter() {
        let win = windows.get(m.id).expect("window not found");
        let cursor_pos = m.position;
        for (cam_tr, cam, proj) in q_cam.iter() {
            if m.id != cam.window {
                continue;
            }
            let cursor_pos = window_to_world(cursor_pos, win, cam_tr, proj);

            let point_q = intersect_line_terrain_plain(cam_tr.translation, cursor_pos);

            // hex size = 1
            let q = 3.0f32.sqrt() / 3.0 * point_q.x - point_q.z / 3.;
            let r = 2. * point_q.z / 3.;

            let axial_on_plane = cao_math::hex::round_to_nearest_axial(q, r);

            let axial = AxialPos {
                q: axial_on_plane.x as i32,
                r: axial_on_plane.y as i32,
            };

            st.axial = axial;
            st.world_pos = point_q;
        }
    }
}

fn update_interaction_system(
    mut eguiint: ResMut<EguiInteraction>,
    ctx: ResMut<bevy_egui::EguiContext>,
) {
    eguiint.0 = ctx.ctx().wants_pointer_input();
}

fn should_room_systems_run(
    eguiint: Res<EguiInteraction>,
    state: Res<State<AppState>>,
) -> ShouldRun {
    if !eguiint.0 && matches!(state.current(), AppState::Room) {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

pub struct RoomInteractionPlugin;

impl Plugin for RoomInteractionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(HoveredTile::default())
            .insert_resource(LookAtRoom::default())
            .insert_resource(EntitySelection::default())
            .insert_resource(SelectedEntity::default())
            .insert_resource(EguiInteraction(false))
            .add_system(update_interaction_system.system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(should_room_systems_run.system())
                    .with_system(update_selected_tile_system.system())
                    .with_system(update_lookat_room_system.system())
                    .with_system(select_tile_system.system()),
            );
    }
}
