#version 450

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inUV;

layout(location = 0) out vec2 fragUV;
layout(location = 1) out vec4 fragColor;

layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 view;
    mat4 proj;
} ubo;

layout(push_constant) uniform PushConstants {
    mat4 model; // offset 0
    vec4 color; // offset 64
} pcs;

void main() {
    vec4 localPosition = vec4(
        0.0,
        inPosition.x,
        -inPosition.y,
        1.0
    );

    gl_Position =
        ubo.proj * ubo.view * pcs.model * localPosition;

    fragUV = inUV;
    fragColor = vec4(
        inColor * pcs.color.rgb,
        pcs.color.a
    );
}