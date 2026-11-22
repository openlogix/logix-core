// window.logixHost: what Logix Viewer offers the pages it shows (the page
// contract, logix-engine docs/requirements.md, Logix Viewer). Every call
// goes to the app as one JSON message; nothing comes back.
(() => {
  const post = (msg) => window.ipc.postMessage(JSON.stringify(msg));
  window.logixHost = {
    version: 1,
    features: ["saveAs", "writeDeclared", "openSource", "openUrl"],
    // The system's Save dialog; data is text, or base64 for a PNG.
    saveAs: (name, mime, data) => post({ cmd: "saveAs", name, mime, data }),
    // A file the engine declared for this page, by id, such as "view".
    writeDeclared: (id, text) => post({ cmd: "writeDeclared", id, text }),
    openSource: (file, line) => post({ cmd: "openSource", file, line }),
    openUrl: (url) => post({ cmd: "openUrl", url }),
  };
  // Today's pages call these names; they map onto logixHost.
  window.logixWaveSaveView = (view) =>
    window.logixHost.writeDeclared("view", JSON.stringify(view, null, 2) + "\n");
  window.logixWaveSaveFile = (name, mime, data) => window.logixHost.saveAs(name, mime, data);
  window.logixWaveOpenSource = (file, line) => window.logixHost.openSource(file, line);
  window.logixDiagramOpenSource = (file, line) => window.logixHost.openSource(file, line);
})();
