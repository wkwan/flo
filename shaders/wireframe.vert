#version 450

// Per-vertex attributes
layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec4 in_color;

// Uniforms
layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 view;
    mat4 proj;
    vec3 camera_pos;
    float time;
} ubo;

// Per-instance attributes for instancing
layout(location = 4) in mat4 instance_transform; // Instance transform matrix (4x vec4)
layout(location = 8) in vec4 instance_color;     // Instance color

// Outputs to fragment shader
layout(location = 0) out vec4 fragColor;

void main() {
    // Apply instance transform to vertex position
    vec4 world_pos = instance_transform * vec4(in_position, 1.0);
    
    // Apply view-projection matrix
    gl_Position = ubo.proj * ubo.view * world_pos;
    
    // Pass color to fragment shader
    // Mix vertex color with instance color if needed
    fragColor = instance_color * in_color;
}