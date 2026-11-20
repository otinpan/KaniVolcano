#version 450

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inTexCoord;

layout(location = 0) out vec4 fragColor;
layout(location = 1) out vec2 fragTexCoord;

layout(push_constant) uniform PushConstants {
    mat4 model;
} pcs;

void main() {
    // UI vertices occupy the engine's YZ plane. Preserve the existing vertical flip.
    vec4 world = pcs.model * vec4(0.0, inPosition.x, inPosition.y, 1.0);
    gl_Position = vec4(world.y, -world.z, 0.0, 1.0);
    fragColor = vec4(inColor, 1.0);
    fragTexCoord = inTexCoord;
}
