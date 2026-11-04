/**
 * Hold or release one key.
 *
 * `Key` is a DOM code the game already knows (`ArrowUp`, `ArrowLeft`, ...) and
 * `On` is spliced as code, not text, because it is a boolean and quoting it
 * would send the string "true" to a function that wants a truth value.
 */
declare const __PARAM_Key__: string;
declare const __PARAM_On__: boolean;
window.__drv.hold(__PARAM_Key__, __PARAM_On__)
