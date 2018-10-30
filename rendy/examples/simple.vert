#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec4 inColor;

layout(location = 0) out vec4 fragColor;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    fragColor = inColor;
    gl_Position = vec4(inPosition, 1.0);
}
