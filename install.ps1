$packwiz = "https://nightly.link/packwiz/packwiz/workflows/go/main/Windows%2064-bit.zip"
$packwiz_archive="./packwiz.zip"
if (-not (Test-Path -Path $packwiz_archive))
{
    Invoke-WebRequest -Uri $packwiz -OutFile $packwiz_archive
}
Expand-Archive $packwiz_archive

$modpack="https://github.com/GregTechLite/GregTech-Lite-Modpack/archive/refs/heads/main.zip"
$modpack_archive="./modpack.zip"
if (-not (Test-Path -Path $modpack_archive))
{
    Invoke-WebRequest -Uri $modpack -OutFile $modpack_archive
}
Expand-Archive $modpack_archive
Set-Location "./modpack/GregTech-Lite-Modpack-Main"
. '../../packwiz/packwiz.exe' curseforge export -y -o "../../GregTech-Lite-Modpack.cf.zip"