#version 450

#include "common/lighting.glsl"

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec3 fragPos;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 lightDir = normalize(vec3(0.5, -1.0, -0.3));
    
    float diff = calculateDiffuse(normalize(fragNormal), -lightDir);
    
    vec3 ambient = vec3(0.5, 0.5, 0.5) * 0.3;
    vec3 diffuse = vec3(0.5, 0.5, 0.5) * diff;
    
    vec3 result = ambient + diffuse;
    
    outColor = vec4(result, 1.0);
}