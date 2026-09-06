# Publie une nouvelle version de Driver imprimante.
# Contrairement à krino, cette version ne compile RIEN localement : elle met
# à jour le numéro de version, pousse un tag git, et c'est le workflow
# GitHub Actions (.github/workflows/release.yml) qui build et signe sur les
# serveurs GitHub. L'ordinateur local n'est jamais bloqué par la compilation.
#
# Usage : .\publier.ps1 -Version 0.2.0 [-Notes "Description de la version"]
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Notes = ""
)

$ErrorActionPreference = "Stop"

$confPath = "src-tauri\tauri.conf.json"
$conf = Get-Content $confPath -Raw | ConvertFrom-Json
$conf.version = $Version
$conf | ConvertTo-Json -Depth 10 | Set-Content $confPath -Encoding utf8

$pkgPath = "package.json"
$pkg = Get-Content $pkgPath -Raw | ConvertFrom-Json
$pkg.version = $Version
$pkg | ConvertTo-Json -Depth 10 | Set-Content $pkgPath -Encoding utf8

$cargoPath = "src-tauri\Cargo.toml"
(Get-Content $cargoPath -Raw) -replace '(?m)^version = ".*"', "version = `"$Version`"" |
    Set-Content $cargoPath -Encoding utf8

git add $confPath $pkgPath $cargoPath
git commit -m "Bump version to $Version"
git tag "v$Version" -m $Notes
git push
git push --tags

Write-Host "Tag v$Version poussé." -ForegroundColor Green
Write-Host "Build et publication en cours sur GitHub Actions (pas sur cette machine) :" -ForegroundColor Cyan
Write-Host "  https://github.com/Bastien-Gaffet/driver-imprimante/actions"
