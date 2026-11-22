// window.logixHost: what Logix Viewer offers the pages it shows (the page
// contract, logix-engine docs/requirements.md, Logix Viewer). Every call
// goes to the app as one JSON message; nothing comes back.
(() => {
  const post = (msg) => window.ipc.postMessage(JSON.stringify(msg));
  // A small note at the bottom of the window, selectable, gone after a few
  // seconds or on a click elsewhere.
  let note = null;
  let timer = 0;
  const showPlace = (file, line) => {
    if (!note) {
      note = document.createElement("div");
      note.id = "logix-host-note";
      note.setAttribute("role", "status");
      note.style.cssText =
        "position:fixed;left:50%;bottom:16px;transform:translateX(-50%);z-index:2147483647;" +
        "max-width:80vw;padding:6px 12px;border-radius:6px;font:13px system-ui,sans-serif;" +
        "background:#26323a;color:#fff;box-shadow:0 2px 8px rgba(0,0,0,.3);user-select:text;" +
        "-webkit-user-select:text;overflow-wrap:anywhere";
      document.addEventListener("pointerdown", (e) => {
        if (note && note.isConnected && !note.contains(e.target)) note.remove();
      }, true);
    }
    note.textContent = line ? `${file}, line ${line}` : String(file);
    document.body.appendChild(note);
    clearTimeout(timer);
    timer = setTimeout(() => note.remove(), 6000);
  };
  window.logixHost = {
    version: 1,
    features: ["saveAs", "writeDeclared", "openSource", "openUrl"],
    // The system's Save dialog; data is text, or base64 for a PNG.
    saveAs: (name, mime, data) => post({ cmd: "saveAs", name, mime, data }),
    // A file the engine declared for this page, by id, such as "view".
    writeDeclared: (id, text) => post({ cmd: "writeDeclared", id, text }),
    // Shows where, such as "main_tb.v, line 12", and opens nothing: Logix
    // Viewer has no editor, and the system's default app for a .v file is
    // rarely one (the user's decision, 2026-11-22).
    openSource: (file, line) => showPlace(file, line),
    openUrl: (url) => post({ cmd: "openUrl", url }),
  };
  // Today's pages call these names; they map onto logixHost.
  window.logixWaveSaveView = (view) =>
    window.logixHost.writeDeclared("view", JSON.stringify(view, null, 2) + "\n");
  window.logixWaveSaveFile = (name, mime, data) => window.logixHost.saveAs(name, mime, data);
  window.logixWaveOpenSource = (file, line) => window.logixHost.openSource(file, line);
  window.logixDiagramOpenSource = (file, line) => window.logixHost.openSource(file, line);
})();
