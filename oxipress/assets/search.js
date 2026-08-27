(function () {
  var input = document.getElementById("search-input");
  var results = document.getElementById("search-results");
  if (!input) return;
  var timer;
  var index = null;
  var loading = null;

  function load() {
    if (index) return Promise.resolve(index);
    if (!loading) {
      loading = fetch("/search-index.json")
        .then(function (r) { return r.json(); })
        .then(function (data) { index = data; return index; });
    }
    return loading;
  }

  function score(entry, terms) {
    var title = entry.title.toLowerCase();
    var desc = (entry.description || "").toLowerCase();
    var tags = (entry.tags || []).map(function (t) { return t.toLowerCase(); });
    var s = 0;
    for (var i = 0; i < terms.length; i++) {
      var term = terms[i];
      if (title.indexOf(term) !== -1) s += 8;
      if (tags.indexOf(term) !== -1) s += 5;
      if (desc.indexOf(term) !== -1) s += 3;
    }
    return s;
  }

  function render(q) {
    var terms = q.toLowerCase().split(/\s+/).filter(Boolean);
    var scored = [];
    for (var i = 0; i < index.length; i++) {
      var s = score(index[i], terms);
      if (s > 0) scored.push([s, index[i]]);
    }
    scored.sort(function (a, b) { return b[0] - a[0]; });

    results.innerHTML = "";
    if (!scored.length) {
      results.innerHTML = "<li>No results</li>";
    } else {
      scored.slice(0, 10).forEach(function (pair) {
        var p = pair[1];
        var li = document.createElement("li");
        var a = document.createElement("a");
        a.href = "/posts/" + p.slug + "/";
        a.textContent = p.title;
        var desc = document.createElement("span");
        desc.className = "desc";
        desc.textContent = p.description || "";
        li.appendChild(a);
        li.appendChild(desc);
        results.appendChild(li);
      });
    }
    results.hidden = false;
  }

  input.addEventListener("input", function () {
    clearTimeout(timer);
    var q = input.value.trim();
    if (!q) {
      results.hidden = true;
      results.innerHTML = "";
      return;
    }
    timer = setTimeout(function () {
      load().then(function () { render(q); }).catch(function () { results.hidden = true; });
    }, 200);
  });

  input.addEventListener("focus", load);

  document.addEventListener("click", function (e) {
    if (!results.contains(e.target) && e.target !== input) results.hidden = true;
  });
})();
