/**
 * Release every held key.
 *
 * Called on completion, on Ctrl-C and on error, because a held throttle survives
 * the process that set it: the car keeps driving after the driver is gone.
 */
window.__drv.releaseAll();
