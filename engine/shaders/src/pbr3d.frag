#version 450

const float PI = 3.14159265359;
const int MAX_POINT_LIGHTS = 8;
const int MAX_SPOT_LIGHTS = 8;

// Uses the existing material descriptor layout. Bind a valid fallback texture
// even when disabled. Base-color RGB is decoded by the sRGB image format.
layout(set = 1, binding = 0) uniform sampler2D baseColorSampler;

// Pbr3D requires this additional fragment-visible camera UBO (16 bytes).
// set 0, binding 0 remains the vertex shader's view/projection UBO.
layout(std140, set = 0, binding = 1) uniform CameraUniform {
  vec4 worldPosition;
} camera;

// Same layout and light intensity/range conventions as Lit3D.
layout(std140, set = 2, binding = 0) uniform LightUniform {
  vec4 directionalDirection;
  vec4 directionalColor;
  vec4 ambientColor;
  // x = spot light count, y = point light count
  vec4 lightParams;
  vec4 pointLightPositions[MAX_POINT_LIGHTS];
  vec4 pointLightColors[MAX_POINT_LIGHTS];
  vec4 spotLightPositions[MAX_SPOT_LIGHTS];
  vec4 spotLightDirections[MAX_SPOT_LIGHTS];
  vec4 spotLightColors[MAX_SPOT_LIGHTS];
  vec4 spotLightConeParams[MAX_SPOT_LIGHTS];
} light;

// Vertex model matrix occupies bytes 0..63.
// Pbr3D must upload 48 fragment bytes at offset 64; total range is 112 bytes.
// Colors are linear. Only the base-color texture is supported so far;
// other texture maps, alpha modes and IBL are not implemented yet.
layout(push_constant) uniform PbrMaterialPushConstants {
  layout(offset = 64) vec4 baseColor;
  // x = metallic, y = roughness, z = use base-color texture (0/1), w reserved
  layout(offset = 80) vec4 materialParams;
  layout(offset = 96) vec4 emissive; // rgb = color, a = strength
} pcs;

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec2 fragTexCoord;
layout(location = 2) in vec3 fragNormal;
layout(location = 3) in vec3 fragWorldPos;
layout(location = 0) out vec4 outColor;

vec3 safeNormalize(vec3 value) {
  float lengthSquared = dot(value, value);
  return value * inversesqrt(max(lengthSquared, 1e-12));
}

vec3 fresnelSchlick(float cosine, vec3 f0) {
  float grazing = pow(1.0 - clamp(cosine, 0.0, 1.0), 5.0);
  return f0 + (vec3(1.0) - f0) * grazing;
}

// Cook-Torrance: GGX distribution, height-correlated Smith visibility,
// Schlick Fresnel. Reference: https://google.github.io/filament/Filament.md
vec3 evaluateLight(vec3 N, vec3 V, vec3 L, vec3 radiance,
                   vec3 baseColor, float metallic, float roughness, vec3 f0) {
  float NoL = clamp(dot(N, L), 0.0, 1.0);
  float NoV = clamp(dot(N, V), 0.0, 1.0);
  if (NoL <= 0.0 || NoV <= 0.0) {
    return vec3(0.0);
  }

  vec3 H = safeNormalize(V + L);
  float NoH = clamp(dot(N, H), 0.0, 1.0);
  float VoH = clamp(dot(V, H), 0.0, 1.0);
  float alpha = roughness * roughness;
  float alphaSquared = alpha * alpha;

  // This form avoids cancellation at NoH = 1 for smooth surfaces.
  float denominator = (1.0 - NoH * NoH) + NoH * NoH * alphaSquared;
  float distribution = alphaSquared / (PI * denominator * denominator);
  float viewMasking = NoL * sqrt(NoV * NoV * (1.0 - alphaSquared) + alphaSquared);
  float lightMasking = NoV * sqrt(NoL * NoL * (1.0 - alphaSquared) + alphaSquared);
  float visibility = 0.5 / max(viewMasking + lightMasking, 1e-7);

  vec3 fresnel = fresnelSchlick(VoH, f0);
  vec3 specular = distribution * visibility * fresnel;
  vec3 diffuse = (vec3(1.0) - fresnel) * (1.0 - metallic) * baseColor / PI;
  return (diffuse + specular) * radiance * NoL;
}

float rangeAttenuation(float distanceToLight, float radius) {
  if (radius <= 0.0) {
    return 0.0;
  }
  float falloff = clamp(1.0 - distanceToLight / radius, 0.0, 1.0);
  return falloff * falloff;
}

void main() {
  vec3 N = safeNormalize(fragNormal);
  vec3 V = safeNormalize(camera.worldPosition.xyz - fragWorldPos);
  vec4 base = pcs.baseColor;
  if (pcs.materialParams.z > 0.5) {
    base *= texture(baseColorSampler, fragTexCoord);
  }
  vec3 baseColor = clamp(base.rgb * fragColor, 0.0, 1.0);
  float metallic = clamp(pcs.materialParams.x, 0.0, 1.0);
  // Avoid the singular perfectly smooth GGX distribution.
  float roughness = clamp(pcs.materialParams.y, 0.045, 1.0);
  vec3 f0 = mix(vec3(0.04), baseColor, metallic);

  vec3 color = evaluateLight(N, V, safeNormalize(-light.directionalDirection.xyz),
      light.directionalColor.rgb * light.directionalColor.a,
      baseColor, metallic, roughness, f0);

  int pointCount = int(clamp(light.lightParams.y, 0.0, float(MAX_POINT_LIGHTS)));
  for (int i = 0; i < pointCount; ++i) {
    vec3 toLight = light.pointLightPositions[i].xyz - fragWorldPos;
    float attenuation = rangeAttenuation(length(toLight), light.pointLightPositions[i].w);
    if (attenuation <= 0.0) {
      continue;
    }
    vec3 radiance = light.pointLightColors[i].rgb * light.pointLightColors[i].a * attenuation;
    color += evaluateLight(N, V, safeNormalize(toLight), radiance,
        baseColor, metallic, roughness, f0);
  }

  int spotCount = int(clamp(light.lightParams.x, 0.0, float(MAX_SPOT_LIGHTS)));
  for (int i = 0; i < spotCount; ++i) {
    vec3 toLight = light.spotLightPositions[i].xyz - fragWorldPos;
    float attenuation = rangeAttenuation(length(toLight), light.spotLightPositions[i].w);
    if (attenuation <= 0.0) {
      continue;
    }
    vec3 L = safeNormalize(toLight);
    float theta = dot(-L, safeNormalize(light.spotLightDirections[i].xyz));
    float inner = light.spotLightConeParams[i].x;
    float outer = light.spotLightConeParams[i].y;
    // Equal cutoffs represent a hard cone; avoid undefined smoothstep edges.
    float cone = inner > outer ? smoothstep(outer, inner, theta) : step(outer, theta);
    vec3 radiance = light.spotLightColors[i].rgb * light.spotLightColors[i].a * attenuation * cone;
    color += evaluateLight(N, V, L, radiance, baseColor, metallic, roughness, f0);
  }

  // Temporary diffuse-only ambient approximation, not environment reflections.
  vec3 ambientFresnel = fresnelSchlick(max(dot(N, V), 0.0), f0);
  color += light.ambientColor.rgb * light.ambientColor.a
      * (vec3(1.0) - ambientFresnel) * (1.0 - metallic) * baseColor;
  color += max(pcs.emissive.rgb, vec3(0.0)) * max(pcs.emissive.a, 0.0);

  // Linear HDR output. Apply tone mapping in a later display pass, and sRGB
  // conversion exactly once (e.g. via an sRGB attachment). No gamma pow here.
  outColor = vec4(color, clamp(base.a, 0.0, 1.0));
}
