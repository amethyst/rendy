#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(early_fragment_tests) in;

layout(location = 0) in vec2 uv;
// layout(set = 0, binding = 0) uniform sampler2D bunny_image;
layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(gl_FragCoord.zzz, 1.0); //texture(bunny_image, uv);
}
