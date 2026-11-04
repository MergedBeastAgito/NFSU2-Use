/**
 * Show the decision on the page's overlay.
 *
 * The payload is spliced as code because it is a JSON literal the driver already
 * serialised; the overlay is the reader, never the controller, so a failure here
 * must not stop the car - it returns 0 and the loop carries on.
 */
declare const __PARAM_Payload__: object;
window.__drv.publish(__PARAM_Payload__);
