#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec4 frag_pos;
layout(location = 1) in vec3 frag_norm;
layout(location = 2) in vec4 frag_color;
layout(location = 0) out vec4 color;

struct Light {
    vec3 pos;
    float pad;
    float intencity;
};

layout(std140, set = 0, binding = 1) uniform FArgs {
    mat4 proj;
    mat4 view;
    int lights_count;
    int pad1;
    int pad2;
    int pad3;
    Light lights[32];
};

void main() {
    float acc = 0.0;

    vec3 frag_pos = frag_pos.xyz / frag_pos.w;

    for (int i = 0; i < lights_count; ++i) {
        vec3 v = lights[i].pos - frag_pos;
        float d = length(v);
        float l = lights[i].intencity / d / d;
        l *= max(0.0, dot(normalize(v), frag_norm));
        acc += l;
        // acc += 0.5;
        // acc += lights[i].intencity;
    }
    acc = min(acc, 1.0);
    color = frag_color * vec4(acc, acc, acc, 1.0);
    // color = frag_color;
}
