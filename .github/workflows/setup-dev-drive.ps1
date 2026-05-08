# Configures a dev drive for Windows CI to speed up Rust builds.
#
# The main performance win is disabling Windows Defender antivirus scanning
# on the build drive — AV scanning every .o, .rlib, .rmeta, and .exe during
# a Rust build is extremely expensive.
#
# Three code paths:
#   1. D: drive exists (some runners) — use it directly
#   2. Hyper-V available — create a Dev Drive VHD (best perf, supports fsutil devdrv)
#   3. No Hyper-V (Warp/EC2 runners) — use diskpart + ReFS format

if (Test-Path "D:\") {
    Write-Output "Using existing drive at D:"
    $Drive = "D:"
} elseif (Get-Command New-VHD -ErrorAction SilentlyContinue) {
    # Hyper-V is available — create a proper Dev Drive
    $Volume = New-VHD -Path C:/dev_drive.vhdx -SizeBytes 25GB |
                      Mount-VHD -Passthru |
                      Initialize-Disk -Passthru |
                      New-Partition -AssignDriveLetter -UseMaximumSize |
                      Format-Volume -DevDrive -Confirm:$false -Force

    $Drive = "$($Volume.DriveLetter):"

    # Mark as trusted and disable antivirus filtering
    fsutil devdrv trust $Drive
    fsutil devdrv enable /disallowAv

    # Remount so the changes take effect
    Dismount-VHD -Path C:/dev_drive.vhdx
    Mount-VHD -Path C:/dev_drive.vhdx

    Write-Output $Volume
    fsutil devdrv query $Drive
    Write-Output "Created Dev Drive at $Drive"
} else {
    # No Hyper-V — fall back to diskpart + ReFS
    Write-Output "No Hyper-V detected, creating ReFS drive via diskpart..."

    $vhdPath = "C:\dev_drive.vhdx"
    @"
create vdisk file="$vhdPath" maximum=25600 type=expandable
attach vdisk
create partition primary
active
assign letter=V
"@ | diskpart

    format V: /fs:ReFS /q /y
    $Drive = "V:"

    Write-Output "Created ReFS drive at $Drive"
}

$Tmp = "$($Drive)\tmp"

# Create directories ahead of time to avoid race conditions
New-Item $Tmp -ItemType Directory -Force

# Move Cargo to the dev drive so registry/crate downloads also benefit
New-Item -Path "$($Drive)/.cargo/bin" -ItemType Directory -Force
if (Test-Path "C:/Users/runneradmin/.cargo") {
    Copy-Item -Path "C:/Users/runneradmin/.cargo/*" -Destination "$($Drive)/.cargo/" -Recurse -Force
}

Write-Output `
    "DEV_DRIVE=$($Drive)" `
    "TMP=$($Tmp)" `
    "TEMP=$($Tmp)" `
    "RUSTUP_HOME=$($Drive)/.rustup" `
    "CARGO_HOME=$($Drive)/.cargo" `
    "DIOXUS_WORKSPACE=$($Drive)/dioxus" `
    "PATH=$($Drive)/.cargo/bin;$env:PATH" `
    >> $env:GITHUB_ENV
