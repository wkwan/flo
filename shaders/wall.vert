#version 450

#include "common/matrices.glsl"

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

layout(push_constant) uniform PushConstants {
    float time;
    float cameraPositionX;
    float cameraPositionY;
    float cameraPositionZ;
    vec2 resolution;
    float waterLevel;
    float gridScale;
} push;

layout(location = 0) out vec3 fragWorldPos;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out vec2 fragUV;

void main() {
    vec3 worldPos = inPosition;
    
    // Same view and projection as water - use individual camera position components
    mat4 view = getViewMatrix(vec3(-push.cameraPositionX, -push.cameraPositionY, -push.cameraPositionZ));
    
    // Projection matrix using actual aspect ratio from resolution
    float aspectRatio = push.resolution.x / push.resolution.y;
    mat4 projection = mat4(
        1.0 / aspectRatio, 0.0, 0.0, 0.0,
        0.0, -1.0, 0.0, 0.0,
        0.0, 0.0, -1.0, -1.0,
        0.0, 0.0, -0.2, 0.0
    );
    
    fragWorldPos = worldPos;
    fragNormal = inNormal;
    fragUV = inUV;
    
    gl_Position = projection * view * vec4(worldPos, 1.0);
}