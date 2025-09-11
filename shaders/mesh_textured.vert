#version 450

#include "common/matrices.glsl"
#include "common/constants.glsl"

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

layout(push_constant) uniform PushConstants {
    float time;
} push;

layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec3 fragPos;
layout(location = 2) out vec2 fragUV;

void main() {
    vec3 position = inPosition;
    vec3 normal = inNormal;
    
    mat4 view = getViewMatrix(vec3(0.0, -0.5, -4.0));
    
    float angle = push.time * 0.5;
    mat4 rotationY = getRotationMatrixY(angle);
    mat4 rotationX = getRotationMatrixX(PI); // 180 degree rotation to flip vertically
    
    mat4 projection = getProjectionMatrix();
    
    vec4 worldPos = rotationY * rotationX * vec4(position, 1.0);
    fragPos = worldPos.xyz;
    fragNormal = mat3(rotationY * rotationX) * normal;
    fragUV = inUV;
    
    gl_Position = projection * view * worldPos;
}