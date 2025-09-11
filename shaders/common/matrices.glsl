// Common matrix functions for shaders

#ifndef MATRICES_GLSL
#define MATRICES_GLSL

mat4 getProjectionMatrix() {
    // Adjust for typical 16:9 aspect ratio (1920x1080)
    float aspectRatio = 16.0 / 9.0;
    return mat4(
        1.0 / aspectRatio, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, -1.0, -1.0,
        0.0, 0.0, -0.2, 0.0
    );
}

mat4 getProjectionMatrixVulkan() {
    // Y-axis flipped for Vulkan's coordinate system
    // Adjust for typical 16:9 aspect ratio (1920x1080)
    float aspectRatio = 16.0 / 9.0;
    return mat4(
        1.0 / aspectRatio, 0.0, 0.0, 0.0,
        0.0, -1.0, 0.0, 0.0,
        0.0, 0.0, -1.0, -1.0,
        0.0, 0.0, -0.2, 0.0
    );
}

mat4 getViewMatrix(vec3 translation) {
    return mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        translation.x, translation.y, translation.z, 1.0
    );
}

mat4 getRotationMatrixY(float angle) {
    float c = cos(angle);
    float s = sin(angle);
    return mat4(
        c, 0.0, s, 0.0,
        0.0, 1.0, 0.0, 0.0,
        -s, 0.0, c, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
}

mat4 getRotationMatrixX(float angle) {
    float c = cos(angle);
    float s = sin(angle);
    return mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, c, -s, 0.0,
        0.0, s, c, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
}

#endif // MATRICES_GLSL