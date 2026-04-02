import { RootProvider } from 'fumadocs-ui/provider/next';
import type { ReactNode } from 'react';
import './global.css';

export const metadata = {
  title: {
    default: 'WeftOS',
    template: '%s | WeftOS',
  },
  description: 'The AI framework that remembers everything. Build agents with persistent memory, governance, and cryptographic provenance.',
  openGraph: {
    title: 'WeftOS',
    description: 'The AI framework that remembers everything. Build agents with persistent memory, governance, and cryptographic provenance.',
    siteName: 'WeftOS',
  },
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}
