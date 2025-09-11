# CLAUDE.md

## Workflow

When running an example which requires a human to visually verify the result or interact with the game, don't run with a timeout. If you can can verify the result from the logs, you can use a timeout.  

When running an example after changing a shader, compile the shaders first:  
- **Linux/macOS**: `./compile_shaders.sh`  
- **Windows**: `powershell -ExecutionPolicy Bypass -File compile_shaders.ps1`  
Or compile the individual shaders modified with the appropriate command.

## Coding Style

Avoid unneccessary software design patterns.  
Use functions to avoid code duplication.  
Don't make functions for logic that's only used once.  