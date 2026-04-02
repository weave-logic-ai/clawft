'use client';

import dynamic from 'next/dynamic';

const WasmSandbox = dynamic(() => import('./WasmSandbox'), { ssr: false });

export default function ClawftPage() {
  return <WasmSandbox />;
}
