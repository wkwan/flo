#version 450

#include "common/lighting.glsl"

layout(binding = 0) uniform sampler2D texSampler;

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec3 fragPos;
layout(location = 2) in vec2 fragUV;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 lightDir = normalize(vec3(0.5, 1.0, 0.8));
    vec3 normal = normalize(fragNormal);
    
    // Sample texture
    vec3 texColor = texture(texSampler, fragUV).rgb;
    
    // Basic lighting
    float diff = calculateDiffuse(normal, lightDir);
    vec3 ambient = vec3(0.3) * texColor;
    vec3 diffuse = texColor * diff;
    
    // Specular
    vec3 viewDir = normalize(-fragPos);
    float spec = calculateSpecular(normal, lightDir, viewDir, 32.0);
    vec3 specular = vec3(0.3) * spec;
    
    vec3 color = ambient + diffuse + specular;
    
    outColor = vec4(color, 1.0);
}