/**
 * Release every held key, and stop the fast loop that would hold them again.
 *
 * Called on completion, on Ctrl-C and on error, because a held throttle - or a
 * cruise goal still being applied sixty times a second - survives the process
 * that set it: the car keeps driving after the driver is gone.
 */
window.__drv.cruise(false, 0, 0);
window.__drv.releaseAll();
