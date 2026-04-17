'use client';

import { useState } from 'react';
import { SliceNavigator } from './SliceNavigator';
import { VowlNavigator } from './VowlNavigator';
import { sampleOntology } from './sample-data';
import type { VowlJson } from './types';

type ViewMode = 'slices' | 'vowl';

export default function VowlNavigatorPage() {
  const [mode, setMode] = useState<ViewMode>('slices');
  const [vowlData, setVowlData] = useState<VowlJson>(sampleOntology);
  const [jsonError, setJsonError] = useState<string | null>(null);

  function handleFileUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const parsed = JSON.parse(reader.result as string) as VowlJson;
        if (!parsed.class || !parsed.classAttribute || !parsed.property || !parsed.propertyAttribute) {
          setJsonError('Invalid VOWL JSON: missing required keys');
          return;
        }
        setVowlData(parsed);
        setMode('vowl');
        setJsonError(null);
      } catch (err) {
        setJsonError(`Parse error: ${err instanceof Error ? err.message : String(err)}`);
      }
    };
    reader.readAsText(file);
  }

  return (
    <div className="min-h-screen bg-neutral-50 dark:bg-neutral-950 p-6">
      <div className="max-w-[1400px] mx-auto">
        <div className="mb-4">
          <h1 className="text-2xl font-bold text-neutral-900 dark:text-neutral-100 mb-1">
            Topology Navigator
          </h1>
          <p className="text-sm text-neutral-500 mb-3">
            Explore the WeftOS codebase as a navigable topology. Start at the package level,
            double-click to drill into modules and functions.
          </p>
          <div className="flex items-center gap-3 text-sm">
            <button
              onClick={() => setMode('slices')}
              className={`px-3 py-1 rounded ${mode === 'slices' ? 'bg-indigo-600 text-white' : 'bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300'}`}
            >
              Drill-down
            </button>
            <button
              onClick={() => setMode('vowl')}
              className={`px-3 py-1 rounded ${mode === 'vowl' ? 'bg-indigo-600 text-white' : 'bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300'}`}
            >
              VOWL flat
            </button>
            <label className="text-neutral-500">
              Upload:
              <input type="file" accept=".json" onChange={handleFileUpload} className="ml-1 text-xs" />
            </label>
          </div>
          {jsonError && (
            <p className="mt-2 text-sm text-red-600 dark:text-red-400">{jsonError}</p>
          )}
        </div>

        {mode === 'slices' ? (
          <SliceNavigator width={1100} height={650} />
        ) : (
          <VowlNavigator data={vowlData} width={1100} height={650} />
        )}
      </div>
    </div>
  );
}
