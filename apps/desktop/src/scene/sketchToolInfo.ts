// What each sketch tool is called, what it looks like, and how it is used.
//
// One table so the ribbon button, its tooltip and the status-bar prompt can
// never drift apart.

import type { IconName } from "../components/icons";
import type { SketchTool } from "./sketchTools";

export interface ToolInfo {
  label: string;
  icon: IconName;
  /** Tooltip on the ribbon button */
  hint: string;
  /** Shown in the status bar while the tool is armed */
  prompt: string;
}

export const TOOLS: Record<SketchTool, ToolInfo> = {
  select: {
    label: "Select",
    icon: "select",
    hint: "Pick entities — Shift adds to the selection",
    prompt: "Select: click an entity, Shift adds — then pick a constraint",
  },
  point: {
    label: "Point",
    icon: "point",
    hint: "Place a construction point",
    prompt: "Point: click to place one",
  },
  line: {
    label: "Line",
    icon: "line",
    hint: "Click point to point; close on the first point",
    prompt: "Line: click point to point, close on the first point to end the loop",
  },
  rect: {
    label: "Rectangle",
    icon: "rect",
    hint: "Click two opposite corners",
    prompt: "Rectangle: click two opposite corners",
  },
  rectCenter: {
    label: "Centre Rect",
    icon: "rectCenter",
    hint: "Click the centre, then a corner",
    prompt: "Centre rectangle: click the centre, then a corner",
  },
  rect3: {
    label: "3-Point Rect",
    icon: "rect3",
    hint: "Two clicks give one edge, the third the width",
    prompt: "3-point rectangle: click along one edge, then set the width",
  },
  circle: {
    label: "Circle",
    icon: "circle",
    hint: "Click the centre, then the radius",
    prompt: "Circle: click the centre, then a point on the radius",
  },
  circle2: {
    label: "2-Point Circle",
    icon: "circle2",
    hint: "Click two ends of a diameter",
    prompt: "2-point circle: click both ends of a diameter",
  },
  circle3: {
    label: "3-Point Circle",
    icon: "circle3",
    hint: "Click three points on the circle",
    prompt: "3-point circle: click three points it passes through",
  },
  arc3: {
    label: "3-Point Arc",
    icon: "arc",
    hint: "Click both ends, then a point on the arc",
    prompt: "3-point arc: click both ends, then bulge it through a third point",
  },
  arcCenter: {
    label: "Centre Arc",
    icon: "arcCenter",
    hint: "Click the centre, the start, then the end",
    prompt: "Centre arc: click the centre, the start, then sweep to the end",
  },
  polygon: {
    label: "Polygon",
    icon: "polygon",
    hint: "Inscribed in a circle — the click sets a corner",
    prompt: "Polygon: click the centre, then a corner",
  },
  polygonCirc: {
    label: "Circumscribed",
    icon: "polygonCirc",
    hint: "Around a circle — the click sets an edge",
    prompt: "Circumscribed polygon: click the centre, then the middle of an edge",
  },
  polygonEdge: {
    label: "Edge Polygon",
    icon: "polygon",
    hint: "Built from one edge you draw",
    prompt: "Edge polygon: click both ends of one edge",
  },
  ellipse: {
    label: "Ellipse",
    icon: "ellipse",
    hint: "Centre, then each axis in turn",
    prompt: "Ellipse: click the centre, the major axis, then the minor",
  },
  slot: {
    label: "Slot",
    icon: "slot",
    hint: "Centre to centre, then the width",
    prompt: "Slot: click both centres, then set the width",
  },
  slotOverall: {
    label: "Overall Slot",
    icon: "slot",
    hint: "End to end including the round caps, then the width",
    prompt: "Overall slot: click both ends, then set the width",
  },
  spline: {
    label: "Spline",
    icon: "spline",
    hint: "Click through points; Enter finishes, first point closes",
    prompt: "Spline: click through the points — Enter finishes, the first point closes it",
  },
  fillet: {
    label: "Fillet",
    icon: "fillet",
    hint: "Round the corner where two lines meet",
    prompt: "Fillet: click the corner between two lines",
  },
  trim: {
    label: "Trim",
    icon: "trim",
    hint: "Cut away the stretch under the pointer",
    prompt: "Trim: click the piece of curve to remove",
  },
  extend: {
    label: "Extend",
    icon: "extend",
    hint: "Run a line out to the next thing it meets",
    prompt: "Extend: click a line near the end you want lengthened",
  },
  offset: {
    label: "Offset",
    icon: "offset",
    hint: "Copy curves a fixed distance to one side",
    prompt: "Offset: click a curve on the side you want the copy",
  },
  mirror: {
    label: "Mirror",
    icon: "mirror",
    hint: "Reflect the selection across a line",
    prompt: "Mirror: select first, then click the line to reflect across",
  },
  patternRect: {
    label: "Rect Pattern",
    icon: "patternRect",
    hint: "Repeat the selection along a direction",
    prompt: "Rectangular pattern: select first, then click to set the direction",
  },
  patternCirc: {
    label: "Circ Pattern",
    icon: "patternCirc",
    hint: "Repeat the selection around a centre",
    prompt: "Circular pattern: select first, then click the centre",
  },
};
