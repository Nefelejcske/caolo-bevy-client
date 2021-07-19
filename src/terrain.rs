mod terrain_assets;

use bevy::{
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph,
    },
};

use crate::{
    bots::pos_2d_to_3d,
    cao_sim_client::{cao_client::CaoClient, Connected},
};
use crate::{
    cao_sim_client::{cao_sim_model::TerrainTy, hex_axial_to_pixel, NewTerrain},
    room_interaction::HoveredTile,
};

pub struct TerrainPlugin;
pub struct CurrentRoom(pub crate::cao_sim_client::cao_sim_model::AxialPos);

pub struct Room;

fn terrain2color(ty: TerrainTy) -> Color {
    match ty {
        TerrainTy::Empty => Color::rgba(0.0, 0.0, 0.0, 0.0),
        TerrainTy::Plain => Color::rgb(0.4, 0.3, 0.0),
        TerrainTy::Wall => Color::rgb(0.5, 0.1, 0.0),
        TerrainTy::Bridge => Color::rgb(0.0, 0.8, 0.0),
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

fn on_enter_system(current_room: Res<CurrentRoom>, client: Res<CaoClient>) {
    info!("Sending initial room");
    client.send_subscribe_room(current_room.0);
}

fn on_reconnect_system(
    current_room: Res<CurrentRoom>,
    client: Res<CaoClient>,
    mut on_reconnect: EventReader<Connected>,
) {
    for _ in on_reconnect.iter() {
        info!("Reconnect event received, sending current room");
        client.send_subscribe_room(current_room.0);
    }
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

fn on_new_terrain_system(
    mut cmd: Commands,
    mut new_terrain: EventReader<NewTerrain>,
    assets: Res<terrain_assets::TerrainRenderingAssets>,
    mut materials: ResMut<Assets<terrain_assets::TerrainMaterial>>,
    existing_tiles: Query<Entity, With<Room>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for new_terrain in new_terrain.iter() {
        info!("Got new terrain {:?}", new_terrain.room_id);
        let start = std::time::Instant::now();

        for e in existing_tiles.iter() {
            // TODO: maybe update these instead of despawning all...?
            cmd.entity(e).despawn_recursive();
        }

        let mut vertices = Vec::with_capacity(new_terrain.terrain.len() * 6);
        let mut indices = Vec::with_capacity(new_terrain.terrain.len() * 6);
        let mut colors = Vec::with_capacity(new_terrain.terrain.len() * 6);
        let mut normals = Vec::with_capacity(new_terrain.terrain.len() * 6);
        for (p, ty) in new_terrain.terrain.iter() {
            let p = &p;
            let p = hex_axial_to_pixel(p.q as f32, p.r as f32);
            let mut p = pos_2d_to_3d(p);
            p.y -= 1.0;

            let color = terrain2color(*ty);

            let ys: &[f32] = match *ty {
                TerrainTy::Wall => &[-1., 0.34],
                _ => &[-1.],
            };
            let l = ys.len();
            let vertex0ind = vertices.len() as u16;

            _build_hex_prism_bases(
                ys,
                p,
                color,
                0.95,
                &mut vertices,
                &mut indices,
                &mut colors,
                &mut normals,
            );

            debug_assert!(l <= 2);
            if l == 2 {
                _build_hex_prism_sides(vertex0ind, &mut indices);
            }
        }
        let mut mesh = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.set_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_indices(Some(bevy::render::mesh::Indices::U16(indices)));

        let mesh = meshes.add(mesh);

        let material = materials.add(terrain_assets::TerrainMaterial {
            cursor_pos: Vec3::ZERO,
        });

        cmd.spawn_bundle(MeshBundle {
            mesh,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                assets.pipeline.clone_weak(),
            )]),
            ..Default::default()
        })
        .insert(material)
        .insert(Room);
        let end = std::time::Instant::now();

        let dur = end - start;
        info!("New terrain processing done in {:?}", dur);
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

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set(
                SystemSet::on_enter(crate::AppState::Room).with_system(on_enter_system.system()),
            )
            .add_system_set(
                SystemSet::on_update(crate::AppState::Room)
                    .with_system(on_new_terrain_system.system())
                    .with_system(update_terrain_material_system.system())
                    .with_system(on_reconnect_system.system()),
            )
            .init_resource::<terrain_assets::TerrainRenderingAssets>()
            .insert_resource(CurrentRoom(
                crate::cao_sim_client::cao_sim_model::AxialPos { q: 15, r: 15 },
            ))
            .add_asset::<terrain_assets::TerrainMaterial>();
    }
}
