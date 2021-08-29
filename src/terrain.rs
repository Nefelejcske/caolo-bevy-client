mod terrain_assets;

use std::{
    collections::HashSet,
    time::{self, Duration},
};

use bevy::{
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;

use crate::{
    cao_entities::pos_2d_to_3d,
    cao_sim_client::{
        cao_client::CaoClient,
        cao_sim_model::{AxialPos, EntityPosition, TerrainTy},
        hex_axial_to_pixel, Connected, NewTerrain,
    },
    room_interaction::HoveredTile,
};
use lru::LruCache;

pub struct TerrainPlugin;

struct LastY(pub f32);
struct NextY(pub f32);
struct AnimTimer(Timer);

#[derive(Debug, Clone, Copy)]
pub struct CurrentRoom {
    pub room_id: AxialPos,
    pub visible_range: u32,
}
pub struct NewCurrentRoom(pub AxialPos);

pub struct Room(pub AxialPos);

#[derive(Debug, Clone, Copy)]
pub struct RoomMeta {
    pub offset: AxialPos,
    pub entity: Entity,
}

/// room_id → metadata
pub struct RoomData(pub LruCache<AxialPos, RoomMeta>);

fn terrain2color(ty: TerrainTy) -> Color {
    match ty {
        TerrainTy::Empty => Color::rgba(0.0, 0.0, 0.0, 0.0),
        TerrainTy::Plain => Color::rgb(0.4, 0.3, 0.0),
        TerrainTy::Wall => Color::rgb(0.5, 0.1, 0.0),
        TerrainTy::Bridge => Color::rgb(0.0, 0.8, 0.0),
    }
}

fn room_gc_system(mut cmd: Commands, current_room: Res<CurrentRoom>, q: Query<(Entity, &Room)>) {
    for (e, room) in q.iter() {
        if !is_room_visible(&*current_room, &*room) {
            trace!("Garbage collecting room {:?}", room.0);
            cmd.entity(e).despawn_recursive();
        }
    }
}

fn _build_hex_prism_bases(
    ys: &[f32],
    p: Vec3,
    color: Color,
    size: f32,
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u16>,
    colors: &mut Vec<[f32; 4]>,
    normals: &mut Vec<[f32; 3]>,
) {
    const SQRT3: f32 = 1.732_050_8; // sqrt (3)
    let w: f32 = SQRT3 * size;
    let h: f32 = 2.0 * size;
    for y in ys {
        // flat hexagons
        //
        let vertex0ind = vertices.len() as u16;
        for offset in [
            Vec2::new(0., -h / 2.),
            Vec2::new(w / 2., -h / 4.),
            Vec2::new(w / 2., h / 4.),
            Vec2::new(0., h / 2.),
            Vec2::new(-w / 2., h / 4.),
            Vec2::new(-w / 2., -h / 4.),
        ]
        .iter()
        {
            let p = p + pos_2d_to_3d(*offset);
            vertices.push([p.x, *y, p.z]);
            colors.push([color.r(), color.g(), color.b(), color.a()]);
        }
        // side triangle 0
        indices.push(vertex0ind + 2);
        indices.push(vertex0ind + 1);
        indices.push(vertex0ind + 0);
        // side triangle 1
        indices.push(vertex0ind + 4);
        indices.push(vertex0ind + 3);
        indices.push(vertex0ind + 2);
        // side triangle 2
        indices.push(vertex0ind + 0);
        indices.push(vertex0ind + 5);
        indices.push(vertex0ind + 4);
        // center triangle
        indices.push(vertex0ind + 4);
        indices.push(vertex0ind + 2);
        indices.push(vertex0ind + 0);
    }

    for y in ys {
        normals.extend(
            [
                Vec3::new(0., *y, 1.).normalize(),
                Vec3::new(-0.5, *y, -1.).normalize(),
                Vec3::new(-0.5, *y, 1.).normalize(),
                Vec3::new(0., *y, -1.).normalize(),
                Vec3::new(0.5, *y, 1.).normalize(),
                Vec3::new(0.5, *y, -1.).normalize(),
            ]
            .iter()
            .map(|v| [v.x, v.y, v.z]),
        );
    }
}

/// assumes that the 16 vertices of the two base hexes are contigous, in clockwise order
fn _build_hex_prism_sides(vertex0ind: u16, indices: &mut Vec<u16>) {
    for i in 0..6 {
        let a1 = i + 0;
        let b1 = (i + 1) % 6;
        let a2 = a1 + 6;
        let b2 = b1 + 6;

        indices.push(vertex0ind + a1);
        indices.push(vertex0ind + b1);
        indices.push(vertex0ind + a2);

        indices.push(vertex0ind + b1);
        indices.push(vertex0ind + b2);
        indices.push(vertex0ind + a2);
    }
}

fn on_enter_system(mut new_rooms: EventWriter<NewCurrentRoom>) {
    info!("Sending initial room");
    // TODO:
    // some smarter way to get the initial room...
    new_rooms.send(NewCurrentRoom(AxialPos { q: 17, r: 10 }));
}

fn on_reconnect_system(
    current_room: Res<CurrentRoom>,
    mut on_reconnect: EventReader<Connected>,
    mut new_rooms: EventWriter<NewCurrentRoom>,
) {
    if on_reconnect.iter().next().is_some() {
        info!("Reconnect event received, sending current room");
        new_rooms.send(NewCurrentRoom(current_room.room_id));
    }
}

pub fn is_room_visible(current: &CurrentRoom, room_id: &Room) -> bool {
    let dq = current.room_id.q - room_id.0.q;
    let dr = current.room_id.r - room_id.0.r;
    let range = current.visible_range as i32;

    dq.abs() <= range && dr.abs() <= range && (dq + dr).abs() <= range
}

fn update_terrain_material_system(
    selected_tile: Res<HoveredTile>,
    mut materials: ResMut<Assets<terrain_assets::TerrainMaterial>>,
    rooms: Query<&Handle<terrain_assets::TerrainMaterial>>,
) {
    for room_mat in rooms.iter() {
        if let Some(mat) = materials.get_mut(room_mat) {
            mat.cursor_pos = selected_tile.world_pos;
        }
    }
}

struct TerrainMeshResult {
    start: time::Instant,
    vertices: [Vec<[f32; 3]>; 2],
    mesh: Mesh,
    id: AxialPos,
    offset: Vec3,
    offset_axial: AxialPos,
}

struct AnimatedVertices {
    from: Vec<[f32; 3]>,
    to: Vec<[f32; 3]>,
}

fn handle_terrain_mesh_tasks_system(
    mut cmd: Commands,
    mut tasks: Query<(Entity, &mut Task<TerrainMeshResult>)>,
    assets: Res<terrain_assets::TerrainRenderingAssets>,
    mut materials: ResMut<Assets<terrain_assets::TerrainMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    existing_rooms: Query<(Entity, &Room)>,
    mut rooms: ResMut<RoomData>,
) {
    for (e, mut task) in tasks.iter_mut() {
        if let Some(mesh) = future::block_on(future::poll_once(&mut *task)) {
            let TerrainMeshResult {
                start,
                mesh,
                id,
                offset,
                offset_axial,
                vertices,
            } = mesh;

            // clean up
            cmd.entity(e).despawn_recursive();

            for (e, room) in existing_rooms.iter() {
                if room.0 == id {
                    cmd.entity(e).despawn_recursive();
                }
            }

            // spawn the new mesh
            let mesh_handle = meshes.add(mesh);

            let material = materials.add(terrain_assets::TerrainMaterial {
                cursor_pos: Vec3::ZERO,
            });

            let transform = Transform::from_translation(offset - Vec3::Y * 30.0);

            let [to, from] = vertices;
            let entity = cmd
                .spawn_bundle(MeshBundle {
                    mesh: mesh_handle,
                    render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                        assets.pipeline.clone_weak(),
                    )]),
                    ..Default::default()
                })
                .insert_bundle((
                    LastY(transform.translation.y),
                    NextY(offset.y),
                    AnimTimer(Timer::new(Duration::from_secs(2), false)),
                    AnimatedVertices { from, to },
                ))
                .insert(material)
                .insert(transform)
                .insert(Room(id))
                .id();

            rooms.0.put(
                id,
                RoomMeta {
                    offset: offset_axial,
                    entity,
                },
            );

            let end = std::time::Instant::now();

            let dur = end - start;
            info!("New terrain processing done in {:?}", dur);
        }
    }
}

/// touch the current room and neighbours in the LRU cache to move them to the top of the LRU so
/// they aren't garbage collected
fn touch_lru_system(current_room: Res<CurrentRoom>, mut rooms: ResMut<RoomData>) {
    rooms.0.get(&current_room.room_id);
    for neighbour in room_neighbours(current_room.room_id) {
        rooms.0.get(&neighbour);
    }
}

fn animate_mesh_system(
    mut meshes: ResMut<Assets<Mesh>>,
    q: Query<(&AnimTimer, &AnimatedVertices, &Handle<Mesh>)>,
) {
    for (t, vert, mesh) in q.iter() {
        debug_assert!(vert.from.len() == vert.to.len());
        let mut v = Vec::with_capacity(vert.from.len());

        let t = ezing::back_out(t.0.percent());

        for ([_, y1, _], [x, y2, z]) in vert.from.iter().zip(vert.to.iter()) {
            v.push([*x, lerp_f32(*y1, *y2, t), *z]);
        }
        let mesh = meshes.get_mut(mesh).unwrap();
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, v);
    }
}

fn remove_stale_animations_system(mut cmd: Commands, q: Query<(Entity, &AnimTimer)>) {
    for (e, t) in q.iter() {
        if t.0.finished() {
            cmd.entity(e)
                .remove_bundle::<(AnimTimer, AnimatedVertices)>();
        }
    }
}

fn on_new_terrain_system(
    mut cmd: Commands,
    mut new_terrain: EventReader<NewTerrain>,
    pool: Res<AsyncComputeTaskPool>,
) {
    for new_terrain in new_terrain.iter() {
        info!("Got new terrain {:?}", new_terrain.room_id);
        let start = std::time::Instant::now();
        let room_id = new_terrain.room_id;
        let offset = new_terrain.offset;
        let new_terrain = new_terrain.terrain.clone();
        let task = pool.spawn(async move {
            use futures_lite::StreamExt;

            let mut vertices_a = Vec::with_capacity(new_terrain.len() * 6);
            let mut vertices_b = Vec::with_capacity(new_terrain.len() * 6);
            let mut indices = Vec::with_capacity(new_terrain.len() * 6);
            let mut colors = Vec::with_capacity(new_terrain.len() * 6);
            let mut normals = Vec::with_capacity(new_terrain.len() * 6);
            let mut stream = futures_lite::stream::iter(new_terrain.as_slice());
            while let Some((p, ty)) = stream.next().await {
                let p = hex_axial_to_pixel(p.q as f32, p.r as f32);
                let mut p = pos_2d_to_3d(p);
                p.y -= 1.0;

                let color = terrain2color(*ty);

                let ys: &[f32] = match *ty {
                    TerrainTy::Wall => &[-1., 0.34],
                    _ => &[-1.],
                };
                let l = ys.len();
                let vertex0ind = vertices_a.len() as u16;

                _build_hex_prism_bases(
                    ys,
                    p,
                    color,
                    0.95,
                    &mut vertices_a,
                    &mut indices,
                    &mut colors,
                    &mut normals,
                );

                let yoffset = -10.0 * fastrand::f32(); // * 2.0 - 1.0); // remap to [-1‥1]
                vertices_b.extend(
                    vertices_a[vertex0ind as usize..]
                        .iter()
                        .map(|[x, y, z]| [*x, *y + yoffset, *z]),
                );

                debug_assert!(l <= 2);
                if l == 2 {
                    _build_hex_prism_sides(vertex0ind, &mut indices);
                }
            }
            let mut mesh = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);
            mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices_a.clone());
            mesh.set_attribute(Mesh::ATTRIBUTE_COLOR, colors);
            mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            mesh.set_indices(Some(bevy::render::mesh::Indices::U16(indices)));

            TerrainMeshResult {
                start,
                vertices: [vertices_a, vertices_b],
                mesh,
                id: room_id,
                offset: pos_2d_to_3d(hex_axial_to_pixel(offset.q as f32, offset.r as f32)),
                offset_axial: offset,
            }
        });

        cmd.spawn().insert(task);
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<bevy::render::pipeline::PipelineDescriptor>>,
    mut render_graph: ResMut<render_graph::RenderGraph>,
    mut terrain_rendering_assets: ResMut<terrain_assets::TerrainRenderingAssets>,
) {
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(
        bevy::render::shader::ShaderStages {
            vertex: asset_server.load::<Shader, _>("shaders/terrain.vert"),
            fragment: Some(asset_server.load::<Shader, _>("shaders/terrain.frag")),
        },
    ));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind BotMaterial resources to our shader
    render_graph.add_system_node(
        "terrain_material",
        render_graph::AssetRenderResourcesNode::<terrain_assets::TerrainMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "terrain_material" node to the main pass node. This ensures "terrain_material" runs before the main pass
    render_graph
        .add_node_edge("terrain_material", render_graph::base::node::MAIN_PASS)
        .unwrap();

    *terrain_rendering_assets = terrain_assets::TerrainRenderingAssets {
        pipeline: pipeline_handle,
    };
}

pub fn room_neighbours(axial: AxialPos) -> [AxialPos; 6] {
    let q = axial.q;
    let r = axial.r;

    [
        AxialPos { q: q + 1, r },
        AxialPos { q: q + 1, r: r - 1 },
        AxialPos { q, r: r - 1 },
        AxialPos { q: q - 1, r },
        AxialPos { q: q - 1, r: r + 1 },
        AxialPos { q, r: r + 1 },
    ]
}

fn update_current_room_system(
    mut cache: Local<(HashSet<AxialPos>, HashSet<AxialPos>)>,
    mut incoming: EventReader<NewCurrentRoom>,
    mut current_room: ResMut<CurrentRoom>,
    client: Res<CaoClient>,
) {
    let (ref mut current_visible_set, ref mut newly_visible_set) = &mut *cache;
    for room in incoming.iter() {
        debug!("Change main room to: {:?}", room.0);
        let currently_visible = room_neighbours(current_room.room_id);
        let newly_visible = room_neighbours(room.0);

        current_visible_set.clear();
        current_visible_set.extend(currently_visible.iter().copied());
        current_visible_set.insert(current_room.room_id);

        newly_visible_set.clear();
        newly_visible_set.extend(newly_visible.iter().copied());
        newly_visible_set.insert(room.0);

        let new_rooms = newly_visible_set.difference(&current_visible_set);
        client.send_subscribe_room_iter(new_rooms.copied());
        let old_rooms = current_visible_set.difference(&newly_visible_set);
        client.send_unsubscribe_rooms_iter(old_rooms.copied());

        current_room.room_id = room.0;
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    t * (b - a) + a
}

fn update_pos_system(
    time: Res<Time>,
    mut query: Query<(&LastY, &NextY, &mut Transform, &mut AnimTimer)>,
) {
    let delta = time.delta();
    for (last, next, mut curr, mut t) in query.iter_mut() {
        t.0.tick(delta);
        curr.translation.y = lerp_f32(last.0, next.0, ezing::elastic_out(t.0.percent()));
    }
}

fn update_entity_positions(
    mut meta: ResMut<RoomData>,
    mut q: Query<(&mut Transform, &EntityPosition)>,
    rooms: Query<&GlobalTransform>,
) {
    for (mut tr, wp) in q.iter_mut() {
        if let Some(room) = meta.0.get(&wp.room) {
            if let Ok(room_tr) = rooms.get(room.entity) {
                tr.translation.y = room_tr.translation.y + 1.0;
            }
        }
    }
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<NewCurrentRoom>()
            .add_startup_system(setup.system())
            .add_system(update_current_room_system.system())
            .add_system(handle_terrain_mesh_tasks_system.system())
            .add_system(room_gc_system.system())
            .add_system(touch_lru_system.system())
            .add_system_set(
                SystemSet::on_enter(crate::AppState::Room).with_system(on_enter_system.system()),
            )
            .add_system_set(
                SystemSet::on_update(crate::AppState::Room)
                    .with_system(on_new_terrain_system.system())
                    .with_system(update_terrain_material_system.system())
                    .with_system(update_pos_system.system())
                    .with_system(update_entity_positions.system())
                    .with_system(animate_mesh_system.system())
                    .with_system(remove_stale_animations_system.system())
                    .with_system(on_reconnect_system.system()),
            )
            .init_resource::<terrain_assets::TerrainRenderingAssets>()
            .insert_resource(CurrentRoom {
                room_id: AxialPos { q: -1, r: -1 },
                visible_range: 1,
            })
            .insert_resource(RoomData(LruCache::new(32)))
            .add_asset::<terrain_assets::TerrainMaterial>();
    }
}
