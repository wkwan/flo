#version 450

// Input from vertex shader
layout(location = 0) in vec4 fragColor;

// Output
layout(location = 0) out vec4 outColor;

void main() {
    // Simple passthrough of color
    outColor = fragColor;
    
    // Ensure alpha is 1.0 for solid lines
    outColor.a = 1.0;
}