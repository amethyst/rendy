#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 pos;
layout(location = 1) in vec2 in_uv;
layout(location = 0) out vec2 uv;

void main() {
    uv = in_uv;
    gl_Position = vec4(pos, 1.0);
}
