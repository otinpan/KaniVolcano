#version 450

layout(set = 0, binding = 0) uniform GlobalUniform {
  mat4 view;
  mat4 proj;
} ubo;

layout(push_constant) uniform PushConstants {
  mat4 model;
} pcs;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inTexCoord;
layout(location = 3) in vec3 inNormal;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 fragTexCoord;
layout(location = 2) out vec3 fragNormal;
layout(location = 3) out vec3 fragWorldPos;

void main() {
  vec4 worldPos = pcs.model * vec4(inPosition, 1.0);
  mat3 normalMatrix = transpose(inverse(mat3(pcs.model)));

  gl_Position = ubo.proj * ubo.view * worldPos;
  fragColor = inColor;
  fragTexCoord = inTexCoord;
  fragNormal = normalize(normalMatrix * inNormal);
  fragWorldPos = worldPos.xyz;
}
