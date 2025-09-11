// Common lighting functions for shaders

#ifndef LIGHTING_GLSL
#define LIGHTING_GLSL

#include "constants.glsl"

float calculateDiffuse(vec3 normal, vec3 lightDir) {
    return max(dot(normal, lightDir), 0.0);
}

float calculateSpecular(vec3 normal, vec3 lightDir, vec3 viewDir, float shininess) {
    vec3 reflectDir = reflect(-lightDir, normal);
    return pow(max(dot(viewDir, reflectDir), 0.0), shininess);
}

float calculateBlinnPhongSpecular(vec3 normal, vec3 lightDir, vec3 viewDir, float shininess) {
    vec3 halfwayDir = normalize(lightDir + viewDir);
    return pow(max(dot(normal, halfwayDir), 0.0), shininess);
}

// Enhanced diffuse with wrap-around lighting
float diffuseWrap(vec3 n, vec3 l, float p) {
    return pow(dot(n, l) * 0.4 + 0.6, p);
}

// Enhanced specular with energy conservation
float specularEnergy(vec3 n, vec3 l, vec3 e, float s) {
    float nrm = (s + 8.0) / (PI * 8.0);
    return pow(max(dot(reflect(e, n), l), 0.0), s) * nrm;
}

// Simple Fresnel effect
float fresnel(vec3 normal, vec3 viewDir, float power) {
    return pow(1.0 - max(dot(normal, viewDir), 0.0), power);
}

#endif // LIGHTING_GLSL