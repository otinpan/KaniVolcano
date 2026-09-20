$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$ShaderDir = Join-Path $Root "shaders/src"
$OutDir = Join-Path $Root "shaders/compiled"

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

glslc `
    (Join-Path $ShaderDir "mesh3d.vert") `
    -o (Join-Path $OutDir "mesh3d_vert.spv")

glslc `
    (Join-Path $ShaderDir "mesh3d.frag") `
    -o (Join-Path $OutDir "mesh3d_frag.spv")

glslc `
    (Join-Path $ShaderDir "debug_line.vert") `
    -o (Join-Path $OutDir "debug_line_vert.spv")

glslc `
    (Join-Path $ShaderDir "debug_line.frag") `
    -o (Join-Path $OutDir "debug_line_frag.spv")

glslc `
    (Join-Path $ShaderDir "transparent3d.vert") `
    -o (Join-Path $OutDir "transparent3d_vert.spv")

glslc `
    (Join-Path $ShaderDir "transparent3d.frag") `
    -o (Join-Path $OutDir "transparent3d_frag.spv")

glslc `
    (Join-Path $ShaderDir "lit3d.vert") `
    -o (Join-Path $OutDir "lit3d_vert.spv")

glslc `
    (Join-Path $ShaderDir "lit3d.frag") `
    -o (Join-Path $OutDir "lit3d_frag.spv")

glslc `
    (Join-Path $ShaderDir "ui2d.vert") `
    -o (Join-Path $OutDir "ui2d_vert.spv")

glslc `
    (Join-Path $ShaderDir "ui2d.frag") `
    -o (Join-Path $OutDir "ui2d_frag.spv")

glslc `
    (Join-Path $ShaderDir "skybox.vert") `
    -o (Join-Path $OutDir "skybox_vert.spv")

glslc `
    (Join-Path $ShaderDir "skybox.frag") `
    -o (Join-Path $OutDir "skybox_frag.spv")

glslc `
    (Join-Path $ShaderDir "text_ui2d.vert") `
    -o (Join-Path $OutDir "text_ui2d_vert.spv")

glslc `
    (Join-Path $ShaderDir "text_ui2d.frag") `
    -o (Join-Path $OutDir "text_ui2d_frag.spv")
Write-Host "Shaders compiled to $OutDir"
