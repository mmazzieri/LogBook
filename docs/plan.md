# DiaryCraft — piano di implementazione v1 (bozza)

Stato: v1 del 24/11/2026, scritta da Claude; passi 1-5 realizzati nella prima PR; evidenziazione del Markdown, anteprima e conversione delle voci .txt aggiunte con l'issue #4. Decisioni di Mauro già prese (24/11/2026): nome DiaryCraft; l'app nasce dall'idea logbook (OneDrive\Progetti\03 Prototype\logbook), la scrive Claude direttamente come BalanceCraft e non con Craft Anything, e il metodo di Kimbro (mappa-pensieri) ne diventa una funzionalità.

## Punto di partenza

- `mmazzieri/LogBook` contiene solo lo scheletro di ABP Framework generato nel 2021 (identità, permessi, Blazor, test d'esempio). Non c'è nessuna entità del diario e il prototipo Windows Forms del 2014 non è nel repository. Non c'è codice da portare.
- La specifica è la nota del 16/12/2014: cartella radice configurabile; un file per giorno in sottocartelle per anno, mese e giorno; finestra con cartella e ricerca in alto, calendario ed editor sotto; salvataggio automatico al cambio di data e alla chiusura; allegati facoltativi (gli altri file nella cartella del giorno); le versioni non desktop salvano su OneDrive o simili.
- `mmazzieri/complete-thought-map` contiene il libro di Kimbro (CC BY-SA 1.0) e il README di un programma «ibrido» mai scritto. Il software lì è dichiarato GPL-3.0, ma non c'è codice: la parte Kimbro di DiaryCraft si scrive da zero sotto MIT OR Apache-2.0.

## Obiettivo

App desktop locale in Rust ed egui per tenere un diario: una voce per giorno, scritta in testo semplice, salvata come file normale in una cartella che l'utente sceglie e che può stare in locale o in una cartella sincronizzata (OneDrive, Google Drive, Dropbox, Syncthing) o su un disco condiviso. I file restano leggibili e modificabili con qualunque editor anche senza l'app. Più avanti: candidatura come community app nell'ArtCraft Launcher.

Fuori dalla v1: account, server, cifratura, app mobile, build web, AI. (CLI e server MCP: aggiunti dopo la 0.2, `crates/commands` e `apps/diarycraft-cli`.)

## Vincoli

- Le stesse regole di BalanceCraft e delle app Craft: `unsafe` vietato, clippy `-D warnings`, `unwrap`/`expect`/`panic` negati fuori dai test, dipendenze solo dall'allowlist delle licenze controllata con `cargo deny`, nessun codice GPL/LGPL/AGPL nemmeno letto.
- Clean room: nessun testo, icona o asset di altri programmi di diario; nessun nome di prodotto nell'interfaccia.
- **Il testo dell'utente non si perde mai.** Ogni scrittura è atomica (file temporaneo e rinomina); se il file sul disco è cambiato da quando l'app l'ha letto, l'app non lo sovrascrive: tiene entrambe le versioni e lo dice. È la regola che conta più delle funzioni.

## Architettura (workspace Cargo, come BalanceCraft)

```
crates/
  model/     tipi: Entry (data, testo, metadati facoltativi), DiaryConfig, Subject, SpeedThought…
  engine/    operazioni pure: apertura del giorno, ricerca, calendario (giorni con voce),
             parsing e scrittura del front matter, parte Kimbro; orologio passato come parametro
  file/      layout della cartella, lettura e scrittura atomica, controllo delle modifiche esterne,
             copie di conflitto dei client di sincronizzazione, allegati
  ui-egui/   interfaccia
apps/
  diarycraft/   eseguibile desktop (eframe)
xtask/          ci, packaging
```

`model` ed `engine` non fanno I/O e compilano in wasm32, per una futura build web.

## Formato su disco

- Cartella radice scelta dall'utente. Al primo avvio DiaryCraft propone `DiaryCraft` dentro la cartella Documenti (su Windows quella indicata dal registro, anche se OneDrive l'ha spostata; su Linux quella delle directory utente XDG; su macOS `~/Documents`), con «Usa questa cartella» e «Scegli un'altra cartella…» (decisione di Mauro, 24/11/2026). Una voce: `AAAA/MM/AAAA-MM-GG.md`, Markdown in UTF-8. Il nome con la data intera rende il file riconoscibile anche fuori dalla sua cartella.
- Il file è solo testo. Metadati facoltativi (luogo, umore, etichette) in un front matter YAML minimo in testa al file, che l'app conserva anche quando contiene chiavi che non conosce.
- Allegati: cartella `AAAA/MM/AAAA-MM-GG/` accanto alla voce; l'app li elenca e li apre con il programma di sistema, e li inserisce nel testo come link relativi Markdown.
- Impostazioni del diario in `.diarycraft/config.json` nella radice (versione del formato, lingua del calendario, primo giorno della settimana), così sono uguali su ogni computer che apre la cartella. Le impostazioni della finestra restano sul computer.
- Import: DiaryCraft legge anche i layout della nota del 2014 (`AAAA/MM/GG.txt` e `AAAA/MM/AAAAMMGG.txt`) e li mostra. Decisione di Mauro (24/11/2026, issue #4): le voci devono essere Markdown, quindi una voce `.txt` diventa `.md` in silenzio la prima volta che Mauro la modifica. Il file viene rinominato in `AAAA/MM/AAAA-MM-GG.md` (non cancellato e riscritto) e poi salvato in UTF-8, solo se è ancora la versione letta dall'editor e se non esiste già un `.md` di quel giorno; altrimenti vale la regola dei conflitti. Le voci non modificate restano come sono. Un `.txt` nascosto da un `.md` dello stesso giorno, per esempio riportato dal client di sincronizzazione, compare tra le copie di conflitto.

## Interfaccia v1

- In alto: la cartella aperta, la ricerca nel testo di tutte le voci, il pulsante Oggi.
- A sinistra il calendario del mese, con i giorni che hanno una voce evidenziati e le frecce per mese e anno; sotto, l'elenco delle voci recenti e i risultati della ricerca.
- A destra l'editor del giorno: testo Markdown con evidenziazione leggera (titoli, grassetto, corsivo, codice, elenchi, citazioni, link, front matter; `crates/engine/src/markdown.rs`, una riga alla volta, senza mai fallire), contatore delle parole, allegati. «Scrivi» e «Anteprima» passano dal testo alla vista formattata (CommonMark, `egui_commonmark`); nella vista i link aprono il browser (solo http, https, mailto) o i file del diario (mai fuori dalla sua cartella).
- Salvataggio automatico un secondo dopo l'ultima modifica, al cambio di giorno e alla chiusura. La voce di un giorno senza testo non crea file. La casella «Salvataggio automatico», sempre visibile in alto a destra (richiesta di Mauro, issue #7), spegne il salvataggio: si salva solo con «Salva» o Ctrl+S. Le modifiche restano in memoria finché l'app è aperta, anche cambiando giorno (i giorni non salvati sono in corsivo nel calendario); chiudere l'app o il diario, o aprirne un altro, chiede «Salva / Non salvare / Annulla» (seconda richiesta di Mauro dopo la prova, 24/11/2026).
- Controllo periodico della cartella: se un'altra copia dell'app o il client di sincronizzazione cambia la voce aperta e l'editor non ha modifiche, l'app ricarica; se le ha, salva il testo locale come copia di conflitto (`AAAA-MM-GG (conflitto PC).md`) e mostra le due versioni affiancate per unirle a mano.
- Primo giorno della settimana, lunedì o domenica, da File › Primo giorno della settimana: sta in `.diarycraft/config.json` (`weekStart`), quindi è uguale su ogni computer che apre il diario; un valore che questa versione non conosce si legge come lunedì e resta nel file.
- Lingue: italiano e inglese. Tema chiaro e scuro.

## Seconda fase: il metodo di Kimbro

Si costruisce dopo la v1, sugli stessi file di testo, senza cambiare il formato delle voci.

- **Speed list** (fatta con l'issue #18, tranne la scorciatoia globale): cattura rapida di un pensiero dal campo «Pensiero veloce» in cima all'app, con numero progressivo, *hint* di una-tre parole e soggetto facoltativo (`spunto: testo @soggetto`). I pensieri del giorno finiscono nella sezione `## Speed list` della voce di oggi, in Markdown normale (`1. **spunto** testo @soggetto`), così il diario resta l'unico posto dove si scrive; anche da `diarycraft-cli capture` e dallo strumento MCP `capture`, e `read` restituisce l'elenco. La scorciatoia globale, fuori dalla finestra, resta da fare.
- **Soggetti**: una cartella per soggetto con la pagina P&P (scopo e principi, inclusioni ed esclusioni), le note di approfondimento (POI), ricerca, fonti, progetti.
- **Smistamento**: dalla speed list del giorno ogni pensiero si sposta nel soggetto giusto, a mano; la proposta automatica con un LLM guidato dalle P&P è un passo successivo e facoltativo.
- **Mappe**: SMOC per soggetto e GSMOC su tutti i soggetti, generate dai collegamenti tra le note e modificabili.
- Il libro: Mauro aveva deciso (08/11/2026) che il repository ne contenga la copia integrale. Si può fare anche qui: cartella `book/` con i file originali non modificati, testo della licenza CC BY-SA 1.0 e riga in `ATTRIBUTION.md`; la documentazione che riprende il libro va sotto CC BY-SA 1.0, il codice sotto MIT OR Apache-2.0. Il nome di Kimbro non si usa in modo da far pensare a un suo avallo.

## Repository

Proposta: riusare `mmazzieri/LogBook`. Mauro lo rinomina in `DiaryCraft` dalle impostazioni di GitHub (i vecchi link reindirizzano). Claude lavora su un ramo che sostituisce lo scheletro ABP con il workspace Rust e apre una PR. La storia del 2021 resta nella cronologia. `complete-thought-map` resta com'è finché la seconda fase non lo assorbe; allora si archivia, oppure si cambia la licenza a MIT OR Apache-2.0 per riusarne il README.

## Passi

1. Workspace, CI (`cargo xtask ci`: fmt, clippy, test, deny) e app vuota che apre una cartella.
2. Formato su disco e crate `file`: lettura, scrittura atomica, elenco delle voci, test sui casi di conflitto.
3. Calendario, editor e salvataggio automatico.
4. Ricerca, allegati, import dei layout del 2014.
5. Modifiche esterne e copie di conflitto, provate con due istanze sulla stessa cartella sincronizzata.
6. Pacchetti per Windows, macOS e Linux come BalanceCraft; candidatura al launcher.
7. Seconda fase: Kimbro.

## Identità (fisse dopo l'ingresso nel launcher)

Id e binario `diarycraft`; bundle id, flatpak id e `startup_wm_class` `it.mauromazzieri.diarycraft`; pacchetti deb e rpm `diarycraft`; publisher MSI «Mauro Mazzieri»; UpgradeCode MSI `7C3F609A-64AD-4075-9EBF-B409478B8DC7`, mai da cambiare. DiaryCraft apre una cartella e non un file, quindi non registra tipi di file né estensioni.

## Pacchetti (0.2)

Icona originale (una pagina di quaderno con un segnalibro su fondo blu) disegnata da `tools/icons.py`. `packaging/` e `.github/workflows/release.yml` ripresi da BalanceCraft: MSI (WiX v5, per utente o per macchina) e zip portabile per Windows, DMG universale per macOS (firmato e notarizzato quando ci sono i secret), deb, rpm e tar.gz per Linux x86_64 e aarch64; nessun AppImage. Al tag `v*` una release in bozza con `SHA256SUMS.txt`, che Mauro pubblica.

## Domande per Mauro

- Repository: va bene riusare LogBook rinominato, o Mauro preferisce un repository nuovo?
- Dove sta il diario che Mauro userà per provarlo (cartella locale o OneDrive), e se esistono voci vecchie da importare.
