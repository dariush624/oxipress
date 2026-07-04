(function () {
  var input = document.getElementById("search-input");
  var results = document.getElementById("search-results");
  if (!input) return;
  var timer;

  input.addEventListener("input", function () {
    clearTimeout(timer);
    var q = input.value.trim();
    if (!q) {
      results.hidden = true;
      results.innerHTML = "";
      return;
    }
    timer = setTimeout(function () {
      fetch("/api/search?q=" + encodeURIComponent(q))
        .then(function (r) { return r.json(); })
        .then(function (data) {
          results.innerHTML = "";
          if (!data.results.length) {
            results.innerHTML = "<li>No results</li>";
          } else {
            data.results.forEach(function (p) {
              var li = document.createElement("li");
              var a = document.createElement("a");
              a.href = "/posts/" + p.slug + "/";
              a.textContent = p.title;
              var desc = document.createElement("span");
              desc.className = "desc";
              desc.textContent = p.description;
              li.appendChild(a);
              li.appendChild(desc);
              results.appendChild(li);
            });
          }
          results.hidden = false;
        })
        .catch(function () { results.hidden = true; });
    }, 200);
  });

  document.addEventListener("click", function (e) {
    if (!results.contains(e.target) && e.target !== input) results.hidden = true;
  });
})();
