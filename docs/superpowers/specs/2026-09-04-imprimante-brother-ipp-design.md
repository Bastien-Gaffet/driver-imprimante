# Application d'impression directe — Brother HL-L3240CDW (ou modèle proche)

## Contexte et problème

L'imprimante réseau du foyer souffre d'instabilité côté Windows : le spooler
reste bloqué en « recherche de l'imprimante » et les jobs échouent souvent.
Un outil maison existant (`toolbox-py/scan_reseau.py`, ping sweep + scan de
ports) a tenté de localiser l'imprimante sur le réseau sans succès fiable :
sur 4 appareils détectés, 2 restent sans nom identifiable (le port 9100 seul
ne suffit pas à identifier une imprimante Brother avec certitude, et le
port IPP 631 n'était pas scanné).

L'IP de l'imprimante est censée être fixe (configurée manuellement), mais ce
réglage n'a pas pu être vérifié en direct au moment du design.

## Objectif

Construire une application de bureau qui imprime directement sur
l'imprimante réseau **sans passer par le spooler d'impression Windows ni un
pilote installé**, avec une interface glisser-déposer simple pour un usage
quotidien depuis plusieurs PC du foyer (téléphone/tablette envisagés plus
tard, hors périmètre initial).

## Stack retenue

Architecture calquée sur le projet existant `krino` (Tauri v2) :

- **Frontend** : TypeScript + Vite, sans framework lourd (cohérent avec
  krino). Fenêtre de glisser-déposer, affichage du statut d'impression.
- **Backend** : Rust (`src-tauri/`), compilé, natif, rapide.

Bibliothèques Rust clés :

- [`ipp`](https://crates.io/crates/ipp) — client IPP conforme RFC 8010/8011
  (Print-Job, Get-Printer-Attributes), client synchrone via `ureq`.
- [`mdns-sd`](https://crates.io/crates/mdns-sd) — découverte mDNS pure Rust,
  sans dépendance à un runtime async, pour repérer l'imprimante par son
  annonce de service plutôt que par un ping sweep brut.
- LibreOffice (binaire externe, invoqué en sous-processus) pour la
  conversion Word/Excel → PDF. À installer par l'utilisateur (gratuit).

Pourquoi ce choix plutôt que Python : interface plus soignée et plus rapide
à l'usage quotidien, binaire natif optimisé, et les deux bibliothèques Rust
ci-dessus couvrent exactement les besoins (IPP + mDNS) sans qu'il soit
nécessaire de réimplémenter le protocole à la main.

## Architecture

```
src/                      # Frontend TypeScript + Vite
  main.ts                 # UI glisser-déposer, appels invoke() vers le backend
  style.css

src-tauri/
  src/
    discovery.rs           # Découverte mDNS + cache de l'IP imprimante
    printer.rs              # Client IPP : Print-Job, Get-Printer-Attributes
    convert.rs              # Conversion Word/Excel -> PDF via LibreOffice
    lib.rs                  # Déclaration des commandes Tauri
  tauri.conf.json
```

Commandes Tauri exposées au frontend :

- `discover_printer()` — relance une découverte mDNS, retourne IP + nom.
- `print_file(path: String)` — point d'entrée principal : détecte le type de
  fichier, convertit si nécessaire, envoie à l'imprimante.
- `get_status()` — dernier état connu de l'imprimante et de la config.

## Flux de données

1. Démarrage de l'app → lecture de l'IP imprimante en cache (fichier de
   config dans le dossier de données de l'app) → test rapide de connexion
   TCP sur le port 631 → si injoignable, lancement d'une découverte
   `mdns-sd` (recherche du service `_ipp._tcp.local.`) → mise à jour du
   cache avec l'IP et le nom trouvés.
2. L'utilisateur glisse un fichier sur la fenêtre.
3. Selon l'extension :
   - `.pdf` → transmis tel quel.
   - `.docx` / `.xlsx` / `.pptx` → converti en PDF temporaire via
     `soffice --headless --convert-to pdf`, puis transmis.
   - autre extension → rejeté avec message clair, rien n'est envoyé.
4. Envoi du PDF à l'imprimante via IPP `Print-Job`, précédé d'un
   `Get-Printer-Attributes` pour vérifier l'état (prête, papier, toner) et
   donner un message utile plutôt qu'un échec silencieux.
5. Résultat affiché dans la fenêtre : succès, ou raison de l'échec avec un
   bouton pour réessayer.

## Gestion des erreurs

- **Imprimante introuvable** après la découverte mDNS : message clair +
  champ de secours pour saisir l'IP manuellement.
- **LibreOffice absent** : message avec instructions d'installation
  (chemin d'installation par défaut vérifié automatiquement au démarrage).
- **Erreur IPP** (bourrage, plus de papier, toner bas, etc.) : le message
  renvoyé par l'imprimante (`printer-state-reasons`) est relayé à
  l'utilisateur plutôt qu'un code d'erreur brut.
- **Type de fichier non supporté** : rejeté avant toute tentative de
  conversion, avec la liste des formats acceptés.

## Tests

- Tests unitaires Rust sur la logique pure : détection du type de fichier
  par extension, lecture/écriture du cache de config JSON, parsing des
  réponses IPP (`printer-state`, `printer-state-reasons`).
- Test d'intégration manuel réel, pendant le développement : impression
  effective d'un PDF et d'un `.docx` converti sur l'imprimante physique
  (non simulable de façon fiable en CI pour du matériel réel).
- Test manuel de l'UI via `npm run tauri dev` avant de considérer une
  fonctionnalité terminée.

## Périmètre exclu (pour l'instant)

- Impression depuis téléphone/tablette (mentionné comme besoin futur, pas
  la cible principale).
- Impression de pages web ou de texte brut (non demandé dans les besoins
  actuels : PDF et Office uniquement).
- Gestion multi-imprimantes (un seul appareil ciblé pour l'instant).
