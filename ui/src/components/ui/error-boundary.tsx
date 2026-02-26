import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "./button";

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("[ErrorBoundary] Caught rendering error:", error, info.componentStack);
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="flex flex-1 flex-col items-center justify-center gap-4 p-8">
          <div className="rounded-lg border border-red-200 bg-red-50 p-6 text-center dark:border-red-800 dark:bg-red-900/20">
            <h2 className="mb-2 text-lg font-semibold text-red-800 dark:text-red-300">
              Something went wrong
            </h2>
            <p className="mb-4 text-sm text-red-600 dark:text-red-400">
              An unexpected error occurred while rendering this page.
            </p>
            {this.state.error && (
              <pre className="mb-4 max-w-md overflow-auto rounded bg-red-100 p-3 text-left text-xs text-red-700 dark:bg-red-900/40 dark:text-red-300">
                {this.state.error.message}
              </pre>
            )}
            <Button variant="outline" size="sm" onClick={this.handleRetry}>
              Retry
            </Button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
