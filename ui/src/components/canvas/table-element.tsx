import type { CanvasElementData } from "../../stores/canvas-store";

interface TableElementProps {
  element: CanvasElementData;
}

export function TableElement({ element }: TableElementProps) {
  const headers = (element.headers as string[]) ?? [];
  const rows = (element.rows as string[][]) ?? [];

  return (
    <div className="overflow-auto rounded-md border border-gray-200 dark:border-gray-700">
      <table className="w-full text-left text-sm">
        <thead className="bg-gray-50 text-gray-700 dark:bg-gray-800 dark:text-gray-300">
          <tr>
            {headers.map((header, i) => (
              <th key={i} className="px-4 py-2 font-medium">
                {header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
          {rows.map((row, ri) => (
            <tr
              key={ri}
              className="text-gray-800 dark:text-gray-200"
            >
              {row.map((cell, ci) => (
                <td key={ci} className="px-4 py-2">
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
