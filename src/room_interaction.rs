use crate::EntityType;
use arrayvec::ArrayVec;
use bevy::{
    prelude::*,
    render::camera::{Camera, CameraProjection, PerspectiveProjection},
};

use crate::{
    camera_control::RoomCameraTag,
    caosim::{cao_sim_model::AxialPos, SimEntityId},
};

#[derive(Debug, Clone, Copy)]
pub struct SelectedEntity {
    pub entity: Option<(SimEntityId, Entity)>,
    pub ty: EntityType,
}

impl Default for SelectedEntity {
    fn default() -> Self {
        Self {
            entity: None,
            ty: EntityType::Undefined,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct EntitySelection {
    pub click_id: i32,
    pub axial: AxialPos,
}

impl Default for EntitySelection {
    fn default() -> Self {
        EntitySelection {
            click_id: 0,
            axial: AxialPos::default(),
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct HoveredTile {
    pub axial: AxialPos,
    pub world_pos: Vec3,
}

fn select_tile_system(
    tile: Res<HoveredTile>,
    keys: Res<Input<MouseButton>>,
    mut selection: ResMut<EntitySelection>,
    mut selected: ResMut<SelectedEntity>,
    bots: Res<crate::bots::EntityPositionMap>,
    // TODO: resources, structures
) {
    if keys
        .get_just_pressed()
        .any(|k| matches!(k, MouseButton::Left))
    {
        let is_new_tile = tile.axial != selection.axial;
        if is_new_tile {
            selection.click_id = 0;
        } else {
            selection.click_id += 1
        }
        selection.axial = tile.axial;
        selected.entity = None;
        let mut all: ArrayVec<_, 3> = ArrayVec::new();
        if let Some(ids) = bots.0.get(&selection.axial).copied() {
            all.push((ids, EntityType::Bot));
        }
        // TODO: resources, structures
        selected.entity = (!all.is_empty()).then(|| {
            let ind = selection.click_id as usize % all.len();
            selected.ty = all[ind].1;
            all[ind].0
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
///
/// - `n=<0, 1, 0>`
/// - `d=-1`
fn intersect_line_terrain_plain(a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;

    let n = Vec3::Y;
    let t = (-1.0 - n.dot(a)) / n.dot(ab);

    a + t * ab
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

            let res = cao_math::hex::round_to_nearest_axial(q, r);

            let axial = AxialPos {
                q: res.x as i32,
                r: res.y as i32,
            };

            st.axial = axial;
            st.world_pos = point_q;
        }
    }
}

pub struct RoomInteractionPlugin;

impl Plugin for RoomInteractionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(HoveredTile::default())
            .insert_resource(EntitySelection::default())
            .insert_resource(SelectedEntity::default())
            .add_system_set(
                SystemSet::on_update(crate::AppState::Room)
                    .with_system(update_selected_tile_system.system())
                    .with_system(select_tile_system.system()),
            );
    }
}
