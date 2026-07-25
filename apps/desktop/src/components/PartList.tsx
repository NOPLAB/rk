import type { PartInfo } from "../engine/api";

interface Props {
  parts: PartInfo[];
  bodyCount: number;
  selected: string | null;
  onSelect: (partId: string | null) => void;
}

export function PartList({ parts, bodyCount, selected, onSelect }: Props) {
  return (
    <div className="part-list">
      {parts.length === 0 && <div className="empty">No parts</div>}
      {parts.map((part) => (
        <div
          key={part.id}
          className={`part-row${part.id === selected ? " selected" : ""}`}
          onClick={() => onSelect(part.id === selected ? null : part.id)}
        >
          <span
            className="swatch"
            style={{
              background: `rgb(${part.color
                .slice(0, 3)
                .map((c) => Math.round(c * 255))
                .join(",")})`,
            }}
          />
          <span className="name">{part.name}</span>
        </div>
      ))}
      {bodyCount > 0 && (
        <div className="empty">
          {bodyCount} CAD {bodyCount === 1 ? "body" : "bodies"}
        </div>
      )}
    </div>
  );
}
