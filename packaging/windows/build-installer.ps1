# Build Windows Installer for Chronos Engine Editor
# Requires: Rust (MSVC toolchain), WiX Toolset v4 (heat.exe, candle.exe, light.exe)
# Run on Windows only.
#
# Install WiX: https://wixtoolset.org/docs/intro/
#
# This script:
#   1. Builds chronos-editor.exe and chronos.exe (CLI) in release mode
#   2. Generates WiX source fragments via heat.exe
#   3. Compiles and links a per-machine MSI installer
#   4. Output: target/windows-installer/Chronos-Engine-Editor-<version>-x64.msi

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Join-Path $ScriptDir "..\.." | Resolve-Path
$BuildDir = Join-Path $ProjectRoot "target\windows-installer"
$AppDir = Join-Path $BuildDir "AppDir"

$AppName = "Chronos Engine Editor"
$AppVersion = "1.0.0"
$AppNameShort = "ChronosEngine"
$CliName = "Chronos Engine CLI"
$Manufacturer = "synth"
$ProductCode = [System.Guid]::NewGuid().ToString("D").ToUpper()
$UpgradeCode = "A1B2C3D4-E5F6-7890-ABCD-EF1234567890"

Write-Host "=== Building Windows Installer for Chronos Engine ===" -ForegroundColor Green

# Build release binaries (MSVC)
Write-Host "Building release binaries (MSVC toolchain)..." -ForegroundColor Yellow
cargo build --bin chronos-editor --features editor --release
cargo build --bin chronos --features full --release

# Prepare staging directory
Write-Host "Preparing staging directory..." -ForegroundColor Yellow
if (Test-Path $AppDir) { Remove-Item -Recurse -Force $AppDir }
New-Item -ItemType Directory -Path $AppDir -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $AppDir "bin") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $AppDir "share\icons\hicolor\256x256\apps") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $BuildDir "wxs") -Force | Out-Null

# Copy editor binary
Copy-Item (Join-Path $ProjectRoot "target\release\chronos-editor.exe") (Join-Path $AppDir "bin\")

# Copy CLI binary
Copy-Item (Join-Path $ProjectRoot "target\release\chronos.exe") (Join-Path $AppDir "bin\")

# Copy icon
Copy-Item (Join-Path $ProjectRoot "icon.png") (Join-Path $AppDir "share\icons\hicolor\256x256\apps\chronos-editor.png")

# Copy license & readme
Copy-Item (Join-Path $ProjectRoot "LICENSE") (Join-Path $AppDir "LICENSE")
if (Test-Path (Join-Path $ProjectRoot "README.md")) {
    Copy-Item (Join-Path $ProjectRoot "README.md") (Join-Path $AppDir "README.md")
}

# Generate WiX fragment via heat
Write-Host "Generating WiX fragment..." -ForegroundColor Yellow
heat dir $AppDir `
    -gg `
    -sreg `
    -sfrag `
    -srd `
    -dr INSTALLDIR `
    -cg AppComponents `
    -out (Join-Path $BuildDir "wxs\components.wxs")

# Create Product.wxs
$ProductWxs = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    <Product Id="$ProductCode"
             Name="$AppName"
             Language="1033"
             Version="$AppVersion"
             Manufacturer="$Manufacturer"
             UpgradeCode="$UpgradeCode">
        <Package InstallerVersion="200"
                 Compressed="yes"
                 InstallScope="perMachine" />

        <MajorUpgrade DowngradeErrorMessage="A newer version of [ProductName] is already installed."
                       Schedule="afterInstallValidate" />

        <Media Id="1" Cabinet="chronos.cab" EmbedCab="yes" />

        <Feature Id="ProductFeature"
                 Title="$AppName"
                 Level="1"
                 ConfigurableDirectory="INSTALLDIR">
            <ComponentGroupRef Id="AppComponents" />
            <ComponentRef Id="DesktopShortcut" />
            <ComponentRef Id="StartMenuShortcut" />
            <ComponentRef Id="CliToPath" />
        </Feature>

        <Directory Id="TARGETDIR" Name="SourceDir">
            <Directory Id="ProgramFiles64Folder">
                <Directory Id="INSTALLDIR" Name="Chronos Engine">
                    <Directory Id="BINDIR" Name="bin" />
                </Directory>
            </Directory>
            <Directory Id="ProgramMenuFolder">
                <Directory Id="StartMenuDir" Name="Chronos Engine Editor" />
            </Directory>
            <Directory Id="DesktopFolder" />
        </Directory>

        <!-- Desktop shortcut -->
        <DirectoryRef Id="DesktopFolder">
            <Component Id="DesktopShortcut" Guid="*">
                <Shortcut Id="DesktopShortcut"
                          Name="$AppName"
                          Description="$AppName"
                          Target="[INSTALLDIR]bin\chronos-editor.exe"
                          WorkingDirectory="INSTALLDIR"
                          Icon="chronos-editor.exe" />
                <RemoveFolder Id="RemoveDesktopShortcut" On="uninstall" />
                <RegistryValue Root="HKCU"
                               Key="Software\[Manufacturer]\[$AppNameShort]"
                               Name="desktop_shortcut"
                               Type="integer"
                               Value="1"
                               KeyPath="yes" />
            </Component>
        </DirectoryRef>

        <!-- Start Menu shortcut -->
        <DirectoryRef Id="StartMenuDir">
            <Component Id="StartMenuShortcut" Guid="*">
                <Shortcut Id="StartMenuShortcut"
                          Name="$AppName"
                          Description="$AppName"
                          Target="[INSTALLDIR]bin\chronos-editor.exe"
                          WorkingDirectory="INSTALLDIR"
                          Icon="chronos-editor.exe" />
                <RemoveFolder Id="RemoveStartMenuDir" On="uninstall" />
                <RegistryValue Root="HKCU"
                               Key="Software\[Manufacturer]\[$AppNameShort]"
                               Name="start_menu_shortcut"
                               Type="integer"
                               Value="1"
                               KeyPath="yes" />
            </Component>
        </DirectoryRef>

        <!-- Add CLI to PATH (opt-in via feature) -->
        <DirectoryRef Id="INSTALLDIR">
            <Component Id="CliToPath" Guid="*">
                <Environment Id="PathUpdate"
                             Name="PATH"
                             Value="[INSTALLDIR]bin"
                             Permanent="no"
                             Part="last"
                             Action="set"
                             System="yes" />
                <RegistryValue Root="HKLM"
                               Key="Software\[Manufacturer]\[$AppNameShort]"
                               Name="cli_path"
                               Type="integer"
                               Value="1"
                               KeyPath="yes" />
            </Component>
        </DirectoryRef>

        <!-- Icon -->
        <Icon Id="chronos-editor.exe"
              SourceFile="[INSTALLDIR]bin\chronos-editor.exe" />

        <!-- UI -->
        <WixVariable Id="WixUILicenseRtf" Value="$(env.ProjectRoot)\LICENSE" />
        <UIRef Id="WixUI_Minimal" />
    </Product>
</Wix>
"@

$ProductWxs | Out-File -FilePath (Join-Path $BuildDir "wxs\Product.wxs") -Encoding UTF8

# Compile and link
Write-Host "Compiling WiX sources..." -ForegroundColor Yellow
candle.exe (Join-Path $BuildDir "wxs\Product.wxs") `
    -out (Join-Path $BuildDir "Product.wixobj") `
    -arch x64

candle.exe (Join-Path $BuildDir "wxs\components.wxs") `
    -out (Join-Path $BuildDir "components.wixobj") `
    -arch x64

Write-Host "Linking MSI..." -ForegroundColor Yellow
light.exe (Join-Path $BuildDir "Product.wixobj"), `
         (Join-Path $BuildDir "components.wixobj") `
    -out (Join-Path $BuildDir "$AppName-$AppVersion-x64.msi") `
    -ext WixUIExtension `
    -cultures:en-us

Write-Host "" -ForegroundColor Green
Write-Host "=== Build Complete ===" -ForegroundColor Green
Write-Host "  MSI: $BuildDir\$AppName-$AppVersion-x64.msi" -ForegroundColor Green
Write-Host "" -ForegroundColor Green
Write-Host "Note: Run this script on Windows with WiX Toolset v4 installed." -ForegroundColor Yellow
