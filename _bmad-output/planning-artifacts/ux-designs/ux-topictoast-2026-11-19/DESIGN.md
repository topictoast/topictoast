---
title: topictoast — DESIGN
status: final
created: 2026-11-19
updated: 2026-11-20
sources:
  - ../../prds/prd-topictoast-2026-11-19/prd.md
  - ../../prds/prd-topictoast-2026-11-19/addendum.md
  - ../../briefs/brief-topictoast-2026-11-18/brief.md
  - .memlog.md
  - .working/color-themes-3.html
  - .working/direction-c-carta-chiara.html
  - .working/directions-brief.md
  - imports/ref-1-pinterest.jpg
  - imports/ref-2-pinterest.jpg
  - imports/ref-3-pinterest.jpg
  - imports/ref-4-dribbble.png
  - imports/ref-4b-dribbble.png
  - mockups/key-editor-desktop.html
  - mockups/key-editor-phone.html
  - mockups/key-execution-view.html
  - mockups/key-stream-editor.html
name: topictoast
description: Un solo design system per il kit React senza marchio, incorporato nella piattaforma ospite, e per la dashboard dell'operatore. Base neutra quasi monocroma, pastelli «Gelato pieno» solo come significato, card su canvas a punti, tema chiaro e tema scuro.
colors:
  # Base neutra. Nel tema scuro ogni colore vale la sua variante con suffisso -dark.
  canvas: '#f6f6f6'
  canvas-dark: '#151517'
  canvas-dot: '#d5d4d9'
  canvas-dot-dark: '#2a2a2d'
  surface: '#ffffff'
  surface-dark: '#1d1c1e'
  border: '#e6e6e8'
  border-dark: '#2f2f32'
  text: '#1a191f'
  text-dark: '#e7e6ea'
  text-secondary: '#585661'
  text-secondary-dark: '#a5a3ae'
  chip: '#eae9ec'
  chip-dark: '#2d2c30'
  # Bordi a 3:1, dove il confine è l'unico segnale: controlli e porte
  border-control: '#8a8991'
  border-control-dark: '#706f77'
  port-border: '#8e8d95'
  port-border-dark: '#74737b'
  # Accenti. Nella dashboard accent-kit cede il posto ad accent-dashboard.
  accent-kit: '#4a4854'
  accent-kit-dark: '#c9c7d1'
  accent-dashboard: '#705ab5'
  accent-dashboard-dark: '#ae9ce2'
  nav-active-dashboard: '#cec3ef'
  nav-active-dashboard-dark: '#3f3658'
  button-primary: '#302e38'
  button-primary-dark: '#d9d8df'
  on-button-primary: '#ffffff'
  on-button-primary-dark: '#18171c'
  # Archi del grafo: l'inattivo a 3:1 sul canvas (T-44)
  edge: '#8e8d95'
  edge-dark: '#74737b'
  edge-active: '#5489c9'
  edge-active-dark: '#86b7f3'
  # Categorie del nodo: primo piano (icona), riempimento (tessera), bordo (tessera)
  category-worker-function: '#774bc3'
  category-worker-function-dark: '#bb99f5'
  category-worker-function-fill: '#ccb8ef'
  category-worker-function-fill-dark: '#433658'
  category-worker-function-border: '#ba9fea'
  category-worker-function-border-dark: '#534171'
  category-custom-node: '#b03b85'
  category-custom-node-dark: '#f48bcd'
  category-custom-node-fill: '#efb8db'
  category-custom-node-fill-dark: '#57364c'
  category-custom-node-border: '#ea9fce'
  category-custom-node-border-dark: '#704160'
  category-flow-control: '#2e8a8a'
  category-flow-control-dark: '#7df2f2'
  category-flow-control-fill: '#b8efef'
  category-flow-control-fill-dark: '#365758'
  category-flow-control-border: '#9feaea'
  category-flow-control-border-dark: '#417071'
  # Stati: primo piano (icona, segmento della barra di stato), riempimento (badge), bordo (badge)
  state-completed: '#2e8a5f'
  state-completed-dark: '#7df2bc'
  state-completed-fill: '#b8efd5'
  state-completed-fill-dark: '#365749'
  state-completed-border: '#9feac7'
  state-completed-border-dark: '#41705b'
  state-running: '#3b6fb0'
  state-running-dark: '#86b7f3'
  state-running-fill: '#b8d1ef'
  state-running-fill-dark: '#364558'
  state-running-border: '#9fc1ea'
  state-running-border-dark: '#415671'
  state-retried: '#8e782f'
  state-retried-dark: '#f2d77d'
  state-retried-fill: '#efe2b8'
  state-retried-fill-dark: '#574f37'
  state-retried-border: '#ead89f'
  state-retried-border-dark: '#706542'
  state-failed: '#b4443c'
  state-failed-dark: '#f4928b'
  state-failed-fill: '#efbcb8'
  state-failed-fill-dark: '#573837'
  state-failed-border: '#eaa49f'
  state-failed-border-dark: '#704442'
  # Famiglia neutra: in attesa, in coda, ricevuta, saltata, annullata, scaduta, sospesa, vicino al limite
  state-neutral: '#82808e'
  state-neutral-dark: '#8b8994'
  state-neutral-fill: '#ececef'
  state-neutral-fill-dark: '#2a292e'
  state-neutral-border: '#dfdee3'
  state-neutral-border-dark: '#38373e'
  # Superfici derivate
  surface-subtle: '#fafafa'
  surface-subtle-dark: '#19181b'
  surface-error: '#f8e1df'
  surface-error-dark: '#493131'
  # Alone di selezione: accento al 20 % (chiaro) o 22 % (scuro), già mescolato sul canvas
  selection-halo: '#d4d3d6'
  selection-halo-dark: '#3d3c40'
  selection-halo-dashboard: '#dbd7e9'
  selection-halo-dashboard-dark: '#373344'
fonts:
  sans: &sans 'system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif'
  mono: &mono 'ui-monospace, "SF Mono", Menlo, Consolas, "DejaVu Sans Mono", monospace'
typography:
  label-xs: { fontFamily: *sans, fontSize: 0.6875rem, fontWeight: '500', lineHeight: '1.45' }
  label-sm: { fontFamily: *sans, fontSize: 0.75rem, fontWeight: '500', lineHeight: '1.5' }
  label-caps: { fontFamily: *sans, fontSize: 0.75rem, fontWeight: '500', lineHeight: '1.45', letterSpacing: 0.04em }
  body-sm: { fontFamily: *sans, fontSize: 0.8125rem, fontWeight: '400', lineHeight: '1.45' }
  body: { fontFamily: *sans, fontSize: 0.875rem, fontWeight: '400', lineHeight: '1.45' }
  body-strong: { fontFamily: *sans, fontSize: 0.875rem, fontWeight: '500', lineHeight: '1.45' }
  heading-sm: { fontFamily: *sans, fontSize: 1rem, fontWeight: '500', lineHeight: '1.4' }
  heading-md: { fontFamily: *sans, fontSize: 1.0625rem, fontWeight: '500', lineHeight: '1.4' }
  heading-lg: { fontFamily: *sans, fontSize: 1.25rem, fontWeight: '500', lineHeight: '1.3' }
  wordmark: { fontFamily: *sans, fontSize: 1.0625rem, fontWeight: '600', lineHeight: '1.3', letterSpacing: -0.01em }
  mono-sm: { fontFamily: *mono, fontSize: 0.75rem, fontWeight: '400', lineHeight: '1.55' }
  mono-md: { fontFamily: *mono, fontSize: 0.8125rem, fontWeight: '400', lineHeight: '1.45' }
  mono-strong: { fontFamily: *mono, fontSize: 0.875rem, fontWeight: '500', lineHeight: '1.45' }
rounded:
  xs: 0.25rem
  sm: 0.375rem
  md: 0.5rem
  lg: 0.625rem
  xl: 0.75rem
  '2xl': 1rem
  full: 9999px
spacing:
  '1': 0.25rem
  '2': 0.5rem
  '3': 0.75rem
  '4': 1rem
  '5': 1.5rem
  '6': 2rem
  dot-grid: 1rem
  # Layout
  rail-width: 4rem
  palette-width: 15rem
  side-panel-width: 22.5rem
  log-panel-height: 12.5rem
  workflow-bar-height: 4rem
  dashboard-bar-height: 3.5rem
  breakpoint-compact: 48rem
  breakpoint-compact-height: 30rem
  # Controlli e bersagli
  control-height: 2.25rem
  target-min: 2.75rem
  nav-item-height: 2rem
  rail-item-height: 2.75rem
  handle-width: 2rem
  handle-height: 0.25rem
  # Nodi e grafo
  node-min-width: 8.75rem
  node-min-height: 4.5rem
  node-max-width: 17.5rem
  category-tag-offset: 1.75em
  icon-tile: 1.625rem
  status-bar-height: 0.625rem
  status-bar-gap: 0.125rem
  status-bar-min-segment: 0.375rem
  # Icone
  icon-size-lg: 1.25rem
  icon-size-md: 0.875rem
  icon-size-sm: 0.75rem
  icon-stroke: '1.6'
  # Selezione, focus, indicatori
  selection-width: 0.09375rem
  halo-width: 0.25rem
  focus-ring-width: 0.09375rem
  focus-ring-offset: 0.125rem
  indicator-width: 0.125rem
  indicator-width-table: 0.1875rem
  # Log
  log-row-min-height: 1.375rem
  log-time-column: 13ch
  log-level-column: 9ch
components:
  canvas:
    background: '{colors.canvas}'
    dot: '{colors.canvas-dot}'
    dot-spacing: '{spacing.dot-grid}'
  list-view:
    background: '{colors.surface}'
    divider: '{colors.border}'
    padding: '{spacing.3}'
    indent: '{spacing.5}'
    name: '{typography.body-strong}'
    name-function: '{typography.mono-strong}'
    detail: '{typography.body-sm}'
    detail-foreground: '{colors.text-secondary}'
  node-card:
    background: '{colors.surface}'
    background-waiting: '{colors.surface-subtle}'
    border: '{colors.border}'
    border-selected: '{colors.accent-kit}'
    border-error: '{colors.state-failed}'
    icon-error: '{colors.state-failed}'
    halo-selected: '{colors.selection-halo}'
    radius: '{rounded.xl}'
    padding: '{spacing.3}'
    label: '{typography.body-strong}'
    label-function: '{typography.mono-strong}'
    min-width: '{spacing.node-min-width}'
    min-height: '{spacing.node-min-height}'
    max-width: '{spacing.node-max-width}'
  category-tag:
    foreground: '{colors.text-secondary}'
    typography: '{typography.label-xs}'
    offset: '{spacing.category-tag-offset}'
    note: 'per ogni categoria c (worker-function, custom-node, flow-control): icon colors.category-c'
  icon-tile:
    size: '{spacing.icon-tile}'
    radius: '{rounded.md}'
    note: 'per ogni categoria c: fill colors.category-c-fill, border colors.category-c-border, icon colors.category-c'
  state-badge:
    foreground: '{colors.text}'
    radius: '{rounded.full}'
    typography: '{typography.label-sm}'
    note: 'per ogni stato s (completed, running, retried, failed, neutral): fill colors.state-s-fill, border colors.state-s-border, icon colors.state-s'
  chip:
    background: '{colors.chip}'
    foreground: '{colors.text-secondary}'
    icon: '{colors.category-worker-function}'
    radius: '{rounded.sm}'
    typography: '{typography.label-xs}'
  fanout-group:
    background: '{colors.surface-subtle}'
    border: '{colors.border}'
    radius: '{rounded.2xl}'
    padding: '{spacing.4}'
    title: '{typography.heading-sm}'
    counts: '{typography.body-sm}'
  status-bar:
    height: '{spacing.status-bar-height}'
    gap: '{spacing.status-bar-gap}'
    min-segment: '{spacing.status-bar-min-segment}'
    radius: '{rounded.xs}'
    segment-running-stripe: '{colors.surface}'
    note: 'per ogni stato s: segment colors.state-s'
  error-card:
    background: '{colors.surface-error}'
    border: '{colors.state-failed-border}'
    radius: '{rounded.xl}'
    label: '{colors.text-secondary}'
    label-icon: '{colors.state-failed}'
    body: '{typography.body-sm}'
  edge:
    stroke: '{colors.edge}'
    stroke-width: 1.25px
    stroke-active: '{colors.edge-active}'
    stroke-active-width: 2px
    dash-pending: '5 4'
    port-radius: 3.5px
    port-fill: '{colors.surface}'
    port-stroke: '{colors.port-border}'
    hit-area: '{spacing.target-min}'
  condition-chip:
    background: '{colors.surface}'
    border: '{colors.border}'
    foreground: '{colors.text}'
    radius: '{rounded.sm}'
    typography: '{typography.label-xs}'
    max-width: '{spacing.node-min-width}'
  workflow-bar:
    background: '{colors.surface}'
    border: '{colors.border}'
    min-height: '{spacing.workflow-bar-height}'
    title: '{typography.heading-md}'
    meta: '{typography.body-sm}'
    meta-foreground: '{colors.text-secondary}'
  button-primary:
    background: '{colors.button-primary}'
    foreground: '{colors.on-button-primary}'
    radius: '{rounded.lg}'
    min-height: '{spacing.control-height}'
    typography: '{typography.body-strong}'
  button-secondary:
    background: '{colors.surface}'
    foreground: '{colors.text}'
    border: '{colors.border}'
    radius: '{rounded.lg}'
    min-height: '{spacing.control-height}'
    typography: '{typography.body-strong}'
  icon-button:
    background: '{colors.surface}'
    icon: '{colors.text}'
    border: '{colors.border}'
    radius: '{rounded.lg}'
    size: '{spacing.control-height}'
    icon-size: '{spacing.icon-size-lg}'
  confirm-dialog:
    background: '{colors.surface}'
    border: '{colors.border}'
    radius: '{rounded.2xl}'
    padding: '{spacing.5}'
    title: '{typography.heading-sm}'
    body: '{typography.body}'
  panel:
    background: '{colors.surface}'
    border: '{colors.border}'
    radius-dashboard: '{rounded.2xl}'
    palette-width: '{spacing.palette-width}'
    side-width: '{spacing.side-panel-width}'
    bottom-height: '{spacing.log-panel-height}'
    title: '{typography.heading-sm}'
    section-heading: '{typography.label-caps}'
    section-heading-foreground: '{colors.text-secondary}'
  tabs:
    foreground: '{colors.text-secondary}'
    foreground-active: '{colors.text}'
    indicator-active: '{colors.accent-kit}'
    indicator-width: '{spacing.indicator-width}'
    typography: '{typography.body-sm}'
  attempt-card:
    background: '{colors.surface-subtle}'
    border: '{colors.border}'
    radius: '{rounded.xl}'
    padding: '{spacing.3}'
    detail: '{typography.body-sm}'
  code-block:
    background: '{colors.surface-subtle}'
    border: '{colors.border}'
    foreground: '{colors.text}'
    radius: '{rounded.lg}'
    typography: '{typography.mono-sm}'
  code-editor:
    background: '{colors.surface}'
    border: '{colors.border-control}'
    border-focus: '{colors.accent-kit}'
    radius: '{rounded.lg}'
    typography: '{typography.mono-sm}'
    gutter-foreground: '{colors.text-secondary}'
    error-line: '{colors.surface-error}'
    error-marker: '{colors.state-failed}'
  log-line:
    typography: '{typography.mono-sm}'
    min-height: '{spacing.log-row-min-height}'
    time-column: '{spacing.log-time-column}'
    level-column: '{spacing.log-level-column}'
    time: '{colors.text-secondary}'
    level: '{colors.text-secondary}'
    message: '{colors.text}'
    icon-info: '{colors.text-secondary}'
    icon-warning: '{colors.state-retried}'
    icon-error: '{colors.state-failed}'
    icon-ok: '{colors.state-completed}'
  palette-item:
    background: '{colors.surface}'
    border: '{colors.border}'
    radius: '{rounded.lg}'
    label: '{typography.body}'
    description: '{typography.body-sm}'
    description-foreground: '{colors.text-secondary}'
    group-heading: '{typography.label-caps}'
  form-field:
    background: '{colors.surface}'
    border: '{colors.border-control}'
    border-focus: '{colors.accent-kit}'
    border-error: '{colors.state-failed}'
    radius: '{rounded.lg}'
    min-height: '{spacing.control-height}'
    label: '{typography.body-sm}'
    label-foreground: '{colors.text-secondary}'
    help-foreground: '{colors.text-secondary}'
  select:
    background: '{colors.surface}'
    border: '{colors.border-control}'
    border-focus: '{colors.accent-kit}'
    indicator: '{colors.text-secondary}'
    radius: '{rounded.lg}'
    min-height: '{spacing.control-height}'
  data-mapping:
    divider: '{colors.border}'
    gap: '{spacing.2}'
    field: '{typography.body-sm}'
    field-type: '{typography.mono-sm}'
    field-type-foreground: '{colors.text-secondary}'
    source: '{typography.mono-md}'
  schedule-section:
    gap: '{spacing.3}'
    expression: '{typography.mono-md}'
    next-run: '{typography.body-sm}'
    next-run-foreground: '{colors.text-secondary}'
  input-form:
    gap: '{spacing.3}'
    list: '{typography.mono-sm}'
    list-border: '{colors.border-control}'
    count: '{typography.body-sm}'
    count-foreground: '{colors.text-secondary}'
  validation-message:
    background: '{colors.surface-error}'
    foreground: '{colors.text}'
    icon: '{colors.state-failed}'
    typography: '{typography.body-sm}'
    path: '{typography.mono-md}'
    radius: '{rounded.lg}'
  tooltip:
    background: '{colors.button-primary}'
    foreground: '{colors.on-button-primary}'
    radius: '{rounded.md}'
    typography: '{typography.body-sm}'
  rail-item:
    width: '{spacing.rail-width}'
    min-height: '{spacing.rail-item-height}'
    icon-size: '{spacing.icon-size-lg}'
    foreground: '{colors.text-secondary}'
    foreground-active: '{colors.text}'
    background-active: '{colors.canvas}'
    icon-active: '{colors.accent-kit}'
    radius: '{rounded.lg}'
    typography: '{typography.label-xs}'
  tenant-selector:
    background: '{colors.surface}'
    border: '{colors.border}'
    padding: '{spacing.2}'
    label: '{typography.body-sm}'
    label-foreground: '{colors.text-secondary}'
  nav-item:
    min-height: '{spacing.nav-item-height}'
    foreground: '{colors.text-secondary}'
    foreground-active: '{colors.text}'
    background-active: '{colors.nav-active-dashboard}'
    indicator-active: '{colors.accent-dashboard}'
    indicator-width: '{spacing.indicator-width}'
    radius: '{rounded.md}'
    typography: '{typography.body}'
  dashboard-bar:
    background: '{colors.surface}'
    border: '{colors.border}'
    min-height: '{spacing.dashboard-bar-height}'
    wordmark: '{typography.wordmark}'
    page-title: '{typography.heading-lg}'
  login-card:
    background: '{colors.surface}'
    border: '{colors.border}'
    radius: '{rounded.2xl}'
    padding: '{spacing.5}'
    title: '{typography.heading-md}'
  data-table:
    background: '{colors.surface}'
    header-background: '{colors.surface-subtle}'
    header-foreground: '{colors.text-secondary}'
    header: '{typography.label-sm}'
    cell: '{typography.body-sm}'
    cell-id: '{typography.mono-md}'
    divider: '{colors.border}'
    selected-indicator: '{colors.accent-kit}'
    selected-indicator-width: '{spacing.indicator-width-table}'
  history-list:
    time: '{typography.mono-sm}'
    time-foreground: '{colors.text-secondary}'
    detail: '{typography.body-sm}'
    divider: '{colors.border}'
  recovery-banner:
    background: '{colors.state-retried-fill}'
    border: '{colors.state-retried-border}'
    icon: '{colors.state-retried}'
    foreground: '{colors.text}'
    radius: '{rounded.2xl}'
    title: '{typography.body-strong}'
    body: '{typography.body-sm}'
  pause-notice:
    background: '{colors.state-neutral-fill}'
    border: '{colors.state-neutral-border}'
    icon: '{colors.state-neutral}'
    foreground: '{colors.text}'
    radius: '{rounded.lg}'
    typography: '{typography.body-sm}'
  load-error:
    foreground: '{colors.text}'
    icon: '{colors.state-failed}'
    typography: '{typography.body}'
  bottom-sheet:
    background: '{colors.surface}'
    border: '{colors.border}'
    radius: '{rounded.2xl}'
    handle: '{colors.border}'
    handle-width: '{spacing.handle-width}'
    handle-height: '{spacing.handle-height}'
  empty-state:
    foreground: '{colors.text-secondary}'
    typography: '{typography.body}'
  skeleton:
    fill: '{colors.chip}'
    radius: '{rounded.md}'
---

# topictoast — DESIGN

> Le spine (questo file ed `EXPERIENCE.md`) vincono, in caso di conflitto, su ogni mock in `mockups/`, `imports/` o `.working/`. I mock approvati stanno in `mockups/` e illustrano le spine; `.working/` resta l'esplorazione (direzioni, palette). Ogni assunzione ha un tag con id `A-n` e una riga in `EXPERIENCE.md`, Elementi aperti.

## Brand & Style

topictoast ha due superfici e un solo design system. Il **kit React** vive dentro la piattaforma
ospite senza marchio: chi lo usa non deve accorgersi che sotto c'è topictoast (PRD §4.9). Per
questo i suoi valori predefiniti sono neutri, a partire dall'accento grafite, e l'ospite li
sostituisce con i propri (FR-44). La **dashboard dell'operatore** usa gli stessi componenti e gli
stessi token: cambia solo l'accento, che diventa viola, e compare il wordmark «topictoast».

Il linguaggio è quello della direzione **«Carta chiara»**
([`.working/direction-c-carta-chiara.html`](.working/direction-c-carta-chiara.html)): card
bianche su un canvas grigio chiaro a griglia di punti, un'etichetta di categoria con tessera icona
sopra ogni nodo, pesi tipografici regolari, spazio generoso, pulsante primario scuro, contorno
sul nodo selezionato, barra icone a sinistra. Il tema scuro ha la stessa struttura. Gli archi sono
curvi. I tratti vengono dai riferimenti di Marco: [ref-3](imports/ref-3-pinterest.jpg) e
[ref-4](imports/ref-4-dribbble.png) per «Carta chiara», [ref-1](imports/ref-1-pinterest.jpg) e
[ref-2](imports/ref-2-pinterest.jpg) per i punti del canvas e gli archi,
[ref-4b](imports/ref-4b-dribbble.png) per il pannello di configurazione e la barra.

I colori sono quelli di **«Gelato pieno»**
([`.working/color-themes-3.html`](.working/color-themes-3.html)): pastelli pieni ma non vivaci,
testo scuro sopra i riempimenti, bordi morbidi un passo più scuri del riempimento. Il colore
compare solo come significato: stato, categoria del nodo, percorso attivo. Mai come decorazione.

## Colors

Tre regole valgono per tutti i token.

- **Tema scuro.** Ogni riferimento a un colore vale, nel tema scuro, la sua variante con
  suffisso `-dark`: `{colors.surface}` diventa `{colors.surface-dark}`. I componenti citano solo
  il nome chiaro.
- **Accento per superficie.** Nella dashboard `{colors.accent-kit}` diventa
  `{colors.accent-dashboard}` e `{colors.selection-halo}` diventa
  `{colors.selection-halo-dashboard}`. Nient'altro cambia.
- **Testo sempre neutro.** Le parole stanno in `{colors.text}` o `{colors.text-secondary}`; su
  ogni riempimento `-fill` di stato o di categoria il testo è solo `{colors.text}`. I primi piani
  di categoria e di stato colorano solo icone, punti e segmenti: sono scelti sulla soglia di 3:1
  degli indicatori non testuali, e sul bianco alcuni restano sotto 4,5:1 (completata 4,27,
  ritentata 4,31, neutro 3,87).

**Base neutra.** `{colors.canvas}` con i punti `{colors.canvas-dot}` è il fondo di canvas e
pagine, `{colors.surface}` quello di card, barre e pannelli, separati da un filo
`{colors.border}`; `{colors.chip}` è il fondo dei chip.

**Bordi a 3:1.** `{colors.border-control}` è il confine di campi, `code-editor`, select,
checkbox, radio e switch; `{colors.port-border}` quello delle porte, che sono bersagli
interattivi. Il filo morbido `{colors.border}` resta su card, pannelli, pulsanti e voci di
palette, che hanno testo: il bordo più scuro vale solo dove il confine è l'unico segnale del
controllo.

**Accenti.** `{colors.accent-kit}` (grafite) segna selezione, anello di focus e tab attiva nel
kit; è neutro di proposito, così si adatta a ogni ospite che lo sostituisce.
`{colors.accent-dashboard}` (viola) fa lo stesso nella dashboard ed è il colore di topictoast; la
voce di navigazione attiva della dashboard ha il fondo `{colors.nav-active-dashboard}`.
`{colors.button-primary}` con `{colors.on-button-primary}` è il pulsante primario scuro, uguale
nelle due superfici.

**Archi.** `{colors.edge}` è l'arco inattivo, a 3:1 sul canvas; `{colors.edge-active}` è il
percorso in esecuzione, più spesso (larghezze in `edge`).

**Categorie.** Funzione del worker (viola), Nodo personalizzato (rosa), Controllo di flusso
(acqua). Ogni categoria ha un primo piano per l'icona, un riempimento e un bordo per la tessera
icona: per esempio `{colors.category-worker-function}`, `{colors.category-worker-function-fill}`,
`{colors.category-worker-function-border}`. Categoria e stato non
condividono mai una tinta: la distanza minima è 28° (Controllo di flusso 180°, completata 152°).

**Stati.** Completata (menta), in esecuzione (cielo), ritentata (burro), fallita (rosa-corallo) e
una famiglia neutra grigia per in attesa, in coda, ricevuta, saltata, annullata, scaduta, pipeline
stream e pianificazione sospese e tenant vicino al limite. Ogni stato ha primo piano, riempimento
e bordo: per esempio
`{colors.state-failed}`, `{colors.state-failed-fill}`, `{colors.state-failed-border}`. Il primo
piano colora anche i segmenti della barra di stato. Gli stati che condividono `state-neutral` si
distinguono per icona e parola.

**Superfici derivate.** `{colors.surface-subtle}` è il fondo di task non ancora partiti, gruppo
fan-out e blocchi secondari; `{colors.surface-error}` quello della scheda dell'elemento fallito e
dei messaggi di errore; `{colors.selection-halo}` è l'alone attorno al nodo selezionato
[ASSUMPTION A-27].

### Contrasti

Soglie WCAG 2.1: 4,5:1 per ogni testo, 3:1 per gli indicatori non testuali che portano
informazione. Ogni riga riporta la coppia peggiore della classe, ricalcolata con la formula WCAG
sugli hex del frontmatter. La matrice completa diventa un test generato dai token nel gate della
dashboard (compito della prima storia del design system).

| Classe | Coppia peggiore | Chiaro | Scuro | Soglia |
|---|---|---|---|---|
| testo | testo / riempimento di stato (fallita in chiaro, completata in scuro) | 10,44 | 6,46 | 4,5 |
| testo secondario | testo secondario / scheda errore | 5,77 | 4,78 | 4,5 |
| accento | accento dashboard / voce nav attiva | 3,32 | 4,60 | 3 |
| icone | icona di categoria / tessera (chiaro), icona di stato / riempimento (scuro) | 3,25 | 4,19 | 3 |
| `border-control` | bordo / superficie | 3,46 | 3,42 | 3 |
| `port-border` | porta / canvas (chiaro), porta / superficie (scuro) | 3,04 | 3,62 | 3 |
| archi | arco inattivo / canvas (chiaro), arco inattivo / gruppo fan-out (scuro) | 3,04 | 3,77 | 3 |
| segmenti | segmento neutro / gruppo fan-out | 3,71 | 5,14 | 3 |

Nessuna deroga sotto 3:1 (T-44): l'arco attivo arriva a 3,35:1 in chiaro e 8,75:1 in scuro.

**Decorativi.** L'alone di selezione è 1,38–1,67:1 sul canvas: a segnalare la selezione è il
contorno (8,95:1 sulla superficie). `{colors.border}` è 1,25:1 sulla superficie e vale solo dove il
testo identifica l'elemento.

**Coppie vietate.** `{colors.text-secondary}` su un riempimento `-fill` di stato o di categoria:
in scuro scende a 3,23–4,17:1 (completata, ritentata, in esecuzione, fallita), in chiaro a 4,30:1
su fallita.

## Typography

Due stack di sistema, dichiarati una volta: `{fonts.sans}` per l'interfaccia, `{fonts.mono}` per
dati, ID, nomi di funzione, indicatori, orari, log, JSON, jq e SQL. Stack di sistema per scelta
(T-54): nessun file di font nel bundle, nessun conflitto con la CSP `default-src 'self'`, e
l'ospite li sostituisce (FR-44).

Scala in rem, base fissa 16 px. Pesi 400 e 500; il 600 solo nel wordmark.

| Token | Uso |
|---|---|
| `label-xs` | etichetta di categoria, chip, voce della barra icone |
| `label-sm` | badge di stato, intestazioni di tabella |
| `label-caps` | titoli di sezione nei pannelli («Tentativi», «Input»), in maiuscolo |
| `body-sm` | metadati, celle di tabella, conteggi, tab, messaggi |
| `body` | testo di base |
| `body-strong` | nome del nodo, pulsanti |
| `heading-sm` | titolo del pannello, nome del gruppo fan-out |
| `heading-md` | nome del workflow nella barra |
| `heading-lg` | titolo di pagina della dashboard |
| `wordmark` | «topictoast», solo dashboard |
| `mono-sm` | righe di log, JSON, orari |
| `mono-md` | ID di worker, indicatori, percorsi negli errori |
| `mono-strong` | nome di funzione sul nodo (`dns_lookup`) |

Regole:

- I nomi di funzione (`censys_search`) e gli ID (`worker-ti-02`, `203.0.113.7`) sono sempre mono.
  I nomi scritti dal tenant («Ricevi elenco», «Calcola punteggio») sono sans.
- Numeri di conteggi, durate e tempi con cifre tabulari.
- Il maiuscolo compare solo in `label-caps`.

## Layout & Spacing

Scala a base 4: `{spacing.1}` 4 px, `{spacing.2}` 8, `{spacing.3}` 12, `{spacing.4}` 16,
`{spacing.5}` 24, `{spacing.6}` 32. Tutto in rem. I punti del canvas stanno su una griglia di
`{spacing.dot-grid}`.

**Area workflow del kit, desktop** (board 1280, 1440, 1920).

- Da sinistra: barra icone (`{spacing.rail-width}`) · palette (`{spacing.palette-width}`, solo
  nell'editor, comprimibile) [ASSUMPTION A-36] · canvas, con sopra la barra del workflow
  (`{spacing.workflow-bar-height}`) · pannello destro (`{spacing.side-panel-width}`).
- Il pannello a tab Log/Input/Output (`{spacing.log-panel-height}`) c'è solo nella vista delle
  esecuzioni: sta sotto il canvas soltanto, e il pannello destro li affianca entrambi; l'editor
  non ce l'ha.
- I pannelli sono agganciati, con un filo verso il canvas (T-51).
- Con la credenziale dell'integratore la fascia `tenant-selector` sta sopra tutta l'area. In alto
  c'è l'intestazione dell'app ospite, che non fa parte del kit.
- Il grafo scorre da sinistra a destra.
- Fin dove si estende la barra del workflow resta aperto (`EXPERIENCE.md`, Elementi aperti).
- Mock: [editor](mockups/key-editor-desktop.html), barra icone, palette, canvas e pannello del
  task; [vista delle esecuzioni](mockups/key-execution-view.html), pannello in basso, modulo di
  «Esegui» e `list-view`.

**Editor delle pipeline stream, desktop.** Barra in alto; al centro la query nel `code-editor` e
sotto «Ultime righe emesse» in una `data-table`; a destra il pannello «Parametri»
(`{spacing.side-panel-width}`). Mock: [editor delle pipeline stream](mockups/key-stream-editor.html).

**Dashboard, desktop.** `dashboard-bar` in alto con wordmark, navigazione e operatore; pagina con
margini `{spacing.5}` in verticale e `{spacing.6}` in orizzontale; pannelli affiancati con spazio
`{spacing.5}`. La navigazione sta solo in alto, con `nav-item`; la barra icone è del kit
[ASSUMPTION A-29].

**Telefono** (board 390 e 360). Il layout compatto vale sotto `{spacing.breakpoint-compact}` di
larghezza e, con puntatore grossolano, sotto `{spacing.breakpoint-compact-height}` di altezza
(regole in `EXPERIENCE.md`, Responsive & Platform). Stesso canvas a tutta larghezza; il grafo
scorre dall'alto in basso; palette, configurazione del task e log salgono dal basso come
`bottom-sheet`; la barra del workflow va su due righe, nome e versione sopra, azioni sotto. Con
`pointer: coarse` ogni controllo, maniglia compresa, misura almeno `{spacing.target-min}`. Mock:
[editor sul telefono](mockups/key-editor-phone.html), 390 px.

**Nodi.** Card da `{spacing.node-min-width}` × `{spacing.node-min-height}` che si allarga col
nome fino a `{spacing.node-max-width}`; oltre, il nome va a capo (regola del nome intero in Do's
and Don'ts) [ASSUMPTION A-30]. Il gruppo fan-out racchiude i suoi nodi con `{spacing.4}` di
margine interno; l'etichetta di categoria sta sopra la card a `{spacing.category-tag-offset}`,
in em, così segue il testo.

**Testo ridimensionabile** (WCAG 1.4.4, 1.4.10, 1.4.12). Barre, voci, controlli e righe di log
hanno altezze minime, non fisse; le colonne del log sono in `ch`; i nodi si ridimensionano col
testo. Prove a 320 px, al 200 % e con la spaziatura del testo di 1.4.12.

## Elevation & Depth

La profondità viene dal tono, non dall'ombra: canvas `{colors.canvas}`, sopra superficie
`{colors.surface}`, separata da un filo `{colors.border}`. Nodi, card e pannelli non hanno ombra
[ASSUMPTION A-33]. Il gruppo fan-out e la card di un task non partito si distinguono con il fondo
`{colors.surface-subtle}`.

## Shapes

I raggi dei componenti stanno nelle chiavi `radius` del frontmatter; a pillola
(`{rounded.full}`) solo i badge di stato.

Le porte sono cerchi (`port-radius` di `edge`). Gli archi sono curve di Bézier morbide tra porta
e porta; il tratteggio è descritto in `edge`.

**Icone.** SVG in linea (la CSP vieta risorse esterne), tratto `{spacing.icon-stroke}`,
estremità arrotondate, nelle misure `{spacing.icon-size-lg}`, `{spacing.icon-size-md}` e
`{spacing.icon-size-sm}`.

| Gruppo | Significato | Icona |
|---|---|---|
| Categorie | Funzione del worker | graffe |
| | Nodo personalizzato | `</>` |
| | Controllo di flusso | ramo |
| Stati | completata | spunta |
| | in esecuzione | arco d'orologio |
| | ritentata | freccia circolare |
| | fallita | croce |
| | in attesa | cerchio tratteggiato |
| | in coda | righe |
| | saltata, annullata, scaduta | cerchio barrato |
| | ricevuta | righe, come in coda |
| Worker | collegato | spunta, come completata |
| | non risponde | croce, come fallita |
| Pipeline stream | attiva (anche una pianificazione) | arco d'orologio, come in esecuzione |
| | rallentata | freccia circolare, come ritentata |
| | sospesa (anche una pianificazione) | pausa |
| Quote | «Vicino al limite», «Al limite» | un'icona ciascuna, distinta da quelle degli stati |

Il disegno di partenza delle icone di categoria e di completata, in esecuzione, ritentata, fallita
e in attesa è in [vista delle esecuzioni](mockups/key-execution-view.html); quello delle altre è
nei mock in `.working/`. Le icone della barra icone cambiano da un mock all'altro e, con le nuove,
si disegnano in Penpot.

## Components

Ogni componente prende i valori dalle sue chiavi nel frontmatter; qui stanno anatomia, contenuto
d'esempio ed eccezioni. Le matrici per stato o per categoria seguono la regola della chiave
`note`. Una parte dei componenti ha l'aspetto derivato solo dai token [ASSUMPTION A-32].

| Mock | Componenti |
|---|---|
| [editor](mockups/key-editor-desktop.html) | `palette-item`, `node-card` con errore, `condition-chip`, `data-mapping` |
| [editor sul telefono](mockups/key-editor-phone.html) | `bottom-sheet` |
| [vista delle esecuzioni](mockups/key-execution-view.html) | `fanout-group`, `error-card`, `attempt-card`, `log-line`, `input-form`, `list-view` |
| [editor delle pipeline stream](mockups/key-stream-editor.html) | `code-editor` SQL, `data-table` |

**Selezione e focus.** La selezione è un contorno di `{spacing.selection-width}` in
`{colors.accent-kit}` più l'alone `{colors.selection-halo}` di `{spacing.halo-width}`. L'anello
di focus è a doppio filo, interno `{colors.surface}` ed esterno `{colors.accent-kit}`, largo
`{spacing.focus-ring-width}` a `{spacing.focus-ring-offset}` dall'elemento: resta distinto dal
contorno di selezione anche sul nodo selezionato.

- **canvas** — Fondo a punti. Porta i tre `icon-button` dello zoom e, nell'editor su desktop,
  l'`icon-button` «Riordina» (comportamento in `EXPERIENCE.md`).
- **list-view** — Al posto del canvas, nella stessa area; la riga selezionata apre il pannello del
  task in sola lettura, come dal canvas (`EXPERIENCE.md`). Una riga per task divisa da `divider`:
  prima riga `icon-tile`, nome (`name`, o `name-function` per un nome di funzione) e
  `state-badge` nella vista delle esecuzioni; sotto, in `detail`, categoria, «dipende da …»,
  «precede …» e i conteggi del fan-out. I task dentro un `fanout-group` rientrano di `indent`
  sotto la riga del gruppo; sotto uno `switch` ogni ramo apre con la sua etichetta.
- **node-card** — Prima riga: `icon-tile` e nome (`label`, o `label-function` per un nome di
  funzione). Seconda riga: `state-badge` nella vista delle esecuzioni, più il `chip` `TS` o `RS`
  per un nodo personalizzato. Nell'editor un task con errori porta sotto il nome l'icona croce in
  `icon-error` e la parola «Errore» o «N errori» in `{colors.text}`; il bordo `border-error` è solo
  un rinforzo (WCAG 1.4.1). Task non ancora partito: fondo `background-waiting`, bordo pieno.
  Selezionato: contorno e alone (Selezione e focus).
- **category-tag** — Sopra la card, senza fondo: icona nel primo piano della categoria e parola
  («Funzione del worker», «Nodo personalizzato», «Controllo di flusso»).
- **icon-tile** — Quadrato con riempimento e bordo della categoria, icona nel suo primo piano.
- **state-badge** — Pillola con riempimento e bordo dello stato, icona nel primo piano dello
  stato e parola. Icona e parola ci sono sempre.
- **chip** — Usi: linguaggio del nodo (`TS`, `RS`), versione («versione 3»), provenienza
  («Dell'integratore», «da codice»), funzioni registrate nella tabella dei worker con l'icona di
  categoria (`icon`) davanti.
- **fanout-group** — Contenitore con l'etichetta «Controllo di flusso» sopra. Intestazione:
  tessera, nome con conteggio («Per ogni indicatore · 300») e badge di stato del gruppo. Sotto:
  `status-bar` e griglia di conteggi su due colonne, ognuno con icona di stato, parola in
  `{colors.text-secondary}` e numero in `{colors.text}` («completate 287», «in esecuzione 9»,
  «ritentate 3», «fallite 1»).
- **status-bar** — Segmenti proporzionali nel primo piano dello stato (T-44), separati da `gap`,
  minimo `min-segment` per segmento (1 fallita su 300 resta visibile); il segmento in esecuzione
  è a righe diagonali alternate con `segment-running-stripe`.
- **error-card** — Scheda dell'elemento fallito del fan-out. Sopra, l'etichetta «Elemento
  fallito» con l'icona `label-icon`. Dentro: badge «Fallita», indicatore in mono, «elemento 87 di
  300», poi funzione, tentativi e messaggio d'errore.
- **edge** — Curva sottile; percorso in esecuzione in `stroke-active`, largo
  `stroke-active-width`; verso un task non partito e lungo un ramo non preso tratteggio
  `dash-pending`. Porte: cerchi neutri con bordo `port-stroke`, a 3:1 perché sono bersagli; il
  colore resta a nodi e archi (T-50). Al tocco l'arco ha un'area invisibile larga `hit-area`.
- **condition-chip** — Etichetta del ramo a metà di un arco che esce da uno `switch`, come in
  [ref-4b](imports/ref-4b-dribbble.png). Sempre neutra, anche per «altrimenti»: mai la famiglia
  fallita, un ramo non è un errore. Oltre `max-width` l'etichetta va a capo, senza puntini.
- **workflow-bar** — Nome del workflow (`title`), `chip` di versione (in un'esecuzione, la
  versione con cui è partita) ed eventuali `chip` «Dell'integratore» o «da codice», separatore,
  badge di stato dell'esecuzione, metadati («In esecuzione · Avviata alle 14:22 dal tenant ·
  durata 1 min 48 s»), azioni a destra. Nell'editor: «Esegui» come `button-secondary` con l'icona
  play in `{colors.text}`, mai verde, e «Salva» come `button-primary` (T-52).
- **button-primary** — Icona a sinistra facoltativa.
- **button-secondary** — Stesse misure del primario (esempio: «Aggiorna»).
- **icon-button** — `button-secondary` quadrato con la sola icona e un nome accessibile: X del
  `panel`, zoom e «Riordina» del `canvas`, «Comprimi palette», pulsanti in testa al
  `bottom-sheet`.
- **confirm-dialog** — Titolo con la domanda, una frase sulla conseguenza, un `button-primary`
  con l'azione e un `button-secondary` «Chiudi».
- **panel** — Tre posizioni.
  - A sinistra, nell'editor: la palette (`palette-item` per gruppo, come in
    [ref-3](imports/ref-3-pinterest.jpg)); compressa, resta una colonna larga quanto
    `{spacing.rail-width}` con il solo `icon-button` «Apri palette».
  - A destra: in cima un `chip` con icona e parola della categoria, nome (mono per una funzione),
    sottotitolo («Indicatore 203.0.113.7 · elemento 142 di 300») e `icon-button` X, come in
    [ref-4b](imports/ref-4b-dribbble.png); sezioni con titolo `section-heading`.
  - In basso: `tabs` Log, Input, Output.
  - Forma: in tutta l'area workflow del kit (editor, vista delle esecuzioni, editor delle pipeline
    stream, come nei mock) i pannelli sono agganciati ai bordi dell'area, senza margine né raggio, con
    un filo sul lato verso il canvas (T-51); nella dashboard sono riquadri con angoli
    `radius-dashboard`.
- **tabs** — La tab attiva ha una linea inferiore `indicator-active`. A destra, il contesto in
  mono («censys_search · 203.0.113.7»).
- **attempt-card** — Una card per tentativo: badge con esito («Tentativo 1 · fallito»), orario in
  mono, versione del nodo eseguita, sotto il dettaglio («timeout dopo 30 s»).
- **code-block** — JSON in sola lettura, a capo automatico.
- **code-editor** — Editor SQL, JSON e jq: numeri di riga in `gutter-foreground`; la riga con errore
  ha fondo `error-line` e un segno `error-marker` nel margine.
- **log-line** — Griglia di tre colonne (orario `time-column`, livello `level-column`,
  messaggio), righe alte almeno `min-height`. Livello: parola neutra («info», «avviso», «errore»,
  «ok») preceduta da un'icona colorata (`icon-info`, `icon-warning`, `icon-error`, `icon-ok`).
- **palette-item** — Voce a riquadro con bordo [ASSUMPTION A-35]: `icon-tile` e nome, sotto la
  descrizione del manifest in `description`, a capo e mai troncata. Le voci stanno sotto i titoli di
  gruppo (`group-heading`), come in [ref-3](imports/ref-3-pinterest.jpg); le voci omonime si
  distinguono con il `chip` di provenienza.
- **form-field** — Etichetta sopra; campo con bordo `border-control`; aiuto sotto in
  `help-foreground` («predefinito: 30 s, massimo: 10 min»). Focus: `border-focus`. Errore:
  `border-error` più `validation-message` sotto.
- **select** — `form-field` con indicatore a destra. Usi: segreti di una connessione nel
  pannello, filtri «Tenant», «Workflow», «Stato».
- **data-mapping** — Sezione «Dati» del pannello del task: una riga per campo d'ingresso, divisa
  da `divider`. Sopra, nome del campo in `field`, tipo in `field-type` («testo», «numero»,
  «elenco») e a destra la scelta della fonte («Valore», «Da un task», «jq»); sotto, la fonte: un
  `form-field`, un `select` con la fonte scritta in `source` (`dns_lookup · record`) o un
  `code-editor` jq.
- **schedule-section** — Sezione «Pianificazione» del pannello del workflow: `select` «Ripeti»,
  intervallo come `form-field` o espressione cron in `expression`, poi la riga «Prossima
  scadenza» in `next-run`, `state-badge` neutro «Sospesa» quando serve e `button-secondary`
  «Sospendi» o «Riprendi».
- **input-form** — Modulo di «Esegui» nel pannello destro: un `form-field` per campo d'ingresso
  del workflow, a distanza `gap`. Il campo elenco è un'area di testo in `list` con bordo
  `list-border`, con sotto il conteggio in `count`; in fondo il `button-secondary` «Mostra JSON»
  e il `button-primary` «Esegui».
- **validation-message** — Riquadro con icona e testo; nome del task, percorso, riga e colonna in
  `path`.
- **tooltip** — Etichetta breve su fondo scuro accanto al controllo.
- **rail-item** — Voce alta almeno `min-height`: icona sopra un'etichetta sempre visibile
  [ASSUMPTION A-34]. Voce attiva: fondo `background-active`, testo `foreground-active`, icona
  `icon-active`.
- **tenant-selector** — Solo kit, solo ruolo integratore. Fascia a tutta larghezza in cima
  all'area workflow, con filo inferiore `border` e margine interno `padding`: etichetta «Tenant»
  in `label` e un `select` con il tenant scelto.
- **nav-item** — Solo dashboard, nella `dashboard-bar`. Voce attiva: fondo `background-active`,
  testo `foreground-active` e linea interna inferiore `indicator-active`.
- **dashboard-bar** — Solo dashboard: wordmark «topictoast» a sinistra, `nav-item` al centro,
  operatore a destra; sotto, il titolo di pagina in `page-title`.
- **login-card** — Pagina Accesso: titolo, un `form-field` per la credenziale, un
  `button-primary` «Accedi» e la frase su dove si trova la credenziale del primo avvio.
- **data-table** — Intestazioni su `header-background`; ID in `cell-id`. Riga selezionata: linea
  `selected-indicator` larga `selected-indicator-width` sul bordo sinistro. Stato in
  `state-badge`, funzioni in `chip`.
- **history-list** — Cronologia di un worker: orario a sinistra, a destra il badge della fase e
  sotto funzione e indicatore.
- **recovery-banner** — Solo dashboard. Riquadro a tutta larghezza sotto il titolo della pagina,
  nella famiglia ritentata, con icona di avviso; titolo e corpo entrambi in `{colors.text}`. Lo
  stesso riquadro, con il suo testo, avvisa del disco pieno (`EXPERIENCE.md`, `recovery-banner`).
- **pause-notice** — Solo kit. Avviso in linea nella barra del workflow, nella famiglia neutra:
  è una pausa, non un errore.
- **load-error** — Messaggio con icona nell'area che non si è caricata, più un
  `button-secondary` «Riprova»; la cornice resta.
- **bottom-sheet** — Solo telefono. Foglio con filo superiore e angoli superiori arrotondati; in
  testa gli `icon-button` «Chiudi» ed «Espandi»/«Comprimi», come la X del `panel`; la maniglia
  (`handle-width` × `handle-height`) è un'aggiunta per il trascinamento. Si stacca dal canvas per
  tono e bordo.
- **empty-state** — Una frase centrata nell'area, più al massimo un `button-secondary`.
- **skeleton** — Blocchi nella forma del contenuto atteso.

## Do's and Don'ts

| Do | Don't |
|---|---|
| Colore solo come significato: stato, categoria, percorso attivo | Colore come decorazione, sfumature, fondi colorati per le sezioni |
| Pastelli «Gelato pieno» come riempimenti, bordi un passo più scuri del riempimento | Riempimenti vivaci o bordi accesi |
| Tinte riconoscibili per stati e categorie | Interfaccia solo grigia e spenta |
| Su un riempimento di stato o di categoria solo `{colors.text}` (regola in Colors) | `{colors.text-secondary}` o parole colorate su un riempimento |
| Stato sempre con icona e parola | Stato detto solo dal colore |
| Nome del nodo sempre intero, a capo se serve | Puntini di sospensione nel nome di un nodo («dns_look…») |
| Categoria e stato con tinte distinte (minimo 28°) | Una tinta condivisa tra una categoria e uno stato |
| Nel kit, accento grafite neutro e nessun nome o logo di topictoast, salvo richiesta dell'ospite (FR-44) | Viola di topictoast, wordmark o logo dentro il kit |
| Nella palette solo funzioni, nodi personalizzati e segreti del tenant, più i costrutti di FR-1 | Catalogo di nodi, template e account propri nel kit (PRD §7, SM-C3) |
| Archi curvi; il percorso attivo è l'unico arco colorato | Archi ortogonali |
| Profondità con tono e filo | Ombre su nodi e card |
| Valori in rem dai token del frontmatter | Valori scritti a mano nei `.css.ts` senza passare da qui |
