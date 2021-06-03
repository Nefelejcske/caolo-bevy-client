#version 450

layout(location = 0) in vec4 V_Color;
layout(location = 1) in vec3 V_Norm;
layout(location = 0) out vec4 o_Target;

// directional light
#define LIGHT normalize(vec3(8., 1., 1.))
#define MIN_INTENSITY 0.32

void main() {
    float intensity = dot(LIGHT, V_Norm);
    intensity = max(intensity, MIN_INTENSITY);
    vec3 color = V_Color.rgb * intensity;
    color = smoothstep(0., 0.3, color);
    o_Target = vec4(color, 1.0);
}
