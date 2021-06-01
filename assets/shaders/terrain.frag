#version 450

layout(location = 0) in vec4 V_Color;
layout(location = 0) out vec4 o_Target;

void main() {
    o_Target = V_Color;
}
