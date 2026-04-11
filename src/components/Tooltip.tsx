export function Tooltip({
  text,
  x,
  y,
}: {
  text: string;
  x: number;
  y: number;
}) {
  return (
    <div
      className="app-tooltip"
      style={{
        left: x,
        top: y - 10,
      }}
    >
      {text}
    </div>
  );
}
