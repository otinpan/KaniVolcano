#version 450

layout(location = 0) in vec2 fragUV;
layout(location = 1) in vec4 fragColor;

layout(set = 1, binding = 0) uniform sampler2D glyphAtlas;

layout(location = 0) out vec4 outColor;

void main() {
    // read specified region from glyphAtlas using mesh data.
    float coverage = texture(glyphAtlas, fragUV).r;

    outColor = vec4(
        fragColor.rgb,
        fragColor.a * coverage
    );
}