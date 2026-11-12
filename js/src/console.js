// Installed before Ibex bindings capture their exception reporter.
// @ref LLP 1027 D10 — console writes to the runner
(function (global) {
  'use strict';
  var writeLog = global.__exact_log;
  delete global.__exact_log;
  if (writeLog) {
    var NativeError = global.Error, stringify = JSON.stringify, string = String;
    // JavaScript catches recover formatting errors while Hermes's uncatchable
    // execution interrupts propagate. A JSI catch would swallow both.
    function render(value) {
      try {
        if (value !== null && (typeof value === 'object' || typeof value === 'function')) {
          if (value instanceof NativeError) {
            var text = string(value);
            try {
              var stack = value.stack;
              if (typeof stack === 'string') {
                if (stack.indexOf(text) === 0) return stack;
                if (stack) text += '\n' + stack;
              }
            } catch (_) { /* Keep the name and message if the stack getter throws. */ }
            return text;
          }
          var json = stringify(value);
          if (typeof json === 'string') return json;
        }
      } catch (_) { /* Cycles, BigInts and getters can refuse serialization. */ }
      try { return string(value); }
      catch (_) { return '[unprintable]'; }
    }
    function log() {
      var line = '';
      for (var i = 0; i < arguments.length; i++) {
        if (i) line += ' ';
        line += render(arguments[i]);
      }
      writeLog(line);
    }
    global.console = { log: log, info: log, warn: log, error: log, debug: log };
  }
})(globalThis);
