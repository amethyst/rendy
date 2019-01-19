#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 color;
layout(location = 0) out vec4 frag_color;

layout(std430, set = 0, binding = 0) buffer _ {
    vec4 posvel[];
} posvelbuff;

vec2 vertices[6] = {
    vec2(0.00, 0.00),
    vec2(0.00, 0.01),
    vec2(0.01, 0.01),
    vec2(0.00, 0.00),
    vec2(0.01, 0.01),
    vec2(0.01, 0.00),
};

void main() {
    vec4 posvel = posvelbuff.posvel[gl_InstanceIndex];
    vec2 pos = posvel.rg;
    vec2 vertex = vertices[gl_VertexIndex];

    vec2 v = ((vertex + pos / 1.01) * 2.0) - vec2(1.0, 1.0);
    v.y = -v.y;

    frag_color = vec4(color.rgb, 1.0);
    gl_Position = vec4(v, 0.0, 1.0);
}
