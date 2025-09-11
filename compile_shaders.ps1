# Try to find a shader compiler
$compiler = $null
$compilerArgs = $null

if (Get-Command glslc -ErrorAction SilentlyContinue) {
    Write-Host "Using glslc to compile shaders..."
    $compiler = "glslc"
    $compilerArgs = @()
} elseif (Get-Command glslangValidator -ErrorAction SilentlyContinue) {
    Write-Host "Using glslangValidator to compile shaders..."
    $compiler = "glslangValidator"
    $compilerArgs = @("-V")
} else {
    Write-Host "No shader compiler found. Please install glslc or glslangValidator." -ForegroundColor Red
    exit 1
}

# Compile all vertex and fragment shaders
$shaderDir = "shaders"
$success = $true

# Find all .vert and .frag files and compile them
$shaderFiles = Get-ChildItem -Path $shaderDir -Filter "*.vert"
$shaderFiles += Get-ChildItem -Path $shaderDir -Filter "*.frag"

foreach ($shader in $shaderFiles) {
    $output = "$($shader.FullName).spv"
    Write-Host "Compiling $($shader.Name) -> $([System.IO.Path]::GetFileName($output))"
    
    $args = $compilerArgs + @($shader.FullName, "-o", $output)
    
    $process = Start-Process -FilePath $compiler -ArgumentList $args -Wait -PassThru -NoNewWindow
    
    if ($process.ExitCode -ne 0) {
        Write-Host "Failed to compile $($shader.Name)" -ForegroundColor Red
        $success = $false
    }
}

if ($success) {
    Write-Host "All shaders compiled successfully!" -ForegroundColor Green
} else {
    Write-Host "Some shaders failed to compile" -ForegroundColor Red
    exit 1
}