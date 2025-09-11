#version 450

// Vertex attributes
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;
layout(location = 3) in vec4 inColor;
layout(location = 4) in uvec4 inJointIndices;
layout(location = 5) in vec4 inJointWeights;
// Per-instance position offset  
layout(location = 6) in vec3 inInstancePosition;

// Uniform buffer for joint matrices
layout(set = 0, binding = 0) uniform JointMatrices {
    mat4 joints[128];
} jointMatrices;

// Uniform buffer for camera matrices
layout(set = 0, binding = 1) uniform CameraMatrices {
    mat4 view;
    mat4 proj;
} camera;

// Push constants
layout(push_constant) uniform PushConstants {
    float time;
} push;

// Output to fragment shader
layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec3 fragPos;
layout(location = 2) out vec2 fragUV;
layout(location = 3) out vec4 fragColor;

void main() {
    // Check if vertex has any skinning weights
    float totalWeight = inJointWeights.x + inJointWeights.y + inJointWeights.z + inJointWeights.w;
    
    vec4 skinnedPos;
    
    if (totalWeight > 0.001) {
        // Normalize weights if they don't sum to 1.0
        vec4 normalizedWeights = inJointWeights;
        if (abs(totalWeight - 1.0) > 0.001) {
            normalizedWeights = inJointWeights / totalWeight;
        }
        
        // Apply skinning with all joints animated
        mat4 skinMatrix = mat4(0.0);
        
        // Process each joint influence with normalized weights
        if (normalizedWeights.x > 0.0) {
            skinMatrix += jointMatrices.joints[inJointIndices.x] * normalizedWeights.x;
        }
        
        if (normalizedWeights.y > 0.0) {
            skinMatrix += jointMatrices.joints[inJointIndices.y] * normalizedWeights.y;
        }
        
        if (normalizedWeights.z > 0.0) {
            skinMatrix += jointMatrices.joints[inJointIndices.z] * normalizedWeights.z;
        }
        
        if (normalizedWeights.w > 0.0) {
            skinMatrix += jointMatrices.joints[inJointIndices.w] * normalizedWeights.w;
        }
        
        // Apply the skin matrix
        skinnedPos = skinMatrix * vec4(inPosition, 1.0);
    } else {
        // No skinning - use original position
        skinnedPos = vec4(inPosition, 1.0);
    }
    
    // Use original model scale
    vec3 scaledPos = skinnedPos.xyz;
    
    // Apply instance offset to position
    vec3 instancedPos = scaledPos + inInstancePosition;
    
    fragPos = instancedPos;
    fragNormal = normalize(inNormal);
    fragUV = inUV;
    
    // Use the vertex color from the model
    fragColor = inColor;
    
    // Apply view and projection
    gl_Position = camera.proj * camera.view * vec4(instancedPos, 1.0);
}