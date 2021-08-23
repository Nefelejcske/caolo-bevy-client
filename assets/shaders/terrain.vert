#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec4 Vertex_Color;
layout(location = 2) in vec3 Vertex_Normal;
layout(location = 0) out vec4 V_Color;
layout(location = 1) out vec3 V_Norm;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 u_view_proj;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 u_model;
};
layout(set = 2, binding = 0) uniform TerrainMaterial_cursor_pos {
    vec3 u_cursor;
};
layout(set = 2, binding = 1) uniform TerrainMaterial_is_visible {
    int u_visible;
};

void main() {
    vec4 vp = u_model * vec4(Vertex_Position, 1.0);
    gl_Position = u_view_proj * vp;
    V_Norm = Vertex_Normal;
    V_Color = Vertex_Color;

    if (u_visible == 0) {
        V_Color = vec4(0.2, 0.2, 0.2, 1.0);
    }
}
