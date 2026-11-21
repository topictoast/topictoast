---
name: dashboard-react
description: Project rules to write, review or change the React dashboard of topictoast (dashboard/ folder): navigation with React Router, data with TanStack Query, loading and error states per area, vanilla-extract with the DESIGN.md tokens, i18n, accessibility, CSP, tests before code. Use for every piece of work in dashboard/, in subagents too; it wins over generic skills.
---

# React dashboard of topictoast

Project rules for `dashboard/`. They apply together with CLAUDE.md ("Dashboard" section) and the
architecture spine in `docs/architecture/` (AD-12 API and generated types, AD-15 and AD-16 layout
and dependencies with `packages/ui`, C-8 real-time channel). For React performance in general there is the
`vercel-react-best-practices` skill: on conflicts these rules win (no Next.js, no SWR, no "server-"
rule). The rules marked **[gate]** are checked by Biome in `npm run gate`.

## 1. Navigation

- Routes live in `src/AppRoutes.tsx`; the shell is a layout with `<Outlet>`, which stays mounted
  from one page to the next.
- Internal links only with `Link` or `NavLink` from `react-router`, never `<a href="/…">`
  **[gate]**: an `<a>` reloads the document.
- Exits to the outside (login, logout, third-party consents) only through a `src/navigate.ts`
  module; `window.location` is not used anywhere else **[gate]**.

```tsx
// yes
<NavLink to={localize('/runs')}>{t.runs}</NavLink>
// no: reloads the whole page
<a href="/runs">{t.runs}</a>
```

## 2. Data

- Only from the hooks in `src/hooks/` built on TanStack Query (`useQuery`, `useMutation`); no
  `fetch` in components and no requests inside `useEffect`.
- The response types are generated once from the engine's OpenAPI schema into `packages/core`
  (architecture AD-12), and the dashboard uses them through the API client of `packages/core`
  (AD-15, AD-16): they are never generated into `dashboard/` and never rewritten by hand.
- Requests that change state are `useMutation` and carry the CSRF protection decided in the
  architecture.

## 3. States of every area

Every area (card, list, chart, pill) handles loading, error and empty on its own. The shell never
disappears.

- Loading: a `Skeleton` component shaped like the content, `aria-busy="true"` on the area and a
  hidden text «Caricamento di …» ("Loading …"). Never a `<p>Caricamento…</p>` in place of the
  page.
- Error: a message in the area, with «Riprova» ("Retry") when it makes sense.
- Empty: one sentence that says what to do.

## 4. Components

- Function components only, small. Container/Presentational pattern: the pages and containers in
  `src/pages/` wire hooks and components; the components are pure (props → JSX) with `Name.tsx`,
  `Name.css.ts`, `Name.test.tsx`, `Name.stories.tsx`, `index.ts`.
- The design system (tokens and shared components) lives in `packages/ui` (architecture AD-15),
  shared with the kit; `src/components/<Name>/` keeps only the components specific to the
  dashboard.
- Compound for the composite pieces of the design system (shell, menus, cards); Provider for theme,
  session and language; `useReducer` when the state gets complex.
- No components declared inside other components **[gate]**; hooks only at the top level and with
  complete dependencies **[gate]**; stable keys in lists **[gate]**.

## 5. Style

- vanilla-extract, one `.css.ts` per component, only the theme `vars`: the values come from the
  DESIGN.md frontmatter through the tokens of `packages/ui` (a test compares them). A new value
  enters DESIGN.md first.
- No inline `style={…}` **[gate]** and no external resources: the CSP is `default-src 'self'`.
- The size unit (rem or em, spine T-57) is fixed before the first design-system story.
- Light and dark theme.

## 6. Text and language

- Every visible text lives in the `src/i18n/<language>.ts` dictionaries, with the same keys; never
  strings in JSX.
- The voice of the texts follows the rules written in UX (EXPERIENCE.md, "Voice and Tone").
- Dates and numbers with `Intl` in the page language.

## 7. Accessibility

- Native elements (`button`, `a` through `Link`, `nav`, `main`), an accessible name for every
  control, `aria-expanded`/`aria-controls` for what opens or collapses, visible focus.
- Tooltips add to the accessible name, they do not replace it.
- Animations respect `prefers-reduced-motion`.

## 8. Tests (before the code)

- Vitest + Testing Library: the test of every component and hook is written and seen red before
  the code.
- Storybook: light and dark stories for every state.
- The flows are Gherkin scenarios in Italian in `dashboard/features/` (playwright-bdd), seen red
  first.
- Closing: `npm run gate`, browser tests and screenshots green; the screenshots are compared with
  the Penpot boards at 1280, 1440, 1920 px and 390/360 on phone.

## 9. Design and dependencies

- A new page is first drawn in Penpot (file "topictoast"), reusing the components and tokens
  already built there; the user approves, then the code is written. Before creating or moving a
  board, compute the bounding boxes of all the boards on the page and use a free spot.
- Dependencies pinned to an exact version, published at least two weeks earlier; a new dependency
  only if the platform or a library already present is not enough, and if the spine does not say
  otherwise. Exception: a version that fixes a published vulnerability (RustSec or GHSA advisory)
  is pinned at once, citing the advisory in the commit.
