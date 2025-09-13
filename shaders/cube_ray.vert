#version 450

#include "common/matrices.glsl"

layout(push_constant) uniform PushConstants {
    float time;
} push;

layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec3 fragPos;
layout(location = 2) out vec3 fragWorldPos;  // Additional output for better raytracing
layout(location = 3) out float objectType;   // 0.0 = cube, 1.0 = plane

void main() {
    // Cube vertices (24 vertices) - centered at origin
    vec3 cubePositions[24] = vec3[24](
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
    
    // Plane vertices (4 vertices for a large ground plane) - positive Y values
    vec3 planePositions[4] = vec3[4](
        vec3(-4.0, 2.0, -4.0),  // Bottom left
        vec3( 4.0, 2.0, -4.0),  // Bottom right
        vec3( 4.0, 2.0,  4.0),  // Top right
        vec3(-4.0, 2.0,  4.0)   // Top left
    );
    
    vec3 cubeNormals[6] = vec3[6](
        vec3( 0.0,  0.0,  1.0),  // Front
        vec3( 0.0,  0.0, -1.0),  // Back
        vec3( 0.0,  1.0,  0.0),  // Top
        vec3( 0.0, -1.0,  0.0),  // Bottom
        vec3( 1.0,  0.0,  0.0),  // Right
        vec3(-1.0,  0.0,  0.0)   // Left
    );
    
    vec3 planeNormal = vec3(0.0, 1.0, 0.0);  // Upward normal for plane
    
    // Combined indices: 36 for cube + 6 for plane = 42 total
    uint indices[42] = uint[42](
        // Cube faces (36 indices)
        0, 1, 2, 2, 3, 0,   // Front
        4, 5, 6, 6, 7, 4,   // Back
        8, 9, 10, 10, 11, 8, // Top
        12, 13, 14, 14, 15, 12, // Bottom
        16, 17, 18, 18, 19, 16, // Right
        20, 21, 22, 22, 23, 20, // Left
        
        // Plane (6 indices - 2 triangles)
        24, 25, 26, 26, 27, 24  // Plane (using indices 24-27)
    );
    
    uint vertexIndex = indices[gl_VertexIndex];
    vec3 position;
    vec3 normal;
    float objType;
    
    if (vertexIndex < 24) {
        // Cube vertex
        position = cubePositions[vertexIndex];
        normal = cubeNormals[vertexIndex / 4];
        objType = 0.0;  // Cube
    } else {
        // Plane vertex
        position = planePositions[vertexIndex - 24];
        normal = planeNormal;
        objType = 1.0;  // Plane
    }
    
    // Create view matrix with slight camera movement for dynamic effect
    vec3 cameraPos = vec3(0.0, 0.0, -3.0 + sin(push.time * 0.5) * 0.2);
    mat4 view = getViewMatrix(cameraPos);
    
    // Transform based on object type
    mat4 model;
    if (objType < 0.5) {
        // Cube - apply rotation around its center (origin)
        float angleY = push.time + 0.785398;
        float angleX = push.time * 0.7 + 0.5236;
        
        mat4 rotationY = getRotationMatrixY(angleY);
        mat4 rotationX = getRotationMatrixX(angleX);
        
        // Apply rotation around center (no translation needed since cube is centered)
        model = rotationX * rotationY;
    } else {
        // Plane - no rotation, fixed position
        model = mat4(1.0);
    }
    
    mat4 projection = getProjectionMatrix();
    
    // Calculate world position
    vec4 worldPos = model * vec4(position, 1.0);
    fragWorldPos = worldPos.xyz;
    
    // Calculate transformed normal
    mat3 normalMatrix = mat3(model);
    fragNormal = normalize(normalMatrix * normal);
    
    // Pass world position for raytracing calculations
    fragPos = worldPos.xyz;
    
    // Pass object type to fragment shader
    objectType = objType;
    
    // Calculate final position
    gl_Position = projection * view * worldPos;
}
