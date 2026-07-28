/**
 * Compact number input that only reports a value when it actually changed.
 *
 * It is uncontrolled so typing is never fought by an engine round trip;
 * callers pass a `key` derived from the value to reset it when the engine
 * reports something new.
 */
export function MiniNum({
  value,
  step = 0.1,
  title,
  onCommit,
}: {
  value: number;
  step?: number;
  title?: string;
  onCommit: (v: number) => void;
}) {
  const rounded = Math.round(value * 1e4) / 1e4;
  return (
    <input
      className="mini-num"
      type="number"
      step={step}
      title={title}
      defaultValue={rounded}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
      }}
      onBlur={(e) => {
        const v = parseFloat(e.currentTarget.value);
        if (Number.isFinite(v) && Math.abs(v - rounded) > 1e-9) onCommit(v);
      }}
    />
  );
}
