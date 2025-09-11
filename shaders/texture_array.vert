#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inTexCoord;
layout(location = 3) in uint inTextureIndex;

layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec2 fragTexCoord;
layout(location = 2) flat out uint fragTextureIndex;

layout(push_constant) uniform PushConstants {
    mat4 viewProj;
} pc;

void main() {
    gl_Position = pc.viewProj * vec4(inPosition, 1.0);
    fragNormal = inNormal;
    fragTexCoord = inTexCoord;
    fragTextureIndex = inTextureIndex;
}