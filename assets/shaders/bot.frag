#version 450

layout(location = 0) in vec2 V_Uv;
layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform BotMaterial_color {
    vec4 Color;
};
layout(set = 2, binding = 1) uniform BotMaterial_time {
    float Time;
};

float frag(float f) {
    return f - floor(f);
}

void main() {
    float t = frag(Time) * 0.25;
    vec2 disp = sin(V_Uv) * 1.2; // + vec2(t, t);
    o_Target = Color * dot(disp, disp);
}
