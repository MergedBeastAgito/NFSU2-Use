/**
 * Set the cruise goal: a speed to hold and a steering bias to lean on.
 *
 * This is the fast loop's only input. The driver writes it once a tick; the
 * page applies it every 16 ms, so the keys follow the goal at frame rate rather
 * than at the tick rate - a slow model, or a slow round trip, never makes the
 * car stutter. `On = false` stops the loop and releases every key.
 *
 * `Kmh` is the speed to hold and `Bias` is -1 (hard left) to +1 (hard right):
 * the steering is a duty cycle of a 110 ms window, so a small bias is a short
 * pulse rather than a held key.
 */
declare const __PARAM_On__: boolean;
declare const __PARAM_Kmh__: number;
declare const __PARAM_Bias__: number;
window.__drv.cruise(__PARAM_On__, __PARAM_Kmh__, __PARAM_Bias__);
