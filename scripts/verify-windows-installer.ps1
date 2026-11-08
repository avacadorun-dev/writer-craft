param([Parameter(Mandatory = $true)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows') {
    throw 'Installer install/remove verification runs only on the native hosted Windows runner.'
}
$output = (Resolve-Path $OutputDirectory).Path
$evidencePath = Join-Path $output 'distribution-evidence.json'
$evidence = Get-Content -Raw $evidencePath | ConvertFrom-Json -AsHashtable
$package = Join-Path $output $evidence.package.filename
if ((Get-FileHash -Algorithm SHA256 $package).Hash.ToLowerInvariant() -ne $evidence.package.sha256) {
    throw 'Installer checksum changed before verification.'
}
$installedRoot = Join-Path $env:LOCALAPPDATA 'Programs\WriterCraft'
$shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'WriterCraft\WriterCraft.lnk'
$registryPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\WriterCraft'
$userData = Join-Path $env:APPDATA 'WriterCraft'
$sentinel = Join-Path $userData 'distribution-preservation-sentinel.bin'
if ((Test-Path $installedRoot) -or (Test-Path $shortcut) -or (Test-Path $sentinel) -or (Test-Path $registryPath)) {
    throw 'Hosted verification requires a clean installation and never overwrites an existing sentinel.'
}
New-Item -ItemType Directory -Force $userData | Out-Null
[IO.File]::WriteAllBytes($sentinel, [byte[]](87,114,105,116,101,114,67,114,97,102,116,0,255))
$sentinelHash = (Get-FileHash -Algorithm SHA256 $sentinel).Hash
$process = Start-Process -FilePath $package -ArgumentList '/S' -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "NSIS install failed with exit $($process.ExitCode)." }
try {
    $executable = Join-Path $installedRoot 'WriterCraft.exe'
    if (-not (Test-Path $executable)) { throw 'Per-user installed executable is missing.' }
    if ((Get-FileHash -Algorithm SHA256 $executable).Hash.ToLowerInvariant() -ne $evidence.executable_sha256) {
        throw 'Installed executable checksum differs from the built executable.'
    }
    $version = (& $executable --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $version -ne "WriterCraft $($evidence.version)") { throw 'Installed app version mismatch.' }
    $installed = Get-Content -Raw (Join-Path $installedRoot 'package-metadata.json') | ConvertFrom-Json
    if ($installed.commit -ne $evidence.commit -or $installed.version -ne $evidence.version) { throw 'Installed source identity mismatch.' }
    foreach ($notice in @('LICENSE', 'THIRD_PARTY_NOTICES.txt', 'NSIS-COPYING.txt', 'WriterCraft.ico')) {
        if ((Get-Item (Join-Path $installedRoot $notice)).Length -eq 0) { throw "Installed $notice is empty." }
    }
    $registration = Get-ItemProperty $registryPath
    if ($registration.DisplayName -ne "WriterCraft $($evidence.version)" -or $registration.DisplayVersion -ne $evidence.version -or $registration.SourceCommit -ne $evidence.commit -or $registration.ApplicationID -ne 'app.writercraft.desktop' -or -not $registration.UninstallString) {
        throw 'Per-user application/uninstall registration mismatch.'
    }
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut($shortcut)
    if ($link.TargetPath -ne $executable -or $link.WorkingDirectory -ne $installedRoot -or -not $link.IconLocation) { throw 'Installed Start menu shortcut is incorrect.' }
    $explorer = New-Object -ComObject Shell.Application
    $folder = $explorer.Namespace((Split-Path $shortcut))
    $item = $folder.ParseName((Split-Path $shortcut -Leaf))
    if ($item.ExtendedProperty('System.AppUserModel.ID') -ne 'app.writercraft.desktop') { throw 'Installed shortcut lost its application identity.' }
    if ((Get-FileHash -Algorithm SHA256 $sentinel).Hash -ne $sentinelHash) { throw 'Installing changed user workspace data.' }
    # An unknown file in the program directory also survives the explicit owned-file uninstall.
    $foreign = Join-Path $installedRoot 'synthetic-unowned-file.txt'
    [IO.File]::WriteAllText($foreign, 'Synthetic unowned file: preserve on uninstall.')
    $foreignHash = (Get-FileHash -Algorithm SHA256 $foreign).Hash
} finally {
    $uninstaller = Join-Path $installedRoot 'Uninstall.exe'
    $process = Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "NSIS uninstall failed with exit $($process.ExitCode)." }
    # The normal NSIS uninstaller copies itself to TEMP before removing its original.
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    while ((Test-Path (Join-Path $installedRoot 'Uninstall.exe')) -and [DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 200
    }
}
if ((Get-FileHash -Algorithm SHA256 $sentinel).Hash -ne $sentinelHash) { throw 'Uninstalling changed user workspace data.' }
if ((Get-FileHash -Algorithm SHA256 $foreign).Hash -ne $foreignHash) { throw 'Uninstalling changed an unowned program-directory file.' }
foreach ($name in @($evidence.owned_files) + @('Uninstall.exe')) {
    if (Test-Path (Join-Path $installedRoot $name)) { throw "Uninstall left owned file $name installed." }
}
if ((Test-Path $shortcut) -or (Test-Path $registryPath)) { throw 'Uninstall left the Start menu entry or application registration installed.' }
foreach ($check in @('unattended_install', 'unattended_uninstall', 'user_data_preserved', 'unowned_files_preserved', 'start_menu', 'application_identity', 'installed_version', 'installed_binary_checksum')) {
    $evidence.checks[$check] = $true
}
$evidence.sentinel_sha256 = $sentinelHash.ToLowerInvariant()
$evidence | ConvertTo-Json -Depth 20 | Set-Content -Encoding utf8NoBOM $evidencePath
Write-Host 'Verified per-user NSIS install/uninstall, Start menu identity and preserved user-data/unowned-file sentinels.'
