# LocalWebs Windows Installer
$ErrorActionPreference = "Stop"

$repo = "nvquanghuy/localwebs"
$installDir = "$env:USERPROFILE\.local\bin"

# Detect architecture
$arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }

$tarball = "localwebs-${arch}-pc-windows-msvc.zip"
$url = "https://github.com/${repo}/releases/latest/download/${tarball}"

Write-Host "📦 Downloading LocalWebs for Windows/${arch}..." -ForegroundColor Cyan

$tmpDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
$tmpFile = Join-Path $tmpDir $tarball

try {
    Invoke-WebRequest -Uri $url -OutFile $tmpFile -UseBasicParsing

    Write-Host "📂 Installing to ${installDir}..." -ForegroundColor Cyan
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null

    Expand-Archive -Path $tmpFile -DestinationPath $installDir -Force

    Write-Host ""
    Write-Host "✅ LocalWebs installed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "📍 Location: ${installDir}\localwebs.exe" -ForegroundColor White
    Write-Host "🚀 Run: localwebs" -ForegroundColor White
    Write-Host "🌐 Open: http://localhost:4444" -ForegroundColor White
    Write-Host ""

    # Check if in PATH
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$installDir*") {
        Write-Host "⚠️  Add to your PATH:" -ForegroundColor Yellow
        Write-Host "  `$env:PATH += `";${installDir}`"" -ForegroundColor Gray
        Write-Host ""
    }

    Write-Host "Need help? https://github.com/${repo}#readme" -ForegroundColor Cyan
}
finally {
    Remove-Item -Recurse -Force $tmpDir
}
