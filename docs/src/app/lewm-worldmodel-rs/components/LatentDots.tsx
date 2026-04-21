export default function LatentDots() {
  const dots = Array.from({ length: 192 }, (_, i) => i);
  return (
    <div className="lewm-dots" aria-hidden="true">
      {dots.map((i) => (
        <span key={i} style={{ animationDelay: `${(i % 16) * 0.05}s` }} />
      ))}
    </div>
  );
}
