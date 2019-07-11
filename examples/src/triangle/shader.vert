#version 450

layout(location = 0) in vec3 pos;
layout(location = 1) in vec4 color;
layout(location = 0) out vec4 frag_color;

void main() {
    frag_color = color;
    gl_Position = vec4(pos, 1.0);
}
