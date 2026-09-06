# Installe ou met à jour Driver imprimante depuis la dernière release GitHub.
# Le dépôt est privé, donc ce script utilise GitHub CLI (gh), déjà connecté
# sur cet ordinateur, pour télécharger l'installateur signé le plus récent.
#
# Usage (depuis un PC où `gh` est installé et connecté au même compte) :
#   .\install.ps1
$ErrorActionPreference = "Stop"
$repo = "Bastien-Gaffet/driver-imprimante"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "GitHub CLI (gh) est requis pour installer depuis ce dépôt privé. " +
          "Installez-le (https://cli.github.com), lancez `gh auth login`, puis relancez ce script. " +
          "Sinon, téléchargez l'installateur à la main depuis https://github.com/$repo/releases/latest " +
          "en étant connecté à GitHub dans votre navigateur."
}

$tempDir = Join-Path $env:TEMP "driver-imprimante-install"
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

Write-Host "Téléchargement de la dernière version depuis $repo..." -ForegroundColor Cyan
gh release download --repo $repo --pattern "*-setup.exe" --dir $tempDir --clobber

$setup = Get-ChildItem "$tempDir\*-setup.exe" | Select-Object -First 1
if (-not $setup) { throw "Aucun installateur trouvé dans la dernière release." }

Write-Host "Lancement de l'installateur ($($setup.Name))..." -ForegroundColor Cyan
Start-Process -FilePath $setup.FullName -Wait

Write-Host "Installation terminée. L'application se mettra à jour automatiquement au démarrage." -ForegroundColor Green
