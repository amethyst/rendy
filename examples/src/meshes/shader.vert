#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 color;
layout(location = 2) in vec3 normal;
// vec4[4] is used instead of mat4 due to spirv-cross bug for dx12 backend
layout(location = 3) in vec4 model_1; // per-instance.
layout(location = 4) in vec4 model_2; // per-instance.
layout(location = 5) in vec4 model_3; // per-instance.
layout(location = 6) in vec4 model_4; // per-instance.

struct Light {
    vec3 pos;
    float pad;
    float intencity;
};

layout(std140, set = 0, binding = 0) uniform Args {
    mat4 proj;
    mat4 view;
    int lights_count;
    int pad1;
    int pad2;
    int pad3;
    Light lights[32];
};

layout(location = 0) out vec4 frag_pos;
layout(location = 1) out vec3 frag_norm;
layout(location = 2) out vec4 frag_color;

void main() {
    mat4 model_mat = mat4(model_1, model_2, model_3, model_4);
    frag_color = color;
    frag_norm = normalize((vec4(normal, 1.0) * model_mat).xyz);
    frag_pos = model_mat * vec4(position, 1.0);
    gl_Position = proj * view * frag_pos;
}
