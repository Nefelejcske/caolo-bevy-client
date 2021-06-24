#version 450

layout(location = 0) in vec2 V_Uv;
layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform MiningLaserMaterial_color {
    vec4 u_Color;
};
layout(set = 2, binding = 1) uniform MiningLaserMaterial_t {
    float u_T;
};

void main() {
    if (V_Uv.x > u_T) {
        o_Target = vec4(0.0);
        return;
    }
    o_Target = u_Color; // TODO
}
