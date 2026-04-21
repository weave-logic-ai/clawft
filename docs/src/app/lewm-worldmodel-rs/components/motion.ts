import { cubicBezier } from 'motion';

// Cubic-bezier for scroll-driven reveals. Single ease across the whole page.
// motion's useTransform options want an EasingFunction, not a bezier tuple,
// so we pre-compile via cubicBezier().
export const EASE = cubicBezier(0.16, 1, 0.3, 1);
