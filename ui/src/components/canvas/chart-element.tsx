import { useMemo } from "react";
import type { CanvasElementData } from "../../stores/canvas-store";

interface ChartDataPoint {
  label: string;
  value: number;
}

const DEFAULT_COLORS = [
  "#6366f1",
  "#22c55e",
  "#f59e0b",
  "#ef4444",
  "#3b82f6",
  "#8b5cf6",
  "#ec4899",
  "#14b8a6",
];

interface ChartElementProps {
  element: CanvasElementData;
}

export function ChartElement({ element }: ChartElementProps) {
  const data = (element.data as ChartDataPoint[]) ?? [];
  const chartType = (element.chartType as string) ?? "bar";
  const title = (element.title as string) ?? "";
  const colors = (element.colors as string[]) ?? DEFAULT_COLORS;

  if (data.length === 0) {
    return (
      <div className="flex h-32 items-center justify-center text-sm text-gray-400 dark:text-gray-500">
        No chart data
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {title && (
        <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300">
          {title}
        </h4>
      )}
      {chartType === "bar" && <BarChart data={data} colors={colors} />}
      {chartType === "line" && <LineChart data={data} colors={colors} />}
      {chartType === "pie" && <PieChart data={data} colors={colors} />}
    </div>
  );
}

// ── Bar Chart (CSS-based) ─────────────────────────────────────

function BarChart({
  data,
  colors,
}: {
  data: ChartDataPoint[];
  colors: string[];
}) {
  const maxValue = useMemo(
    () => Math.max(...data.map((d) => d.value), 1),
    [data],
  );

  return (
    <div className="space-y-2">
      {data.map((point, i) => {
        const pct = (point.value / maxValue) * 100;
        const color = colors[i % colors.length];
        return (
          <div key={i} className="flex items-center gap-2">
            <span className="w-20 shrink-0 truncate text-xs text-gray-600 dark:text-gray-400">
              {point.label}
            </span>
            <div className="relative flex-1 h-5 rounded bg-gray-100 dark:bg-gray-800">
              <div
                className="h-full rounded transition-all duration-300"
                style={{ width: `${pct}%`, backgroundColor: color }}
              />
            </div>
            <span className="w-12 shrink-0 text-right text-xs font-mono text-gray-600 dark:text-gray-400">
              {point.value}
            </span>
          </div>
        );
      })}
    </div>
  );
}

// ── Line Chart (SVG-based) ────────────────────────────────────

function LineChart({
  data,
  colors,
}: {
  data: ChartDataPoint[];
  colors: string[];
}) {
  const width = 400;
  const height = 200;
  const padding = { top: 20, right: 20, bottom: 40, left: 50 };
  const innerW = width - padding.left - padding.right;
  const innerH = height - padding.top - padding.bottom;

  const values = data.map((d) => d.value);
  const maxVal = Math.max(...values, 1);
  const minVal = Math.min(...values, 0);
  const range = maxVal - minVal || 1;

  const points = data.map((d, i) => ({
    x: padding.left + (i / Math.max(data.length - 1, 1)) * innerW,
    y: padding.top + innerH - ((d.value - minVal) / range) * innerH,
  }));

  const pathD = points
    .map((p, i) => `${i === 0 ? "M" : "L"} ${p.x} ${p.y}`)
    .join(" ");

  const color = colors[0];

  // Y-axis ticks
  const yTicks = useMemo(() => {
    const ticks: number[] = [];
    const step = range / 4;
    for (let i = 0; i <= 4; i++) {
      ticks.push(minVal + step * i);
    }
    return ticks;
  }, [minVal, range]);

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className="w-full max-w-md"
      preserveAspectRatio="xMidYMid meet"
    >
      {/* Grid lines */}
      {yTicks.map((tick, i) => {
        const y = padding.top + innerH - ((tick - minVal) / range) * innerH;
        return (
          <g key={i}>
            <line
              x1={padding.left}
              y1={y}
              x2={padding.left + innerW}
              y2={y}
              stroke="currentColor"
              strokeOpacity={0.1}
              strokeDasharray="4 4"
            />
            <text
              x={padding.left - 8}
              y={y + 3}
              textAnchor="end"
              className="fill-gray-400 dark:fill-gray-500"
              fontSize={10}
            >
              {Math.round(tick)}
            </text>
          </g>
        );
      })}

      {/* Line path */}
      <path d={pathD} fill="none" stroke={color} strokeWidth={2} />

      {/* Data points */}
      {points.map((p, i) => (
        <circle key={i} cx={p.x} cy={p.y} r={3} fill={color} />
      ))}

      {/* X-axis labels */}
      {data.map((d, i) => (
        <text
          key={i}
          x={points[i].x}
          y={height - 8}
          textAnchor="middle"
          className="fill-gray-400 dark:fill-gray-500"
          fontSize={10}
        >
          {d.label.length > 8 ? d.label.slice(0, 7) + "..." : d.label}
        </text>
      ))}
    </svg>
  );
}

// ── Pie Chart (SVG-based) ─────────────────────────────────────

function PieChart({
  data,
  colors,
}: {
  data: ChartDataPoint[];
  colors: string[];
}) {
  const size = 200;
  const cx = size / 2;
  const cy = size / 2;
  const radius = size / 2 - 20;

  const total = useMemo(
    () => data.reduce((sum, d) => sum + d.value, 0) || 1,
    [data],
  );

  const slices = useMemo(() => {
    let startAngle = -Math.PI / 2;
    return data.map((d, i) => {
      const sweepAngle = (d.value / total) * Math.PI * 2;
      const endAngle = startAngle + sweepAngle;

      const x1 = cx + radius * Math.cos(startAngle);
      const y1 = cy + radius * Math.sin(startAngle);
      const x2 = cx + radius * Math.cos(endAngle);
      const y2 = cy + radius * Math.sin(endAngle);

      const largeArc = sweepAngle > Math.PI ? 1 : 0;

      const pathD = [
        `M ${cx} ${cy}`,
        `L ${x1} ${y1}`,
        `A ${radius} ${radius} 0 ${largeArc} 1 ${x2} ${y2}`,
        "Z",
      ].join(" ");

      const result = {
        path: pathD,
        color: colors[i % colors.length],
        label: d.label,
        value: d.value,
        midAngle: startAngle + sweepAngle / 2,
      };

      startAngle = endAngle;
      return result;
    });
  }, [data, total, cx, cy, radius, colors]);

  return (
    <div className="flex items-center gap-4">
      <svg
        viewBox={`0 0 ${size} ${size}`}
        className="w-48 h-48 shrink-0"
      >
        {slices.map((slice, i) => (
          <path
            key={i}
            d={slice.path}
            fill={slice.color}
            stroke="white"
            strokeWidth={1}
            className="dark:stroke-gray-900"
          />
        ))}
      </svg>
      <div className="space-y-1 text-xs">
        {slices.map((slice, i) => (
          <div key={i} className="flex items-center gap-2">
            <span
              className="inline-block h-3 w-3 rounded-sm shrink-0"
              style={{ backgroundColor: slice.color }}
            />
            <span className="text-gray-600 dark:text-gray-400">
              {slice.label}
            </span>
            <span className="font-mono text-gray-500 dark:text-gray-500">
              {slice.value}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
