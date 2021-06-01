#version 450

layout(location = 0) in vec2 V_Uv;
layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform TerrainMaterial_color {
    vec4 Color;
};

void main() {
    o_Target = Color;
}
