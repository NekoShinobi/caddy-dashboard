# Caddy Dashboard redesign brand specification

The redesign keeps the existing cool, technical Caddy palette and typography, then reorganizes them into a denser operations workspace.

```css
:root {
  --bg: oklch(0.145 0.015 230);
  --surface: oklch(0.185 0.018 230);
  --fg: oklch(0.94 0.008 220);
  --muted: oklch(0.68 0.018 220);
  --border: oklch(0.31 0.02 230);
  --accent: oklch(0.72 0.13 205);
}
```

- Display: `'DM Sans Variable', 'Avenir Next', 'Segoe UI', sans-serif`
- Body: `'DM Sans Variable', 'Avenir Next', 'Segoe UI', sans-serif`
- Mono: `'JetBrains Mono Variable', 'SFMono-Regular', Consolas, monospace`

Observed visual rules:

- Cool blue-black surfaces provide the working canvas; cyan is reserved for selection and primary data emphasis.
- Operational values and timestamps use mono type with tabular numerals.
- Controls have compact 44px targets, clear borders, and restrained 9-14px radii.
- The interface supports light and dark appearances through semantic tokens.
- Content is organized around live traffic, reliability, routing, clients, and response performance.
