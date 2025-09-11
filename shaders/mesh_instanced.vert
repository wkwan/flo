#version 450

#include "common/matrices.glsl"

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;

// Instance attributes
layout(location = 2) in vec3 instancePos;

layout(push_constant) uniform PushConstants {
    float time;
} push;

layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec3 fragPos;

void main() {
    vec3 position = inPosition * 0.2; // Scale down for 1000 instances
    vec3 normal = inNormal;
    
    mat4 view = getViewMatrix(vec3(0.0, 2.0, -20.0)); // Camera further back
    
    float angle = push.time * 0.5;
    mat4 rotationY = getRotationMatrixY(angle);
    
    mat4 translation = mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        instancePos.x, instancePos.y, instancePos.z, 1.0
    );
    
    mat4 projection = getProjectionMatrix();
    
    vec4 worldPos = translation * rotationY * vec4(position, 1.0);
    fragPos = worldPos.xyz;
    fragNormal = mat3(rotationY) * normal;
    
    gl_Position = projection * view * worldPos;
}