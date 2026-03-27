import { createRouter, createRoute, createRootRoute, RouterProvider } from '@tanstack/react-router';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MainLayout } from './components/layout/MainLayout';
import { DashboardPage } from './routes/index';
import { AgentsPage } from './routes/agents';
import { CanvasPage } from './routes/canvas';
import { ChatPage } from './routes/chat';
import { SessionsPage } from './routes/sessions';
import { ToolsPage } from './routes/tools';
import { SkillsPage } from './routes/skills';
import { MemoryPage } from './routes/memory';
import { ConfigPage } from './routes/config';
import { CronPage } from './routes/cron';
import { ChannelsPage } from './routes/channels';
import { DelegationPage } from './routes/delegation';
import { MonitoringPage } from './routes/monitoring';
import { VoicePage } from './routes/voice';
import { ErrorBoundary } from './components/ui/error-boundary';
import { ModeProvider } from './lib/mode-context.tsx';

const queryClient = new QueryClient();

const rootRoute = createRootRoute({
  component: MainLayout,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: DashboardPage,
});

const agentsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/agents',
  component: AgentsPage,
});

const chatRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/chat',
  component: ChatPage,
});

const sessionsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/sessions',
  component: SessionsPage,
});

const canvasRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/canvas',
  component: CanvasPage,
});

const toolsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/tools',
  component: ToolsPage,
});

const skillsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/skills',
  component: SkillsPage,
});

const memoryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/memory',
  component: MemoryPage,
});

const configRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/config',
  component: ConfigPage,
});

const cronRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/cron',
  component: CronPage,
});

const channelsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/channels',
  component: ChannelsPage,
});

const delegationRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/delegation',
  component: DelegationPage,
});

const monitoringRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/monitoring',
  component: MonitoringPage,
});

const voiceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/voice',
  component: VoicePage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  agentsRoute,
  canvasRoute,
  chatRoute,
  sessionsRoute,
  toolsRoute,
  skillsRoute,
  memoryRoute,
  configRoute,
  cronRoute,
  channelsRoute,
  delegationRoute,
  monitoringRoute,
  voiceRoute,
]);

const router = createRouter({ routeTree });

export default function App() {
  return (
    <ErrorBoundary>
      <ModeProvider>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </ModeProvider>
    </ErrorBoundary>
  );
}
