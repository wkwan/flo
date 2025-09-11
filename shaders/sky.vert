#version 450

// Fullscreen triangle vertices generated in shader
vec2 positions[3] = vec2[](
    vec2(-1.0, -1.0),
    vec2( 3.0, -1.0),
    vec2(-1.0,  3.0)
);

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec3 fragRayDir;

layout(push_constant) uniform PushConstants {
    float time;
    vec3 cameraPosition;
    vec2 resolution;
    float waterLevel;
    float gridScale;
} pc;

void main() {
    vec2 pos = positions[gl_VertexIndex];
    gl_Position = vec4(pos, 0.999, 1.0); // Place at far depth
    fragTexCoord = pos * 0.5 + 0.5;
    
    // Simple ray direction calculation for sky - flip Y to match expected gradient
    vec3 rayDir = normalize(vec3(pos.x, -pos.y, -2.0));
    fragRayDir = rayDir;
}