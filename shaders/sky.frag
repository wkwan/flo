#version 450

layout(location = 0) in vec2 fragTexCoord;
layout(location = 1) in vec3 fragRayDir;

layout(location = 0) out vec4 outColor;

// Include the common sky function
#include "common/sky.glsl"

void main() {
    vec3 skyColor = getSkyColor(normalize(fragRayDir));
    outColor = vec4(skyColor, 1.0);
}