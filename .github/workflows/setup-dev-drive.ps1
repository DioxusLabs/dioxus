# Configures a dev drive for Windows CI to speed up Rust builds.
#
# The main performance win is disabling Windows Defender antivirus scanning
# on the build drive — AV scanning every .o, .rlib, .rmeta, and .exe during
# a Rust build is extremely expensive.
#
# When a D: drive already exists (common on standard GitHub/Warp runners),
# we use it directly. Otherwise we create a Dev Drive VHD on C:.

if (Test-Path "D:\") {
    Write-Output "Using existing drive at D:"
    $Drive = "D:"
} else {
    # 25 GB is enough for a full workspace build with dependencies
    $Volume = New-VHD -Path C:/dev_drive.vhdx -SizeBytes 25GB |
                      Mount-VHD -Passthru |
                      Initialize-Disk -Passthru |
                      New-Partition -AssignDriveLetter -UseMaximumSize |
                      Format-Volume -DevDrive -Confirm:$false -Force

    $Drive = "$($Volume.DriveLetter):"

    # Mark as trusted and disable antivirus filtering for dev drives
    fsutil devdrv trust $Drive
    fsutil devdrv enable /disallowAv

    # Remount so the changes take effect
    Dismount-VHD -Path C:/dev_drive.vhdx
    Mount-VHD -Path C:/dev_drive.vhdx

    Write-Output $Volume
    fsutil devdrv query $Drive
    Write-Output "Created Dev Drive at $Drive"
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
