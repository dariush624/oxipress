(function () {
  var root = document.documentElement;
  var saved = localStorage.getItem("theme");
  if (saved) root.setAttribute("data-theme", saved);
  document.getElementById("theme-toggle").addEventListener("click", function () {
    var dark = root.getAttribute("data-theme") === "dark" ||
      (!root.getAttribute("data-theme") && matchMedia("(prefers-color-scheme: dark)").matches);
    var next = dark ? "light" : "dark";
    root.setAttribute("data-theme", next);
    localStorage.setItem("theme", next);
  });
})();
