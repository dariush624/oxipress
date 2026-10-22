(function () {
  var root = document.documentElement;
  var THEMES = ["original", "brutalist", "retro"];
  var NAMES = { original: "Original", brutalist: "Brutalist", retro: "Retro" };
  var switcher = document.getElementById("style-switch");

  function store(key, value) {
    try { localStorage.setItem(key, value); } catch (e) {}
  }
  function label() {
    var t = root.getAttribute("data-theme");
    switcher.textContent = "Theme: " + NAMES[t];
    switcher.setAttribute("aria-label", "Switch visual theme (current: " + NAMES[t] + ")");
  }

  // Style theme: original -> brutalist -> retro. The initial attribute is set
  // by the inline script in <head> to avoid a flash of the wrong theme.
  switcher.addEventListener("click", function () {
    var i = THEMES.indexOf(root.getAttribute("data-theme"));
    var next = THEMES[(i + 1) % THEMES.length];
    root.setAttribute("data-theme", next);
    store("theme", next);
    label();
  });
  label();

  // Light/dark toggle: only shown in the "original" theme (see style.css).
  document.getElementById("theme-toggle").addEventListener("click", function () {
    var dark = root.getAttribute("data-mode") === "dark" ||
      (!root.getAttribute("data-mode") && matchMedia("(prefers-color-scheme: dark)").matches);
    var next = dark ? "light" : "dark";
    root.setAttribute("data-mode", next);
    store("mode", next);
  });
})();
