import type { ReactNode } from 'react';

export const metadata = {
  title: 'LeWM under the DAG',
  description:
    'JEPA as a sub-layer of CMVG — weftos × LeWorldModel — ADR-048 through ADR-058. A sensor-primary, world-model-optional architecture for robotics-grade ECC.',
  openGraph: {
    title: 'LeWM under the DAG · weftos',
    description:
      'JEPA as a sub-layer of CMVG — weftos × LeWorldModel — ADR-048 through ADR-058.',
    siteName: 'WeftOS',
  },
};

export default function LewmLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
