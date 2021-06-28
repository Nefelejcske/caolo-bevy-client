use bevy::{
    prelude::*,
    render::camera::{Camera, CameraProjection, PerspectiveProjection},
};

use crate::{camera_control::RoomCameraTag, caosim::cao_sim_model::AxialPos};

#[derive(Default, Debug, Clone, Copy)]
pub struct SelectedTile {
    pub axial: AxialPos,
    pub world_pos: Vec3,
}

fn window_to_world(
    pos: Vec2,
    window: &Window,
    cam: &GlobalTransform,
    proj: &PerspectiveProjection,
) -> Vec3 {
    // normalized device coordinates
    let norm = Vec3::new(
        (2.0 * pos.x) / window.width() - 1.,
        (2.0 * pos.y) / window.height() - 1.,
        proj.near,
    );

    let ndc_to_world = cam.compute_matrix() * proj.get_projection_matrix().inverse();
    ndc_to_world.project_point3(norm)
}

/// intersect a given AB line with the plane of the terrain
fn intersect_line_terrain_plain(a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;

    // intersect against plane
    // n=<0, 1, 0>
    // d=-1
    let n = Vec3::Y;
    let t = (-1.0 - n.dot(a)) / n.dot(ab);

    a + t * ab
}

fn update_selected_tile_system(
    mut st: ResMut<SelectedTile>,
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
        app.insert_resource(SelectedTile::default()).add_system_set(
            SystemSet::on_update(crate::AppState::Room)
                .with_system(update_selected_tile_system.system()),
        );
    }
}
