#version 450
#extension GL_ARB_separate_shader_objects : enable
// layout(set = 0, binding = 1, rgba32f) uniform imageBuffer bunnies;

vec2 positions[6] = vec2[](
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(1.0, 1.0),
    vec2(1.0, 1.0),
    vec2(0.0, 1.0),
    vec2(0.0, 0.0)
);

layout(location = 0) out vec2 uv;

void main() {
    float isin = sin(gl_InstanceIndex * 3469);
    float icos = cos(gl_InstanceIndex * 7901);
    vec2 vertex = positions[gl_VertexIndex];
    vec4 bunny = vec4(isin, icos, 0.0, 0.0);// imageLoad(bunnies, gl_InstanceIndex);
    vec2 pos = bunny.rg;
    pos += vertex * 0.003;
    pos = pos / 1.003 * 2.0 - 1.0;
    uv = vertex;
    gl_Position = vec4(pos, isin * icos, 1.0);
}
