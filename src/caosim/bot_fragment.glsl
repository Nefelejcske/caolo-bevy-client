#version 450

layout(location = 0) out vec4 o_Target;
layout(set = 1, binding = 1) uniform BotMaterial_color {
    vec4 color;
};

void main() {
    o_Target = color;
}
