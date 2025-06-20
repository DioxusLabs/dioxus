#!/usr/bin/env pwsh

$ErrorActionPreference = 'Stop'

if ($v) {
  $Version = "v${v}"
}
if ($Args.Length -eq 1) {
  $Version = $Args.Get(0)
}

$DxInstall = $env:DX_INSTALL
$BinDir = if ($DxInstall) {
  "${DxInstall}\bin"
} else {
  "${Home}\.dx\bin"
}

$DxZip = "$BinDir\dx.zip"
$DxExe = "$BinDir\dx.exe"
$Target = 'x86_64-pc-windows-msvc'

$DownloadUrl = if (!$Version) {
    "https://github.com/dioxuslabs/dioxus/releases/latest/download/dx-${target}.zip"
} else {
    "https://github.com/dioxuslabs/dioxus/releases/download/${Version}/dx-${target}.zip"
}


if (!(Test-Path $BinDir)) {
  New-Item $BinDir -ItemType Directory | Out-Null
}

curl.exe --ssl-revoke-best-effort -Lo $DxZip $DownloadUrl

tar.exe xf $DxZip -C $BinDir

Remove-Item $DxZip

$CargoBin = "${Home}\.cargo\bin"

if (!(Test-Path $CargoBin)) {
    New-Item $CargoBin -ItemType Directory | Out-Null
}

Copy-Item $DxExe "$CargoBin\dx.exe" -Force

# $User = [System.EnvironmentVariableTarget]::User
# $Path = [System.Environment]::GetEnvironmentVariable('Path', $User)
# if (!(";${Path};".ToLower() -like "*;${BinDir};*".ToLower())) {
#   [System.Environment]::SetEnvironmentVariable('Path', "${Path};${BinDir}", $User)
#   $Env:Path += ";${BinDir}"
# }

Write-Output "dx was installed successfully! 💫"
Write-Output "Run 'dx --help' to get started"
