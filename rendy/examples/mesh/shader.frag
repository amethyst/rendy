#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(early_fragment_tests) in;

layout(location = 0) in vec3 frag_norm;
layout(location = 1) in vec4 frag_color;
layout(location = 0) out vec4 color;

struct Light {
    vec3 pos;
    float intencity;
};

layout(set = 0, binding = 0) uniform Args {
    mat4 proj;
    mat4 view;
    int lights_count;
    Light lights[32];
};

void main() {
    // float acc = 0.0;
    // vec3 frag_pos = gl_FragCoord.xyz / gl_FragCoord.w;

    // for (int i = 0; i < lights_count; ++i) {
    //     vec3 v = lights[i].pos - frag_pos;
    //     float d = length(v);
    //     float l = lights[i].intencity / d / d;
    //     v = normalize(v);
    //     l *= max(0.0, dot(v, frag_norm));
    //     acc += l;
    // }
    // acc = min(acc, 1.0);
    // color = vec4(acc, acc, acc, 1.0);
    color = frag_color;
}
