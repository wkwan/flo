#version 450

#include "common/materials.glsl"

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec2 fragTexCoord;
layout(location = 2) flat in uint fragTextureIndex;

layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2DArray texSampler;

void main() {
    vec4 texColor = texture(texSampler, vec3(fragTexCoord, float(fragTextureIndex)));
    vec3 litColor = calculateSimpleLighting(fragNormal, texColor.rgb);
    float ambient = 0.3;
    vec3 finalColor = texColor.rgb * ambient + litColor * (1.0 - ambient);
    outColor = vec4(finalColor, texColor.a);
}