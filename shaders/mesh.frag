#version 450

#include "common/lighting.glsl"

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec3 fragPos;
layout(location = 2) in vec2 fragUV;
layout(location = 3) in vec4 fragColor;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 lightDir = normalize(vec3(0.5, 1.0, 0.8));
    vec3 normal = normalize(fragNormal);
    
    float diff = calculateDiffuse(normal, lightDir);
    vec3 ambient = vec3(0.3, 0.2, 0.3);
    vec3 diffuse = vec3(0.8, 0.3, 0.4) * diff;
    
    vec3 viewDir = normalize(-fragPos);
    float spec = calculateSpecular(normal, lightDir, viewDir, 32.0);
    vec3 specular = vec3(0.5) * spec;
    
    vec3 color = ambient + diffuse + specular;
    color *= fragColor.rgb;
    
    outColor = vec4(color, fragColor.a);
}