#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 color;
layout(location = 0) out vec4 frag_color;

vec2 vertices[6] = {
    vec2(0.0, 0.0),
    vec2(0.0, 0.1),
    vec2(0.1, 0.1),
    vec2(0.0, 0.0),
    vec2(0.1, 0.1),
    vec2(0.1, 0.0),
};

void main() {
    vec2 pos = vec2(float(gl_InstanceIndex) / 9.0, float(gl_InstanceIndex) / 9.0);
    vec2 vertex = vertices[gl_VertexIndex];

    vec2 v = ((vertex + pos / 1.1) * 2.0) - vec2(1.0, 1.0);

    frag_color = vec4(color.rgb / float(gl_InstanceIndex + 1), 1.0);
    gl_Position = vec4(v * vec2(1.0, -1.0), 0.0, 1.0);
}
