#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 color;
layout(location = 0) out vec4 frag_color;

vec2 pos[6] = {
    vec2(0.0, 0.0),
    vec2(0.0, 1.0),
    vec2(1.0, 1.0),
    vec2(0.0, 0.0),
    vec2(1.0, 1.0),
    vec2(1.0, 0.0),
};

void main() {
    frag_color = color;
    gl_Position = vec4(pos[gl_VertexIndex], 0.0, 1.0);
}
