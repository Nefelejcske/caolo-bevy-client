#version 450

layout(location = 0) out vec4 o_Target;
layout(set = 1, binding = 1) uniform TerrainMaterial_color {
    vec4 color;
};

void main() {
    vec2 coord = gl_FragCoord.xy;
    o_Target = color + vec4(coord, 0., 0.);
}
