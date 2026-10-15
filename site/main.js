// Pick the right download for this computer. Without JavaScript the button
// simply jumps to the list of all downloads.
(function () {
  var BASE = "https://github.com/realwitt/secondhand/releases/latest/download/";
  var FILES = {
    mac: { file: "Secondhand_0.1.0_universal.dmg", label: "Download for Mac", sub: "Free · 37 MB · Apple silicon and Intel" },
    win: { file: "Secondhand_0.1.0_x64-setup.exe", label: "Download for Windows", sub: "Free · 12 MB · Windows 10 and 11" }
  };

  function detect() {
    var ua = navigator.userAgent || "";
    var plat = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || "";
    var touch = navigator.maxTouchPoints > 1;
    if (/iPhone|iPad|iPod|Android/i.test(ua) || (/Mac/i.test(plat) && touch)) return "mobile";
    if (/Win/i.test(plat) || /Windows/i.test(ua)) return "win";
    if (/Mac/i.test(plat) || /Macintosh/i.test(ua)) return "mac";
    return "other";
  }

  var body = document.body;
  var button = document.getElementById("dl-main");
  var label = document.getElementById("dl-label");
  var sub = document.getElementById("dl-sub");
  var note = document.getElementById("dl-note");
  var other = document.getElementById("dl-other");

  function setSteps(os) {
    body.classList.remove("os-mac", "os-win", "os-any");
    body.classList.add(os === "mac" || os === "win" ? "os-" + os : "os-any");
    var sw = document.querySelectorAll("[data-show]");
    for (var i = 0; i < sw.length; i++) sw[i].setAttribute("aria-pressed", sw[i].getAttribute("data-show") === os ? "true" : "false");
  }

  var os = detect();
  if (FILES[os]) {
    var f = FILES[os];
    button.href = BASE + f.file;
    label.textContent = f.label;
    sub.textContent = f.sub;
    other.textContent = os === "mac" ? "On Windows, or need a different file?" : "On a Mac, or need a different file?";
    button.addEventListener("click", function () {
      note.hidden = false;
      note.textContent = "Your download is starting. When it’s done, follow step 2 below.";
    });
  } else if (os === "mobile") {
    label.textContent = "Get Secondhand";
    sub.textContent = "for your Mac or Windows PC";
    note.hidden = false;
    note.textContent = "Secondhand runs on Mac and Windows computers. Open this page there to download it.";
  } else {
    label.textContent = "Get Secondhand";
    sub.textContent = "for Mac or Windows";
  }
  setSteps(os);

  // "Show Mac / Windows steps" switch
  var sws = document.querySelectorAll("[data-show]");
  for (var i = 0; i < sws.length; i++) {
    sws[i].addEventListener("click", function (e) { setSteps(e.currentTarget.getAttribute("data-show")); });
  }
})();
