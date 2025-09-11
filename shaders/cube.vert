#version 450

#include "common/matrices.glsl"

layout(push_constant) uniform PushConstants {
    float time;
} push;

layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec3 fragPos;

void main() {
    vec3 positions[24] = vec3[24](
        vec3(-0.5, -0.5,  0.5),
        vec3( 0.5, -0.5,  0.5),
        vec3( 0.5,  0.5,  0.5),
        vec3(-0.5,  0.5,  0.5),
        
        vec3( 0.5, -0.5, -0.5),
        vec3(-0.5, -0.5, -0.5),
        vec3(-0.5,  0.5, -0.5),
        vec3( 0.5,  0.5, -0.5),
        
        vec3(-0.5,  0.5,  0.5),
        vec3( 0.5,  0.5,  0.5),
        vec3( 0.5,  0.5, -0.5),
        vec3(-0.5,  0.5, -0.5),
        
        vec3(-0.5, -0.5, -0.5),
        vec3( 0.5, -0.5, -0.5),
        vec3( 0.5, -0.5,  0.5),
        vec3(-0.5, -0.5,  0.5),
        
        vec3( 0.5, -0.5,  0.5),
        vec3( 0.5, -0.5, -0.5),
        vec3( 0.5,  0.5, -0.5),
        vec3( 0.5,  0.5,  0.5),
        
        vec3(-0.5, -0.5, -0.5),
        vec3(-0.5, -0.5,  0.5),
        vec3(-0.5,  0.5,  0.5),
        vec3(-0.5,  0.5, -0.5)
    );
    
    vec3 normals[6] = vec3[6](
        vec3( 0.0,  0.0,  1.0),
        vec3( 0.0,  0.0, -1.0),
        vec3( 0.0,  1.0,  0.0),
        vec3( 0.0, -1.0,  0.0),
        vec3( 1.0,  0.0,  0.0),
        vec3(-1.0,  0.0,  0.0)
    );
    
    uint indices[36] = uint[36](
        0, 2, 1, 0, 3, 2,
        4, 6, 5, 4, 7, 6,
        8, 10, 9, 8, 11, 10,
        12, 14, 13, 12, 15, 14,
        16, 18, 17, 16, 19, 18,
        20, 22, 21, 20, 23, 22
    );
    
    uint vertexIndex = indices[gl_VertexIndex];
    vec3 position = positions[vertexIndex];
    fragNormal = normals[vertexIndex / 4];
    
    mat4 view = getViewMatrix(vec3(0.0, 0.0, -3.0));
    
    float angle = push.time + 0.785398;
    mat4 rotationY = getRotationMatrixY(angle);
    
    angle = push.time * 0.7 + 0.5236;
    mat4 rotationX = getRotationMatrixX(angle);
    
    mat4 projection = getProjectionMatrix();
    
    vec4 worldPos = rotationX * rotationY * vec4(position, 1.0);
    fragPos = worldPos.xyz;
    fragNormal = mat3(rotationX * rotationY) * fragNormal;
    
    gl_Position = projection * view * worldPos;
}