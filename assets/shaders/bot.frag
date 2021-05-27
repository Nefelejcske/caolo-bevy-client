#version 450

layout(location = 0) in vec2 V_Uv;
layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform BotMaterial_color {
    vec4 Color;
};
layout(set = 2, binding = 1) uniform BotMaterial_time {
    float Time;
};

void main() {
    vec2 disp = V_Uv;
    vec2 invd = vec2(1., 1.) - disp;
    o_Target = Color * smoothstep(0., 2.0, dot(disp, disp)) * 2.2 + vec4(invd.rg, 0., 1.);
}
