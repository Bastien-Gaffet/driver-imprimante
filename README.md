# Driver imprimante

Imprimer un fichier sur une imprimante réseau en le glissant dans une fenêtre, sans passer par le spooler d'impression de Windows.

L'application trouve l'imprimante toute seule sur le réseau local (mDNS), convertit le document si besoin, puis l'envoie directement à l'imprimante avec le protocole IPP. Elle a été écrite pour une Brother HL-L3240CDW dont le pilote Windows posait problème, mais fonctionne avec toute imprimante IPP.

## Fonctionnement

1. **Découverte** : l'imprimante est cherchée en mDNS. Sa dernière adresse connue est gardée en cache pour un démarrage instantané.
2. **Conversion** : les documents Office (Word, Excel, PowerPoint) sont convertis en PDF par LibreOffice en mode headless.
3. **Rendu** : chaque page du PDF est rasterisée avec Ghostscript, puis encodée dans le format raster attendu par l'imprimante.
4. **Envoi** : le document est transmis en IPP après vérification de l'état de l'imprimante, sans aucun pilote installé.

## Stack

- [Tauri 2](https://tauri.app) : application de bureau légère
- Rust : `ipp` pour l'impression, `mdns-sd` pour la découverte
- TypeScript et Vite pour l'interface (sans framework)
- LibreOffice et Ghostscript, appelés comme programmes externes

## Installation

Télécharger l'installateur depuis la [dernière release](https://github.com/Bastien-Gaffet/driver-imprimante/releases/latest), ou lancer `install.ps1`, qui récupère la version la plus récente avec GitHub CLI.

[Ghostscript](https://ghostscript.com) est nécessaire dans tous les cas. [LibreOffice](https://www.libreoffice.org) l'est en plus pour les documents Office. Formats acceptés : PDF, DOCX, XLSX, PPTX.

## Développement

```bash
npm install
npm run tauri dev
```

Le code Rust est dans [src-tauri/src/](src-tauri/src/) : `discovery.rs` (mDNS), `convert.rs` (LibreOffice), `ghostscript.rs` et `raster.rs` (rendu), `printer.rs` (IPP).
