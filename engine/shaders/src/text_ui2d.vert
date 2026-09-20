#version 450

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inUV;

layout(location = 0) out vec2 fragUV;
layout(location = 1) out vec4 fragColor;

layout(push_constant) uniform PushConstants {
    mat4 transform;    // offset 0
    vec4 color;        // offset 64：RGBと透明度
    vec2 viewportSize; // offset 80：描画領域の幅・高さ
} pcs;

void main() {
    // テキストのローカル座標 → 画面内のピクセル座標
    vec4 pixelPosition =
        pcs.transform * vec4(inPosition, 0.0, 1.0);

    // 左上原点。正の高さのVulkan viewportを使う前提
    vec2 ndc =
        pixelPosition.xy / pcs.viewportSize * 2.0 - 1.0;

    gl_Position = vec4(ndc, 0.0, 1.0);

    fragUV = inUV;
    fragColor = vec4(
        inColor * pcs.color.rgb,
        pcs.color.a
    );
}