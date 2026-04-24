/**
 * Security Headers Middleware for AgentDB
 *
 * Implements comprehensive security headers using Helmet.js:
 * - Content Security Policy (CSP)
 * - HTTP Strict Transport Security (HSTS)
 * - X-Frame-Options
 * - X-Content-Type-Options
 * - X-XSS-Protection
 * - Referrer-Policy
 * - Permissions-Policy
 *
 * Security Features:
 * - Prevents XSS attacks
 * - Prevents clickjacking
 * - Enforces HTTPS
 * - Prevents MIME sniffing
 * - Controls browser features
 */

import helmet from 'helmet';
import { Request, Response, NextFunction } from 'express';

/**
 * Content Security Policy configuration
 */
const cspConfig = {
  directives: {
    defaultSrc: ["'self'"],
    scriptSrc: ["'self'", "'unsafe-inline'"], // Remove unsafe-inline in production
    styleSrc: ["'self'", "'unsafe-inline'"],
    imgSrc: ["'self'", 'data:', 'https:'],
    fontSrc: ["'self'"],
    connectSrc: ["'self'"],
    frameSrc: ["'none'"],
    objectSrc: ["'none'"],
    mediaSrc: ["'self'"],
    manifestSrc: ["'self'"],
    workerSrc: ["'self'"],
    formAction: ["'self'"],
    frameAncestors: ["'none'"],
    baseUri: ["'self'"],
    upgradeInsecureRequests: [],
  },
};

/**
 * Helmet configuration
 */
const helmetConfig = {
  // Content Security Policy
  contentSecurityPolicy: cspConfig,

  // DNS Prefetch Control
  dnsPrefetchControl: {
    allow: false,
  },

  // Frameguard (X-Frame-Options)
  frameguard: {
    action: 'deny' as const,
  },

  // Hide Powered-By header
  hidePoweredBy: true,

  // HTTP Strict Transport Security
  hsts: {
    maxAge: 31536000, // 1 year
    includeSubDomains: true,
    preload: true,
  },

  // Don't Sniff MIME Type
  noSniff: true,

  // Permissions Policy (Feature Policy)
  permittedCrossDomainPolicies: {
    permittedPolicies: 'none' as const,
  },

  // Referrer Policy
  referrerPolicy: {
    policy: 'strict-origin-when-cross-origin' as const,
  },
};

/**
 * Apply all security headers
 */
export const securityHeaders = helmet(helmetConfig);

/**
 * CORS configuration middleware
 */
export function configureCORS(options: {
  allowedOrigins?: string[];
  allowCredentials?: boolean;
  maxAge?: number;
} = {}) {
  const {
    allowedOrigins = ['http://localhost:3000'],
    allowCredentials = true,
    maxAge = 86400, // 24 hours
  } = options;

  return (req: Request, res: Response, next: NextFunction): void => {
    const origin = req.headers.origin;

    // Check if origin is allowed
    if (origin && allowedOrigins.includes(origin)) {
      res.setHeader('Access-Control-Allow-Origin', origin);
    } else if (allowedOrigins.includes('*')) {
      res.setHeader('Access-Control-Allow-Origin', '*');
    }

    // Allow credentials
    if (allowCredentials) {
      res.setHeader('Access-Control-Allow-Credentials', 'true');
    }

    // Allow specific methods
    res.setHeader(
      'Access-Control-Allow-Methods',
      'GET, POST, PUT, DELETE, PATCH, OPTIONS'
    );

    // Allow specific headers
    res.setHeader(
      'Access-Control-Allow-Headers',
      'Content-Type, Authorization, X-API-Key, X-Requested-With'
    );

    // Expose specific headers
    res.setHeader(
      'Access-Control-Expose-Headers',
      'X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset'
    );

    // Max age for preflight cache
    res.setHeader('Access-Control-Max-Age', maxAge.toString());

    // Handle preflight requests
    if (req.method === 'OPTIONS') {
      res.status(204).send();
      return;
    }

    next();
  };
}

/**
 * Add custom security headers
 */
export function customSecurityHeaders(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  // Permissions Policy (Feature Policy)
  res.setHeader(
    'Permissions-Policy',
    [
      'camera=()',
      'microphone=()',
      'geolocation=()',
      'payment=()',
      'usb=()',
      'magnetometer=()',
      'gyroscope=()',
      'accelerometer=()',
    ].join(', ')
  );

  // Expect-CT header (Certificate Transparency)
  res.setHeader(
    'Expect-CT',
    'max-age=86400, enforce'
  );

  // X-Permitted-Cross-Domain-Policies
  res.setHeader('X-Permitted-Cross-Domain-Policies', 'none');

  // Cache-Control for sensitive endpoints
  if (req.path.includes('/auth') || req.path.includes('/api')) {
    res.setHeader('Cache-Control', 'no-store, no-cache, must-revalidate, private');
    res.setHeader('Pragma', 'no-cache');
    res.setHeader('Expires', '0');
  }

  next();
}

/**
 * Remove sensitive headers
 */
export function removeSensitiveHeaders(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  // Remove headers that might leak server information
  res.removeHeader('X-Powered-By');
  res.removeHeader('Server');

  next();
}

/**
 * HSTS preload header (for production)
 */
export function hstsPreload(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  if (process.env.NODE_ENV === 'production') {
    res.setHeader(
      'Strict-Transport-Security',
      'max-age=31536000; includeSubDomains; preload'
    );
  }

  next();
}

/**
 * Security headers for API responses
 */
export function apiSecurityHeaders(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  // JSON responses only
  res.setHeader('Content-Type', 'application/json; charset=utf-8');

  // No caching for API responses
  res.setHeader('Cache-Control', 'no-store, no-cache, must-revalidate, private');
  res.setHeader('Pragma', 'no-cache');
  res.setHeader('Expires', '0');

  // Prevent MIME sniffing
  res.setHeader('X-Content-Type-Options', 'nosniff');

  next();
}

/**
 * Combined security middleware
 */
export function applySecurity(options: {
  cors?: boolean;
  corsOrigins?: string[];
  hsts?: boolean;
  csp?: boolean;
} = {}) {
  const {
    cors = true,
    corsOrigins = ['http://localhost:3000'],
    hsts = true,
    csp = true,
  } = options;

  const middlewares: Array<(req: Request, res: Response, next: NextFunction) => void> = [];

  // Remove sensitive headers
  middlewares.push(removeSensitiveHeaders);

  // Apply Helmet security headers
  if (csp) {
    middlewares.push(securityHeaders);
  }

  // Apply custom security headers
  middlewares.push(customSecurityHeaders);

  // Apply HSTS preload
  if (hsts) {
    middlewares.push(hstsPreload);
  }

  // Apply CORS
  if (cors) {
    middlewares.push(configureCORS({ allowedOrigins: corsOrigins }));
  }

  return (req: Request, res: Response, next: NextFunction): void => {
    let index = 0;

    const runNext = (): void => {
      if (index >= middlewares.length) {
        next();
        return;
      }

      const middleware = middlewares[index++];
      middleware(req, res, runNext);
    };

    runNext();
  };
}

/**
 * Development-only CORS (allow all)
 */
export function developmentCORS(
  req: Request,
  res: Response,
  next: NextFunction
): void {
  if (process.env.NODE_ENV === 'development') {
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', '*');
    res.setHeader('Access-Control-Allow-Headers', '*');

    if (req.method === 'OPTIONS') {
      res.status(204).send();
      return;
    }
  }

  next();
}
