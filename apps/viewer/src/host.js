// window.logixHost: what Logix Viewer offers the pages it shows (the page
// contract, logix-engine docs/requirements.md, Logix Viewer). Every call
// goes to the app as one JSON message; nothing comes back.
(() => {
  const post = (msg) => window.ipc.postMessage(JSON.stringify(msg));
  window.logixHost = {
    version: 1,
    // No openSource: Logix Viewer has no editor, so pages show a place in
    // their hover boxes and offer no double-click to it (the user's
    // decision, 2026-11-23: the note it showed repeated the hover box).
    features: ["saveAs", "writeDeclared", "openUrl"],
    // The system's Save dialog; data is text, or base64 for a PNG.
    saveAs: (name, mime, data) => post({ cmd: "saveAs", name, mime, data }),
    // A file the engine declared for this page, by id, such as "view".
    writeDeclared: (id, text) => post({ cmd: "writeDeclared", id, text }),
    // Pages from before features named it may still call it: nothing.
    openSource: () => {},
    openUrl: (url) => post({ cmd: "openUrl", url }),
  };
})();
