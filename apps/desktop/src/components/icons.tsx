// Inline SVG icon set.
//
// The artifact ships as a single self-contained window, so icons are drawn
// here rather than pulled from a font or sprite sheet. Each glyph is a base
// outline in the current text colour plus an optional accent stroke — the
// two-tone look Inventor's ribbon uses to make a command readable at 16px.

interface Glyph {
  base: string;
  accent?: string;
}

const ICONS = {
  // ---- file / history ---------------------------------------------------
  new: {
    base: "M6.5 2.5h7.2l4.3 4.3v14.7h-11.5z",
    accent: "M13.7 2.5v4.3h4.3",
  },
  open: {
    base: "M2.5 6.5h6.5l1.8 2.3h10.7v10.7h-19z",
    accent: "M5.5 12.5h18l-3 7",
  },
  save: {
    base: "M3.5 3.5h13.5l3.5 3.5v13.5h-17z",
    accent: "M7.5 3.5v6h8v-6M7.5 20.5v-7h9v7",
  },
  saveAs: {
    base: "M3.5 3.5h11.5l3 3v8h-14.5z",
    accent: "M13 21l8-8-2.5-2.5-8 8-.6 3z",
  },
  importMesh: {
    base: "M12 3v11M12 14l-3.5-3.5M12 14l3.5-3.5",
    accent: "M3.5 15.5v5h17v-5",
  },
  importUrdf: {
    base: "M4.5 4.5h7v7h-7zM12.5 12.5h7v7h-7z",
    accent: "M8 11.5v5h4.5",
  },
  exportUrdf: {
    base: "M12 14V3M12 3L8.5 6.5M12 3l3.5 3.5",
    accent: "M3.5 15.5v5h17v-5",
  },
  undo: {
    base: "M4.5 10.5h10a5 5 0 0 1 0 10h-6M8.5 5.5l-4.5 5 4.5 5",
  },
  redo: {
    base: "M19.5 10.5h-10a5 5 0 0 0 0 10h6M15.5 5.5l4.5 5-4.5 5",
  },

  // ---- primitives -------------------------------------------------------
  box: {
    base: "M12 2.5l8.5 5v9l-8.5 5-8.5-5v-9z",
    accent: "M3.5 7.5l8.5 5 8.5-5M12 12.5v9",
  },
  cylinder: {
    base: "M5.5 6.5c0-2.2 2.9-3.5 6.5-3.5s6.5 1.3 6.5 3.5v11c0 2.2-2.9 3.5-6.5 3.5s-6.5-1.3-6.5-3.5z",
    accent: "M5.5 6.5c0 2.2 2.9 3.5 6.5 3.5s6.5-1.3 6.5-3.5",
  },
  sphere: {
    base: "M21.5 12a9.5 9.5 0 1 1-19 0 9.5 9.5 0 0 1 19 0z",
    accent: "M2.5 12h19M12 2.5c3.2 2.5 4.8 5.8 4.8 9.5s-1.6 7-4.8 9.5",
  },
  trash: {
    base: "M5.5 6.5h13l-1.2 14h-10.6z",
    accent: "M9.5 6.5v-3h5v3M3.5 6.5h17",
  },

  // ---- sketch -----------------------------------------------------------
  sketch: {
    base: "M1.5 13.5l8-4 9 3.5-8 4z",
    accent: "M13 16l6.5-6.5 3 3-6.5 6.5-4 1z",
  },
  line: {
    base: "M4.5 19.5l15-15",
    accent:
      "M4.5 17.7a1.8 1.8 0 1 0 0 3.6 1.8 1.8 0 0 0 0-3.6zM19.5 2.7a1.8 1.8 0 1 0 0 3.6 1.8 1.8 0 0 0 0-3.6z",
  },
  rect: {
    base: "M4.5 6.5h15v11h-15z",
    accent:
      "M4.5 4.9a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2zM19.5 15.9a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z",
  },
  circle: {
    base: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z",
    accent: "M12 12l6.36-6.36M12 10.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z",
  },
  rectCenter: {
    base: "M4.5 6.5h15v11h-15z",
    accent: "M12 9.6a2.4 2.4 0 1 0 0 4.8 2.4 2.4 0 0 0 0-4.8zM12 4v3M12 17v3",
  },
  rect3: {
    base: "M3 9.8l7.5-5.3 10.5 6.4-7.5 5.3z",
    accent:
      "M3 8.2a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2zM10.5 2.9a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z",
  },
  circle2: {
    base: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z",
    accent:
      "M3 10.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2zM21 10.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z",
  },
  circle3: {
    base: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z",
    accent:
      "M12 1.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2zM4.2 17.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2zM19.8 17.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z",
  },
  arc: {
    base: "M3.5 18.5a9 9 0 0 1 17 0",
    accent:
      "M3.5 16.9a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2zM20.5 16.9a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z",
  },
  arcCenter: {
    base: "M4.5 19a9 9 0 0 1 15-6.7",
    accent: "M4.5 19h15M12.5 17.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z",
  },
  polygon: {
    base: "M12 3l7.8 4.5v9L12 21l-7.8-4.5v-9z",
  },
  polygonCirc: {
    base: "M12 4.6l6.4 3.7v7.4L12 19.4l-6.4-3.7V8.3z",
    accent: "M18.4 12a6.4 6.4 0 1 1-12.8 0 6.4 6.4 0 0 1 12.8 0z",
  },
  ellipse: {
    base: "M22 12c0 3.6-4.5 6.5-10 6.5S2 15.6 2 12s4.5-6.5 10-6.5S22 8.4 22 12z",
    accent: "M2 12h20",
  },
  slot: {
    base: "M8 6.5h8a5.5 5.5 0 0 1 0 11H8a5.5 5.5 0 0 1 0-11z",
    accent: "M8 12h8",
  },
  spline: {
    base: "M3 18c4-1 4-12 9-12s5 11 9 10",
    accent:
      "M3 16.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2zM12 4.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2zM21 14.4a1.6 1.6 0 1 0 0 3.2 1.6 1.6 0 0 0 0-3.2z",
  },
  point: {
    base: "M12 4v16M4 12h16",
    accent: "M12 9.6a2.4 2.4 0 1 0 0 4.8 2.4 2.4 0 0 0 0-4.8z",
  },
  fillet: {
    base: "M4.5 4.5v9a6 6 0 0 0 6 6h9",
    accent: "M4.5 4.5h9M19.5 19.5v-9",
  },
  trim: {
    base: "M5 4l9.5 12M19 4l-5 6.3",
    accent:
      "M6.6 17.8a2.6 2.6 0 1 0 0 5.2 2.6 2.6 0 0 0 0-5.2zM17.4 17.8a2.6 2.6 0 1 0 0 5.2 2.6 2.6 0 0 0 0-5.2z",
  },
  extend: {
    base: "M3 12h13",
    accent: "M11 7l5 5-5 5M20 4v16",
  },
  offset: {
    base: "M6 5h12v14H6z",
    accent: "M2.5 2h19v20h-19z",
  },
  mirror: {
    base: "M12 2v20",
    accent: "M9.5 6L4 12l5.5 6zM14.5 6l5.5 6-5.5 6z",
  },
  patternRect: {
    base: "M4 4h5v5H4zM15 4h5v5h-5zM4 15h5v5H4z",
    accent: "M15 15h5v5h-5z",
  },
  patternCirc: {
    base: "M12 2.5a2.5 2.5 0 1 1 0 5 2.5 2.5 0 0 1 0-5zM19 16a2.5 2.5 0 1 1 0 5 2.5 2.5 0 0 1 0-5zM5 16a2.5 2.5 0 1 1 0 5 2.5 2.5 0 0 1 0-5z",
    accent: "M20 12a8 8 0 1 1-16 0 8 8 0 0 1 16 0z",
  },
  construction: {
    base: "M3 19h4M10 19h4M17 19h4",
    accent: "M4 15L20 5",
  },
  region: {
    base: "M4 5h16v14H4z",
    accent: "M8 9h8v6H8z",
  },
  solve: {
    base: "M12 3.5l2.6 5.6 6.1.8-4.5 4.2 1.2 6.1-5.4-3-5.4 3 1.2-6.1-4.5-4.2 6.1-.8z",
  },
  align: {
    base: "M3.5 12h17M12 3.5v17",
    accent: "M16.5 12a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0z",
  },
  finish: {
    base: "M4 12.5l5 5 11-11",
  },

  // ---- features ---------------------------------------------------------
  extrude: {
    base: "M3.5 16.5l5.5-3.2 8 3.2-5.5 3.2z",
    accent: "M12 9.5v-6M12 3.5l-2.6 2.8M12 3.5l2.6 2.8",
  },
  revolve: {
    base: "M6.5 2.5v19M9.5 6.5c4 1.5 6 3.4 6 5.5s-2 4-6 5.5",
    accent: "M9.5 6.5l3.4-.5M9.5 6.5l.5 3.4",
  },
  feature: {
    base: "M12 4l7.5 4.2v7.6L12 20l-7.5-4.2V8.2z",
    accent: "M4.5 8.2l7.5 4.2 7.5-4.2",
  },
  rollback: {
    base: "M2.5 12h19",
    accent: "M12 6.5v11",
  },
  suppress: {
    base: "M20 12a8 8 0 1 1-16 0 8 8 0 0 1 16 0z",
    accent: "M6.4 6.4l11.2 11.2",
  },

  // ---- assembly ---------------------------------------------------------
  connect: {
    base: "M2.5 8.5h6v7h-6zM15.5 8.5h6v7h-6z",
    accent: "M8.5 12h7M12 9.2v5.6",
  },
  disconnect: {
    base: "M2.5 8.5h6v7h-6zM15.5 8.5h6v7h-6z",
    accent: "M8.5 12h2.2M13.3 12h2.2M10.4 15.5l3.2-7",
  },
  joint: {
    base: "M17 12a5 5 0 1 1-10 0 5 5 0 0 1 10 0z",
    accent: "M12 2.5v4.5M12 17v4.5M12 10.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z",
  },
  collision: {
    base: "M12 3l8.5 4.8v9.4L12 21l-8.5-4.8V7.8z",
    accent: "M12 7.4l4.6 2.6v5.2L12 17.8l-4.6-2.6V10z",
  },
  reset: {
    base: "M20 12a8 8 0 1 1-2.9-6.2",
    accent: "M12.6 4.2l4.6 1.4-1.4 4.6",
  },

  // ---- view / navigation ------------------------------------------------
  select: {
    base: "M6 3.5l12.5 8-5.4 1.2 3 6-2.6 1.2-3-6-4.5 3.2z",
  },
  move: {
    base: "M12 3v18M3 12h18",
    accent:
      "M12 3l-2.5 2.5M12 3l2.5 2.5M12 21l-2.5-2.5M12 21l2.5-2.5M3 12l2.5-2.5M3 12l2.5 2.5M21 12l-2.5-2.5M21 12l2.5 2.5",
  },
  rotate: {
    base: "M20 12a8 8 0 1 1-2.9-6.2",
    accent: "M12.6 4.2l4.6 1.4-1.4 4.6",
  },
  home: {
    base: "M3.5 11.5l8.5-7.5 8.5 7.5",
    accent: "M6 10.5v9.5h12v-9.5",
  },
  fit: {
    base: "M3.5 8.5v-5h5M20.5 8.5v-5h-5M3.5 15.5v5h5M20.5 15.5v5h-5",
    accent: "M8.5 8.5h7v7h-7z",
  },
  viewFront: {
    base: "M12 3.5l8 4.5v8l-8 4.5-8-4.5v-8zM4 8l8 4.5 8-4.5M12 12.5v8",
    accent: "M4 8v8l8 4.5v-8z",
  },
  viewTop: {
    base: "M12 3.5l8 4.5v8l-8 4.5-8-4.5v-8zM4 8l8 4.5 8-4.5M12 12.5v8",
    accent: "M4 8l8-4.5 8 4.5-8 4.5z",
  },
  viewRight: {
    base: "M12 3.5l8 4.5v8l-8 4.5-8-4.5v-8zM4 8l8 4.5 8-4.5M12 12.5v8",
    accent: "M20 8v8l-8 4.5v-8z",
  },
  viewIso: {
    base: "M12 3.5l8 4.5v8l-8 4.5-8-4.5v-8z",
    accent: "M4 8l8 4.5 8-4.5M12 12.5v8",
  },
  grid: {
    base: "M3.5 3.5h17v17h-17z",
    accent: "M9.2 3.5v17M14.8 3.5v17M3.5 9.2h17M3.5 14.8h17",
  },
  eye: {
    base: "M2 12s3.6-6 10-6 10 6 10 6-3.6 6-10 6-10-6-10-6z",
    accent: "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
  },
  browserPanel: {
    base: "M3.5 4.5h17v15h-17z",
    accent: "M9.5 4.5v15",
  },
  inspectorPanel: {
    base: "M3.5 4.5h17v15h-17z",
    accent: "M14.5 4.5v15",
  },

  // ---- browser tree -----------------------------------------------------
  folder: {
    base: "M2.5 6h6.5l1.8 2.3h10.7v11.7h-19z",
  },
  part: {
    base: "M12 4l7.5 4.2v7.6L12 20l-7.5-4.2V8.2z",
    accent: "M4.5 8.2l7.5 4.2 7.5-4.2",
  },
  plane: {
    base: "M2.5 16.5l7.5-7h11.5l-7.5 7z",
    accent: "M14 9.5v-5M14 4.5l-1.8 2M14 4.5l1.8 2",
  },
  link: {
    base: "M9.5 14.5l5-5",
    accent:
      "M7 12l-2 2a3.5 3.5 0 0 0 5 5l2-2M17 12l2-2a3.5 3.5 0 0 0-5-5l-2 2",
  },
  endOfPart: {
    base: "M2.5 12h19",
    accent: "M12 7.5v9",
  },
  chevron: {
    base: "M9 5.5l6.5 6.5-6.5 6.5",
  },

  // ---- sketch constraints ----------------------------------------------
  cnCoincident: {
    base: "M5.5 18.5l13-13",
    accent:
      "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 10.8a1.2 1.2 0 1 0 0 2.4 1.2 1.2 0 0 0 0-2.4z",
  },
  cnHorizontal: {
    base: "M3.5 12h17",
    accent: "M6 8.5v7M18 8.5v7",
  },
  cnVertical: {
    base: "M12 3.5v17",
    accent: "M8.5 6h7M8.5 18h7",
  },
  cnParallel: {
    base: "M7 20.5l4-17",
    accent: "M14 20.5l4-17",
  },
  cnPerpendicular: {
    base: "M6 4v14h13",
    accent: "M6 14h4v4",
  },
  cnEqual: {
    base: "M3 6.5h8M3 17.5h8",
    accent: "M14 9.5h7M14 14.5h7",
  },
  cnEqualRadius: {
    base: "M9.5 8.5a3.5 3.5 0 1 1-7 0 3.5 3.5 0 0 1 7 0zM21.5 15.5a3.5 3.5 0 1 1-7 0 3.5 3.5 0 0 1 7 0z",
    accent: "M6 8.5l2.5-2.5M18 15.5l2.5-2.5",
  },
  cnTangent: {
    base: "M18 11a6 6 0 1 1-12 0 6 6 0 0 1 12 0z",
    accent: "M2.5 17.5h19",
  },
  cnOnCurve: {
    base: "M3 17c5-11 13-11 18 0",
    accent: "M12 9.9a1.8 1.8 0 1 0 0 3.6 1.8 1.8 0 0 0 0-3.6z",
  },
  cnMidpoint: {
    base: "M3.5 12h17",
    accent: "M12 8v8M3.5 9.5v5M20.5 9.5v5",
  },
  cnFix: {
    base: "M12 3.5a2.2 2.2 0 1 1 0 4.4 2.2 2.2 0 0 1 0-4.4zM12 7.9v12.6",
    accent: "M6 13c0 4.1 2.7 7.5 6 7.5s6-3.4 6-7.5",
  },
  cnDimension: {
    base: "M4 6v12M20 6v12",
    accent: "M4 12h16M7 9l-3 3 3 3M17 9l3 3-3 3",
  },
  cnDimVertical: {
    base: "M6 4h12M6 20h12",
    accent: "M12 4v16M9 7l3-3 3 3M9 17l3 3 3-3",
  },
  cnRadius: {
    base: "M20 12a8 8 0 1 1-16 0 8 8 0 0 1 16 0z",
    accent: "M12 12l5.6-5.6M12 10.8a1.2 1.2 0 1 0 0 2.4 1.2 1.2 0 0 0 0-2.4z",
  },
  cnDiameter: {
    base: "M20 12a8 8 0 1 1-16 0 8 8 0 0 1 16 0z",
    accent: "M6.3 17.7l11.4-11.4M6.3 14.5v3.2h3.2M17.7 9.5V6.3h-3.2",
  },
  cnAngle: {
    base: "M4 20h16M4 20L16 5.5",
    accent: "M14.5 20a10.5 10.5 0 0 0-2.6-6.9",
  },

  // ---- chrome -----------------------------------------------------------
  search: {
    base: "M16 10.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0zM14.5 14.5l5 5",
  },
  menu: {
    base: "M3.5 7h17M3.5 12h17M3.5 17h17",
  },
  close: {
    base: "M6 6l12 12M18 6L6 18",
  },
  plus: {
    base: "M12 4.5v15M4.5 12h15",
  },
} satisfies Record<string, Glyph>;

export type IconName = keyof typeof ICONS;

export function Icon({
  name,
  size = 16,
}: {
  name: IconName;
  size?: number;
}) {
  const glyph: Glyph = ICONS[name];
  return (
    <svg
      className="icon"
      viewBox="0 0 24 24"
      width={size}
      height={size}
      aria-hidden="true"
      focusable="false"
    >
      <path className="ic-base" d={glyph.base} />
      {glyph.accent && <path className="ic-accent" d={glyph.accent} />}
    </svg>
  );
}
